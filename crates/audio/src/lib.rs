pub mod prelude;

#[cfg(test)]
mod tests {
    #![allow(unused)]
    use std::{
        f32::consts::PI,
        thread::sleep,
        time::{Duration, SystemTime},
    };

    use cpal::{
        traits::{DeviceTrait, StreamTrait},
        BufferSize, Sample, SampleRate, StreamConfig,
    };

    use super::*;

    #[test]
    fn test() {
        use cpal::traits::HostTrait;
        let host = cpal::default_host();

        let ddevice = host
            .default_output_device()
            .expect("no audio output device available on the system");

        let supported_output_configs = ddevice
            .supported_output_configs()
            .expect("no supported config for audio host");

        let config = supported_output_configs.last().unwrap();
        let sample_format = config.sample_format();
        let sample_rate = 48000.0;
        let config = config.with_sample_rate(SampleRate(48000)).into();

        println!("{config:?} => {sample_format:?}");

        let mut sample_clock = 0f32;

        let volume = 0.1;
        let pitch = 560.0;

        let stream = ddevice
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for sample in data.iter_mut() {
                        sample_clock = (sample_clock + 1.0) % sample_rate;
                        *sample = volume
                            * (sample_clock * pitch * 2.0 * std::f32::consts::PI / sample_rate)
                                .sin();
                    }
                },
                move |err| println!("Error in stream: {}", err),
                None,
            )
            .unwrap();

        stream.play().unwrap();

        sleep(Duration::from_secs(3));
    }
}

