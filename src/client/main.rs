use std::net::UdpSocket;

extern crate cpal;

use cpal::EventLoop;
use cpal::{StreamData, UnknownTypeOutputBuffer};
use cpal::Sample as CSample;

extern crate bincode;

use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::VecDeque;

extern crate sample;

use sample::{signal, Signal};
use sample::interpolate::{Converter, Floor, Linear};


fn main() {
    let socket = UdpSocket::bind("127.0.0.1:34001").expect("Cannot bind");
    socket.connect("127.0.0.1:3400").expect("Cannot connect");
    
    let event_loop = EventLoop::new();
    let device = cpal::default_output_device().expect("No output devices");
    let mut supported_formats_range = device.supported_output_formats().expect("error formats");
    
    let format = supported_formats_range.next().expect("no formats").with_max_sample_rate();

    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
    event_loop.play_stream(stream_id);

    let mut buf_vec: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    let net_buf = buf_vec.clone();

    thread::spawn(move || {
        while net_buf.lock().unwrap().len() < 30000 {
            
        }
        event_loop.run(move |_stream_id, mut stream_data| {
            match stream_data {
                StreamData::Output { buffer: UnknownTypeOutputBuffer::U16(mut buffer) } => {
                    //                println!("buffer_len:{}", buffer.len());
                    let mut i = 0;
                    for elem in buffer.iter_mut() {
                        *elem = u16::max_value() / 2;
                        i += 1;
                    }
                    println!("i:{}", i);
                },
                StreamData::Output { buffer: UnknownTypeOutputBuffer::I16(mut buffer) } => {
                    //                println!("bufferI16: {}", buffer.len());
                    println!("net_buf len{}", net_buf.lock().unwrap().len());
                    println!("sample rate{:?}, channels{:?}", format.sample_rate, format.channels);
                    for samp in buffer.chunks_mut(format.channels as usize) {
                        let mut val : i16 = 0;
                        for out in samp.iter_mut() {
                            let sample = net_buf.lock().unwrap().pop_front();
                        if sample.is_none() == false {
                            val = sample.unwrap().to_i16();
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

    
    loop {
        let mut buf = [0; 20000];
        
        let (n_bytes, _addr) = socket.recv_from(&mut buf).expect("Didn't receive data");
        let filled_buf = &mut buf[..n_bytes];
        let decoded : Vec<f32> = bincode::deserialize(&filled_buf[..]).unwrap();
        let mut vec_dec = Vec::new();

        /*for chunk in decoded.chunks(2) {
//            let arr = [chunk[0], chunk[1];
            vec_dec.extend_from_slice(&[[chunk[0], chunk[1]]]);
        }*/


        for d in decoded.iter() {
            vec_dec.extend_from_slice(&[[*d]]);
        }
        
        //        println!("decoded buffer {:?}", &decoded);
        let mut iter_signal = signal::from_iter(vec_dec.iter().cloned());

        let interp = Linear::from_source(&mut iter_signal);
        //let frames = iter_signal.from_hz_to_hz(interp, 44_100 as f64, 192_000 as f64);

        //        let frames = iter_signal.set_playback_hz_scale(interp, 0.25);

        let mut frames = Converter::scale_sample_hz(iter_signal, interp, 4 as f64);
        frames.set_hz_to_hz(44_100 as f64, 192_000 as f64);
        
       /* let ring_buffer = ring_buffer::Fixed::from([[0.0]; 100]);
        let sinc = interpolate::Sinc::new(ring_buffer);
        let new_signal = iter_signal.from_hz_to_hz(sinc, 44_100 as f64, 192_000 as f64);
        */
        for frame in frames.until_exhausted() {
            for f in frame.iter() {
                buf_vec.lock().unwrap().push_back(*f);                
            }
        }
        
/*        for s in vec_dec.iter() {
            for i in 0..2 {
                buf_vec.lock().unwrap().push_back(*s);
            }
        }*/
//        buf_vec.lock().unwrap().append(&mut vec_dec);
    }
}
