use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, InputCallbackInfo, OutputCallbackInfo, StreamConfig};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};
use std::net::{SocketAddr, UdpSocket};
use std::thread;

fn from_mic(sock: UdpSocket, mic: Device, config: StreamConfig) {
    let (mut write, mut read) = HeapRb::<u8>::new(20000).split();
    let input_stream_fn = move |buf: &[u8], _: &InputCallbackInfo| {
        for &element in buf {
            let _ = write.try_push(element);
        }
    };
    let input_stream = mic
        .build_input_stream(&config, input_stream_fn, err_fn, None)
        .unwrap();
    let _ = input_stream.play();

    loop {
        let mut buf: [u8; 1024] = [0; 1024];
        let bytes: usize = read.pop_slice(&mut buf);
        sock.send_to(&buf[..bytes], sock.local_addr().unwrap())
            .unwrap();
    }
}

fn to_speaker(sock: UdpSocket, speaker: Device, config: StreamConfig) {
    let (mut write, mut read) = HeapRb::<u8>::new(20000).split();
    let output_stream_fn = move |buf: &mut [u8], _: &OutputCallbackInfo| {
        for element in buf {
            *element = match read.try_pop() {
                Some(x) => x,
                None => 0,
            };
        }
    };
    let output_stream = speaker
        .build_output_stream(&config, output_stream_fn, err_fn, None)
        .unwrap();
    let _ = output_stream.play();

    loop {
        let mut buf: [u8; 10000] = [0; 10000];
        let bytes: usize = sock.recv(&mut buf).unwrap();
        for &element in &buf[0..bytes] {
            match write.try_push(element) {
                Ok(_) => (),
                Err(e) => println!("{e}"),
            };
        }
    }
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}

fn main() {
    let host: Host = cpal::host_from_id(
        cpal::available_hosts()
            .into_iter()
            .next()
            .expect("Could not get host"),
    )
    .unwrap();

    let speaker: Device = host
        .default_output_device()
        .expect("no output device available");
    let mic: Device = host
        .default_input_device()
        .expect("no input device available");

    let config: cpal::StreamConfig = mic.default_input_config().unwrap().config();
    let config2 = config.clone();
    let sock: UdpSocket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let sock2: UdpSocket = sock.try_clone().unwrap();

    thread::spawn(move || from_mic(sock, mic, config));
    thread::spawn(move || to_speaker(sock2, speaker, config2));

    loop {
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}
