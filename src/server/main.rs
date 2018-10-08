use std::net::UdpSocket;

use std::thread;

extern crate bincode;

extern crate simplemad;

use simplemad::Decoder;
use std::fs::File;
use std::path::Path;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::VecDeque;

extern crate ratelimit_meter;

use ratelimit_meter::{LeakyBucket, Decider};

extern crate rustyline;

#[macro_use]
extern crate clap;

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);
    
    for a in &args {
        println!("Streaming to client {}", a);
    }
    
    let buf_vec: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    let clear_mutex: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

//    decode_mp3("test.mp3", buf_vec.clone());
    socket_loop(buf_vec.clone(), clear_mutex.clone(), args.clone());

    let mut rl = rustyline::Editor::<()>::new();
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let boxed_line = Box::new(line);
                let parsed = Box::leak(boxed_line).split_whitespace();
                let mut parsed_vec = Vec::new();

                for p in parsed {
                    parsed_vec.push(p);
                }
                
                if parsed_vec[0] == "load" && parsed_vec.len() == 2 {
                    decode_mp3(parsed_vec[1], buf_vec.clone());
                }
                if parsed_vec[0] == "clear" {
                    clear_mutex.store(true, Ordering::SeqCst);
                    buf_vec.lock().unwrap().clear();
                    clear_mutex.store(false, Ordering::SeqCst);
                }
            },
            Err(_) => std::process::exit(0),
        }
    }
}

fn decode_mp3(filepath: &'static str, buf_vec_clone: Arc<Mutex<VecDeque<f32>>>) -> () {
    thread::spawn(move || {
        println!("Spawning thread to parse mp3 at {}", filepath);
        let path = Path::new(filepath);
        if !path.exists() {
            println!("Path doesn't exist");
            return ();
        }
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
                    if frame.samples.len() > 1 {
                        for sample in frame.samples[1].iter() {
                            buffer_right.push(sample.to_f32());
                        }
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
                   
                    /*println!("Frame sample rate: {} left sample size:{}",
                             frame.sample_rate, buffer_left.len());*/

                    buf_vec_clone.lock().unwrap().append(&mut buffer_left.into_iter().collect());
                }
            }
        }
        println!("Done importing mp3");
    });
}

fn socket_loop(buf_vec: Arc<Mutex<VecDeque<f32>>>,
               clear_mutex: Arc<AtomicBool>,
               clients: Vec<String>) -> () {
    thread::spawn(move || {
        let socket = UdpSocket::bind("0.0.0.0:3400").expect("couldn't bind to address");
        
        let mut lb: LeakyBucket = LeakyBucket::per_second(20).unwrap(); //2205 samples at a time;
        
        loop {
            if lb.check() == Ok(()) {
                let mut to_send : Vec<f32> = Vec::new();
                
                for _i in 0..2205 {
                    if !clear_mutex.load(Ordering::SeqCst) && buf_vec.lock().unwrap().len() > 0 {
                        to_send.push(buf_vec.lock().unwrap().pop_front().unwrap());
                    } else {
                        to_send.push(0 as f32);
                    }
                }
                let ser = bincode::serialize(&to_send).expect("Cannot serialize");

                for client in &clients {
                    socket.send_to(&ser, client).expect("Cannot send to client");
                }
                
                
//                socket.send_to(&ser, "192.168.0.100:34001").expect("Cannot send to client");
         //       println!("Sent frame with {} bytes", ser.len());
            } else {
                std::thread::sleep_ms(10);
            }
        }
    });
}
