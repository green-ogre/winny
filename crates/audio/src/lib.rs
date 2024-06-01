use std::{fmt::Debug, sync::Arc};

use asset::Asset;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate,
};
use ecs::{SparseArrayIndex, SparseSet};
use logger::error;
use wav::WavFormat;

pub mod prelude;
pub mod wav;

#[derive(Debug)]
pub enum Error {
    PlayStream,
    PauseStream,
    HostNA,
    SupportedOutputConfigNA,
    BuildStream,
}

macro_rules! map_stream_err {
    ($err:expr, $f:expr) => {
        $f.map_err(|err| {
            logger::error!("{:?}", err);
            $err
        })
    };
}

pub struct AudioStreamHandle(usize);

impl AudioStreamHandle {
    pub fn new(index: usize) -> Self {
        Self(index)
    }
}

impl SparseArrayIndex for AudioStreamHandle {
    fn to_index(&self) -> usize {
        self.0
    }
}

pub struct AudioSource {
    byte_stream: Vec<u8>,
    format: WavFormat,
}

impl Asset for AudioSource {}

impl AudioSource {
    pub fn new(byte_stream: Vec<u8>, format: WavFormat) -> Self {
        Self {
            byte_stream,
            format,
        }
    }
}

impl Debug for AudioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSource")
            .field("byte_stream: Vec<u8>", &self.byte_stream.len())
            .field("format", &self.format)
            .finish()
    }
}

struct Stream(pub cpal::Stream);

// NOTE: Stream can safely implement Send and Sync because only one system
// can access AudioStream mutably at any given time
unsafe impl Sync for Stream {}
unsafe impl Send for Stream {}

impl Stream {
    pub fn new(raw_stream: cpal::Stream) -> Self {
        Self(raw_stream)
    }

    pub fn play(&mut self) -> Result<(), Error> {
        map_stream_err!(Error::PlayStream, self.0.play())
    }

    pub fn pause(&mut self) -> Result<(), Error> {
        map_stream_err!(Error::PauseStream, self.0.pause())
    }
}

struct AudioStreamPlayer {
    active_streams: SparseSet<AudioStreamHandle, Stream>,
}

impl AudioStreamPlayer {
    pub fn new(&self, data: Arc<AudioSource>) -> Result<cpal::Stream, Error> {
        let host = cpal::default_host();
        let ddevice = map_stream_err!(Error::HostNA, host.default_output_device().ok_or(()))?;
        let mut supported_output_configs = map_stream_err!(
            Error::SupportedOutputConfigNA,
            ddevice.supported_output_configs()
        )?;

        // TODO: fix
        let config = supported_output_configs.nth(2).unwrap();
        let config = config.with_sample_rate(SampleRate(48000)).into();

        let volume = 0.5;
        let mut stream_offset = 0;

        let stream = map_stream_err!(
            Error::BuildStream,
            ddevice.build_output_stream(
                &config,
                move |output: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let bytes_per_sample = data.format.bits_per_sample as usize / 8;
                    let mut byte_offset = 0;

                    for frame in output.chunks_mut(data.format.channels as usize) {
                        let stream_index = byte_offset + stream_offset;
                        for sample in frame.iter_mut() {
                            if stream_index >= data.byte_stream.len() {
                                *sample = 0;
                            } else {
                                let mut s = i16::from_le_bytes([
                                    data.byte_stream[stream_index],
                                    data.byte_stream[stream_index + 1],
                                ]);
                                s = (volume * s as f32) as i16;
                                *sample = s;
                            }

                            byte_offset += bytes_per_sample;
                        }
                    }

                    let samples_read = output.len();
                    stream_offset += samples_read * bytes_per_sample;
                },
                move |err| error!("Error in stream: {}", err),
                None,
            )
        )?;

        let stream = map_stream_err!(
            Error::BuildStream,
            ddevice.build_output_stream(
                &config,
                move |output: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    output.fill(0);
                },
                move |err| error!("Error in stream: {}", err),
                None,
            )
        )?;

        Ok(stream)
    }
}

struct AudioAssetLoader;

// impl AssetLoader for AudioAssetLoader {
//     type Asset = AudioSource;
//
//     fn load(
//         &self,
//         reader: asset::reader::ByteReader<File>,
//     ) -> Result<LoadedAsset<Self::Asset>, ()> {
//         let (bytes, format) = wav::load_from_bytes(reader).map_err(|err| {
//             error!("{:?}", err);
//             ()
//         })?;
//
//         Ok(LoadedAsset::new(AudioSource::new(bytes, format)))
//     }
//
//     fn extensions(&self) -> Vec<String> {
//         vec!["wav".into()]
//     }
// }

struct AudioPlugin;

// impl Plugin for AudioPlugin {
//     fn build(&mut self, app: &mut App) {
//         let loader = AudioAssetLoader {};
//         app.register_asset_loader::<AudioSource>(loader);
//     }
// }
