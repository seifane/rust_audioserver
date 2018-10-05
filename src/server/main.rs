use std::net::UdpSocket;

use std::thread;

extern crate bincode;

use std::f64::consts::PI;

extern crate simplemad;

use simplemad::{Decoder, Frame};
use std::fs::File;
use std::path::Path;

use std::sync::Arc;
use std::sync::Mutex;
use std::collections::VecDeque;

extern crate ratelimit_meter;

use ratelimit_meter::{LeakyBucket, Decider};

fn main() {
    let socket = UdpSocket::bind("127.0.0.1:3400").expect("couldn't bind to address");

    let mut buf_vec: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    let buf_vec_clone = buf_vec.clone();
    
    thread::spawn(move || {

        let path = Path::new("test.mp3");
        let file = File::open(&path).unwrap();
        let decoder = Decoder::decode(file).unwrap();
    
        for res in decoder {
            match res {
                Err(e) => println!("Error: {:?}", e),
                Ok(frame) => {
                    let mut buffer_left = Vec::new();
                    for sample in frame.samples[0].iter() {
                        buffer_left.push(sample.to_f32());
                    }
                    let mut buffer_right = Vec::new();
                    for sample in frame.samples[1].iter() {
                        buffer_right.push(sample.to_f32());
                    }
                    
                    let mut final_buffer = Vec::new();
                    if buffer_right.len() == buffer_left.len() {
                        for i in 0..buffer_right.len() {
                            final_buffer.push(buffer_left[i]);
                            final_buffer.push(buffer_right[i]);
                        }
                    } else {
                        for i in 0..buffer_left.len() {
                            final_buffer.push(buffer_left[i]);
                            final_buffer.push(buffer_left[i]);
                        }
                    }
                    println!("Frame sample rate: {} left sample size:{}",
                             frame.sample_rate, buffer_left.len());

                    buf_vec_clone.lock().unwrap().append(&mut buffer_left.into_iter().collect());
                }
            }
        }
    });

    let mut lb: LeakyBucket = LeakyBucket::per_second(20).unwrap(); //2205 samples at a time;
    
    loop {
        if buf_vec.lock().unwrap().len() >= 2205 && lb.check() == Ok(()) {
            let mut to_send : Vec<f32> = Vec::new();

            for i in 0..2205 {
                to_send.push(buf_vec.lock().unwrap().pop_front().unwrap());
            }
            let ser = bincode::serialize(&to_send).expect("Cannot serialize");
            
            socket.send_to(&ser, "127.0.0.1:34001");
            println!("Sent frame with {} bytes", ser.len());
        } else {
            std::thread::sleep_ms(10);
        }

    }
}


/*extern crate portaudio;

use portaudio as pa;
use std::f64::consts::PI;

const CHANNELS: i32 = 2;
const NUM_SECONDS: i32 = 1;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 1024;
const TABLE_SIZE: usize = 200;

extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;

use std::net::SocketAddr;
use tokio::prelude::*;
use tokio::net::{UdpSocket, UdpFramed};
use tokio_codec::BytesCodec;

fn main() {
    println!("Hello, world!");

    let addr: SocketAddr = "127.0.0.1:3400".parse().unwrap();
    let socket = UdpSocket::bind(&addr).unwrap();
    let (socketSink, socketStream) = UdpFramed::new(socket, BytesCodec::new()).split();

    tokio::run(socket);
    
    match run(&socketSink) {
        Ok(_) => {
            println!("Everything fine");
        },
        e => {
            eprintln!("Error");
        }
    }
}

fn run(socketSink: &tokio::prelude::Sink) -> Result<(), pa::Error> {

    let mut sine = [0.0; TABLE_SIZE];
    for i in 0..TABLE_SIZE {
        sine[i] = (i as f64 / TABLE_SIZE as f64 * PI * 2.0).sin() as f32;
    }

    let mut left_phase = 0;
    let mut right_phase = 0;

    let mut totalFrames = 0;

    println!("sine:{}", sine);
    
    let pa = try!(pa::PortAudio::new());
    let mut settings = try!(pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER));
    settings.flags = pa::stream_flags::CLIP_OFF;

    let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        let mut idx = 0;

        totalFrames += frames;
        
        println!("length {}", totalFrames);
        
        for _ in 0..frames {
            buffer[idx]   = sine[left_phase];
            buffer[idx+1] = sine[right_phase];
            left_phase += 1;
            if left_phase >= TABLE_SIZE { left_phase -= TABLE_SIZE; }
            right_phase += 4;
            if right_phase >= TABLE_SIZE { right_phase -= TABLE_SIZE; }
            idx += 2;
        }

        socketSink.send_all(buffer);
        pa::Continue
    };

    let mut stream = try!(pa.open_non_blocking_stream(settings, callback));

    try!(stream.start());

    println!("Play for {} seconds.", NUM_SECONDS);
    pa.sleep(NUM_SECONDS * 1_000);

    try!(stream.stop());
    try!(stream.close());

    println!("Test finished.");

    Ok(())
}
*/
