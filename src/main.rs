use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() {
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

    let stream = device
        .build_input_stream(
            &supported_config.into(),
            move |data: &[i16], info: &cpal::InputCallbackInfo| {
                println!("{:?}", info.timestamp().capture)
            },
            move |err| {},
            None,
        )
        .expect("cannot build the stream");

    loop {
        stream.play().unwrap();
    }
}
