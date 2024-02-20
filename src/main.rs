use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error");

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("no input device available");

    let mut supported_configs_range = device
        .supported_input_configs()
        .expect("error while querying configs");

    let supported_config = supported_configs_range
        .next()
        .expect("no suported config?!")
        .with_max_sample_rate();

    let audio_data = Arc::new(Mutex::new(Vec::<i16>::new()));

    let cloned_data = Arc::clone(&audio_data);

    let stream = device
        .build_input_stream(
            &supported_config.into(),
            move |data: &[i16], info: &cpal::InputCallbackInfo| {
                cloned_data.lock().unwrap().extend_from_slice(data);
                println!("{:?}", info.timestamp().capture)
            },
            move |err| {},
            None,
        )
        .expect("cannot build the stream");

    while running.load(Ordering::SeqCst) {
        stream.play().unwrap();
    }

    println!("{:?}", audio_data.lock().unwrap());
}
