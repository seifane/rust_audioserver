use std::net::UdpSocket;

extern crate cpal;

use cpal::EventLoop;
use cpal::{StreamData, UnknownTypeOutputBuffer};
use cpal::Sample as CSample;

extern crate bincode;

use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::VecDeque;

extern crate sample;

use sample::{signal, Signal};
use sample::interpolate::{Converter, Linear};

extern crate indicatif;
use indicatif::ProgressBar;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let buf_vec: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    let is_buffering: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    
    let event_loop = EventLoop::new();
    let device = cpal::default_output_device().expect("No output devices");
    let mut supported_formats_range = device.supported_output_formats().expect("error formats");    
    let format = supported_formats_range.next().expect("no formats").with_max_sample_rate();

    let sample_rate = format.sample_rate.0;
    let channels = format.channels;

    println!("Sample_rate: {} | Channels: {}", sample_rate, channels);
    
    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
    
    sound_loop(event_loop, stream_id, format,
               buf_vec.clone(), is_buffering.clone());
    socket_loop(args[1].to_string(), buf_vec, is_buffering.clone());
}

fn sound_loop(event_loop: EventLoop,
              stream_id: cpal::StreamId,
              format: cpal::Format,
              net_buf: Arc<Mutex<VecDeque<f32>>>,
              is_buffering: Arc<AtomicBool>) -> () {
    event_loop.play_stream(stream_id);

    thread::spawn(move || {
        event_loop.run(move |_stream_id, mut stream_data| {
            match stream_data {
                StreamData::Output { buffer: UnknownTypeOutputBuffer::U16(mut buffer) } => {
                    let mut i = 0;
                    for elem in buffer.iter_mut() {
                        *elem = u16::max_value() / 2;
                        i += 1;
                    }
                    println!("i:{}", i);
                },
                StreamData::Output { buffer: UnknownTypeOutputBuffer::I16(mut buffer) } => {
                    for samp in buffer.chunks_mut(format.channels as usize) {
                        let mut val : i16 = 0;
                        for out in samp.iter_mut() {
                            if !is_buffering.load(Ordering::SeqCst) {
                                let sample = net_buf.lock().unwrap().pop_front();
                                if sample.is_none() == false {
                                    val = sample.unwrap().to_i16();
                                }
                            }
                            *out = val;
                        }
                    }
                },
                StreamData::Output { buffer: UnknownTypeOutputBuffer::F32(mut buffer) } => {
                    let mut i = 0;
                    for elem in buffer.iter_mut() {
                        *elem = 0.0;
                        i += 1;
                    }
                    println!("i2:{}", i);
                },
                _ => (),
            }
        });
    });
}

fn socket_loop(server_addr: String,
               buf_vec: Arc<Mutex<VecDeque<f32>>>,
               is_buffering: Arc<AtomicBool>) -> () {
    let socket = UdpSocket::bind("0.0.0.0:34001").expect("Cannot bind");
    socket.connect(server_addr).expect("Cannot connect");

    let bar = ProgressBar::new(200000);
    
    loop {
        let mut buf = [0; 20000];
        
        let (n_bytes, _addr) = socket.recv_from(&mut buf).expect("Didn't receive data");
        let filled_buf = &mut buf[..n_bytes];
        let decoded : Vec<f32> = bincode::deserialize(&filled_buf[..]).unwrap();
        let mut vec_dec = Vec::new();

        for d in decoded.iter() {
            vec_dec.extend_from_slice(&[[*d]]);
        }

        let mut iter_signal = signal::from_iter(vec_dec.iter().cloned());

        let interp = Linear::from_source(&mut iter_signal);

        let mut frames = Converter::scale_sample_hz(iter_signal, interp, 4 as f64);
        frames.set_hz_to_hz(44_100 as f64, 192_000 as f64);

        for frame in frames.until_exhausted() {
            for f in frame.iter() {
                buf_vec.lock().unwrap().push_back(*f);                
            }
        }
        if buf_vec.lock().unwrap().len() > 40000 {
            let diff = buf_vec.lock().unwrap().len() - 30000;
            let mut main_buffer = buf_vec.lock().unwrap();
            main_buffer.drain(diff..);
            println!("Re-sync!");
        }

        if is_buffering.load(Ordering::SeqCst) && buf_vec.lock().unwrap().len() > 30000 {
            println!("Finished buffering");
            is_buffering.store(false, Ordering::SeqCst);
        }

        if !is_buffering.load(Ordering::SeqCst) && buf_vec.lock().unwrap().len() < 20000 {
            println!("Buffering ...");
            is_buffering.store(true, Ordering::SeqCst);
        }
        
        bar.set_position(buf_vec.lock().unwrap().len() as u64);
    }    
}
