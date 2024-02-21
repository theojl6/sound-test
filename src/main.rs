use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::mem::MaybeUninit;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::FromSample;
use cpal::Sample;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const BITDEPTH: u16 = 16;
const SAMPLERATE: u32 = 44100;
const CHANNELS: u16 = 1;
const BLOCKALIGN: u16 = BITDEPTH / 2;
const BYTERATE: u32 = SAMPLERATE * BITDEPTH as u32 / 8;
const FORMAT: u16 = 1; // WAVE_FORMAT_PCM
const CHUNKSIZE: u32 = 16;

fn main() -> std::io::Result<()> {
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

    println!("device.name(){:?}", device.name());

    let mut supported_configs_range = device
        .supported_input_configs()
        .expect("error while querying configs");

    let config = supported_configs_range
        .next()
        .expect("no suported config?!")
        .with_sample_rate(cpal::SampleRate(SAMPLERATE));

    let audio_data = Arc::new(Mutex::new(Vec::new()));

    let cloned_data = Arc::clone(&audio_data);

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i8, i8>(data, cloned_data),
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i16, i16>(data, cloned_data),
            err_fn,
            None,
        ),
        cpal::SampleFormat::I32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i32, i32>(data, cloned_data),
            err_fn,
            None,
        ),
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<f32, f32>(data, cloned_data),
            err_fn,
            None,
        ),
        sample_format => {
            panic!()
        }
    }
    .expect("Cannot build stream");

    while running.load(Ordering::SeqCst) {
        stream.play().unwrap();
    }

    println!("{:?}", audio_data.lock().unwrap());

    // open file
    let mut output_file = File::create("wav_file_with_rust_sample.wav")?;

    // Header
    // - RIFF
    output_file.write_all(&[0x52, 0x49, 0x46, 0x46])?;

    // - ---- placeholder
    let pos_cksize = output_file.stream_position()?;
    output_file.write_all("----".as_bytes())?;
    output_file.write_all("WAVE".as_bytes())?;

    // Format
    output_file.write_all("fmt ".as_bytes())?;
    output_file.write_all(&CHUNKSIZE.to_le_bytes())?;
    output_file.write_all(&FORMAT.to_le_bytes())?;
    output_file.write_all(&CHANNELS.to_le_bytes())?;
    output_file.write_all(&SAMPLERATE.to_le_bytes())?;
    output_file.write_all(&BYTERATE.to_le_bytes())?;
    output_file.write_all(&BLOCKALIGN.to_le_bytes())?;
    output_file.write_all(&BITDEPTH.to_le_bytes())?;

    // Data
    output_file.write_all("data".as_bytes())?;
    let pos_data_placeholder = output_file.stream_position()?;
    output_file.write_all("----".as_bytes())?;
    let pos_data_start = output_file.stream_position()?;

    for audio_slice in audio_data.lock().unwrap().clone().into_iter() {
        output_file.write_all(&audio_slice.to_le_bytes())?;
    }

    let mut pos_end = output_file.stream_position()?;

    let chunk_size_data: u32 = (pos_end - pos_data_start) as u32;
    if chunk_size_data % 2 != 0 {
        output_file.write_all(&[0x00])?;
        pos_end = output_file.stream_position()?;
    }

    output_file.seek(SeekFrom::Start(pos_data_placeholder))?;

    output_file.write_all(&chunk_size_data.to_le_bytes())?;

    output_file.seek(SeekFrom::Start(pos_cksize))?;

    let chunk_size_header: u32 = (pos_end - 8) as u32;
    output_file.write_all(&chunk_size_header.to_le_bytes())?;

    output_file.sync_all()?;

    let max_amplitude: f64 = 2.0f64.powi((BITDEPTH - 1).into()) - 1.0;
    println!("max_amplitude: {}", max_amplitude);

    Ok(())
}

fn write_input_data<T, U>(input: &[T], buffer: Arc<Mutex<Vec<MaybeUninit<u8>>>>)
where
    T: Sample,
    U: Sample + FromSample<T>,
{
    let converted_input: &[MaybeUninit<u8>] = unsafe {
        std::slice::from_raw_parts(input.as_ptr() as *const MaybeUninit<u8>, input.len())
    };

    buffer.lock().unwrap().extend_from_slice(converted_input);
}
