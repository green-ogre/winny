use std::{
    fmt::Debug,
    sync::mpsc::{Receiver, Sender},
};

use asset::{Asset, AssetLoaderError, AssetLoaderEvent, Assets, Handle, LoadedAsset};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, StreamConfig,
};
use ecs::{
    Commands, Entity, EventReader, Query, Res, ResMut, SparseArrayIndex, WinnyBundle,
    WinnyComponent, WinnyResource, Without,
};
use util::tracing::{error, info};
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
            error!("{:?}", err);
            $err
        })
    };
}

#[derive(Debug, Copy, Clone)]
pub struct AudioStreamHandle(usize);

impl AudioStreamHandle {
    pub fn new(index: usize) -> Self {
        Self(index)
    }
}

impl SparseArrayIndex for AudioStreamHandle {
    fn index(&self) -> usize {
        self.0
    }
}

pub enum StreamCommand {
    Play,
    Pause,
    Stop,
}

#[derive(WinnyResource)]
pub struct GlobalAudio {
    pub volume: f32,
    device: Option<Device>,
    config: Option<StreamConfig>,
}

impl Debug for GlobalAudio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalAudio")
            .field("audio", &self.volume)
            .finish()
    }
}

impl GlobalAudio {
    pub fn new() -> Self {
        Self {
            volume: 1.0,
            device: None,
            config: None,
        }
    }

    pub fn device(&mut self) -> Result<Device, Error> {
        if self.device.is_some() {
            Ok(self.device.as_ref().unwrap().clone())
        } else {
            let host = cpal::default_host();
            let device = map_stream_err!(Error::HostNA, host.default_output_device().ok_or(()))?;
            self.device = Some(device);
            Ok(self.device.as_ref().unwrap().clone())
        }
    }

    pub fn config(&mut self) -> Result<StreamConfig, Error> {
        if self.config.is_some() {
            Ok(self.config.as_ref().unwrap().clone())
        } else {
            if self.device.is_some() {
                let mut supported_output_configs = map_stream_err!(
                    Error::SupportedOutputConfigNA,
                    self.device.as_ref().unwrap().supported_output_configs()
                )?;
                // TODO: find the right one
                let config = supported_output_configs.nth(2).unwrap();
                let config = config.with_max_sample_rate().into();
                self.config = Some(config);
                Ok(self.config.as_ref().unwrap().clone())
            } else {
                Err(Error::HostNA)
            }
        }
    }
}

pub struct AudioSource {
    bytes: Box<[u8]>,
    format: WavFormat,
}

impl Asset for AudioSource {}

impl AudioSource {
    pub fn new(reader: asset::reader::ByteReader<std::fs::File>) -> Result<Self, AssetLoaderError> {
        let (bytes, format) = wav::load_from_bytes(reader).map_err(|e| {
            error!("{:?}", e);
            AssetLoaderError::from(e)
        })?;

        Ok(Self {
            bytes: bytes.into_boxed_slice(),
            format,
        })
    }

    pub fn stream(
        &self,
        device: Device,
        config: StreamConfig,
        _playback_settings: PlaybackSettings,
        commands: Receiver<StreamCommand>,
    ) -> Result<(), Error> {
        let volume = 0.5;
        let mut stream_offset = 0;

        let data = self.bytes.clone();
        let format = self.format.clone();

        // TODO: resampling
        let resample_ratio: f64 = format.samples_per_sec as f64 / config.sample_rate.0 as f64;
        let _playhead = 0.0;

        error!("{}", format.samples_per_sec);
        error!("{} {}", resample_ratio, data.len());

        std::thread::spawn(move || {
            let (eos_tx, eos_rx) = std::sync::mpsc::channel();

            let stream = map_stream_err!(
                Error::BuildStream,
                device.build_output_stream(
                    &config,
                    move |output: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        let bytes_per_sample = format.bits_per_sample as usize / 8;
                        let mut byte_offset = 0;
                        let mut end_of_stream = false;

                        for frame in output.chunks_mut(format.channels as usize) {
                            let stream_index = byte_offset + stream_offset;
                            for sample in frame.iter_mut() {
                                if stream_index >= data.len() {
                                    *sample = 0;
                                    end_of_stream = true;
                                } else {
                                    let mut s = i16::from_le_bytes([
                                        data[stream_index],
                                        data[stream_index + 1],
                                    ]);
                                    s = (volume * s as f32) as i16;
                                    *sample = s;
                                }

                                byte_offset += bytes_per_sample;
                            }
                        }

                        let samples_read = output.len();
                        stream_offset += samples_read * bytes_per_sample;

                        if end_of_stream {
                            let _ = eos_tx.send(());
                        }
                    },
                    move |err| error!("Error in audio stream: {}", err),
                    None,
                )
            )
            .unwrap();

            stream.play().map_err(|_| Error::PlayStream).unwrap();

            while let Ok(command) = commands.recv() {
                match command {
                    StreamCommand::Play => {}
                    StreamCommand::Pause => {}
                    StreamCommand::Stop => {
                        break;
                    }
                }

                if let Ok(_) = eos_rx.try_recv() {
                    break;
                }
            }

            info!("Exiting audio stream");
        });

        Ok(())
    }
}

impl Debug for AudioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSource")
            .field("bytes: Vec<u8>", &self.bytes.len())
            .field("format", &self.format)
            .finish()
    }
}

#[derive(WinnyComponent, Clone, Copy)]
pub struct PlaybackSettings {
    pub volume: f32,
    pub speed: f32,
    pub loop_sample: bool,
    pub play_on_creation: bool,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            speed: 1.0,
            loop_sample: false,
            play_on_creation: true,
        }
    }
}

#[derive(Debug, WinnyComponent, Clone)]
pub struct AudioPlayback {
    commands: Sender<StreamCommand>,
}

impl AudioPlayback {
    pub fn new(
        source: &LoadedAsset<AudioSource>,
        playback_settings: PlaybackSettings,
        global_audio: &mut GlobalAudio,
    ) -> Result<Self, Error> {
        let (commands_tx, commands_rx) = std::sync::mpsc::channel();

        let device = global_audio.device()?;
        let config = global_audio.config()?;

        source.stream(device, config, playback_settings, commands_rx)?;

        Ok(Self {
            commands: commands_tx,
        })
    }

    pub fn play(&self) {
        let _ = self
            .commands
            .send(StreamCommand::Play)
            .map_err(|err| error!("Could not play audio playback: {:?}", err));
    }

    pub fn pause(&self) {
        let _ = self
            .commands
            .send(StreamCommand::Pause)
            .map_err(|err| error!("Could not pause audio playback: {:?}", err));
    }

    pub fn stop(&self) {
        let _ = self
            .commands
            .send(StreamCommand::Stop)
            .map_err(|err| error!("Could not stop audio playback: {:?}", err));
    }
}

#[derive(WinnyBundle, Clone)]
pub struct AudioBundle {
    pub handle: Handle<AudioSource>,
    pub playback_settings: PlaybackSettings,
}

fn init_audio_bundle_streams(
    mut commands: Commands,
    bundles: Query<(Entity, Handle<AudioSource>, PlaybackSettings), Without<AudioPlayback>>,
    sources: Res<Assets<AudioSource>>,
    mut global_audio: ResMut<GlobalAudio>,
) {
    for (entity, handle, playback_settings) in bundles.iter() {
        if let Some(source) = sources.get(handle) {
            if let Ok(playback) = AudioPlayback::new(source, *playback_settings, &mut global_audio)
            {
                info!("spawning audio playback");
                commands.get_entity(entity).insert(playback);
            } else {
                error!("Could not create playback for audio bundle");
            }
        }
    }
}

fn look_for_event(event: EventReader<AssetLoaderEvent<AudioSource>>) {
    for e in event.read() {
        println!("{:?}", e);
    }
}

struct AudioAssetLoader;

use app::app::App;
use app::plugins::Plugin;
use asset::{AssetApp, AssetLoader};

impl AssetLoader for AudioAssetLoader {
    type Asset = AudioSource;

    fn load(
        reader: asset::reader::ByteReader<std::fs::File>,
        _path: String,
        ext: &str,
    ) -> Result<Self::Asset, AssetLoaderError> {
        match ext {
            "wav" => AudioSource::new(reader),
            _ => Err(AssetLoaderError::UnsupportedFileExtension),
        }
    }

    fn extensions(&self) -> Vec<&'static str> {
        vec!["wav"]
    }
}

impl From<crate::wav::Error> for AssetLoaderError {
    fn from(value: crate::wav::Error) -> Self {
        if value == crate::wav::Error::InvalidPath {
            Self::FileNotFound
        } else {
            Self::FailedToParse
        }
    }
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&mut self, app: &mut App) {
        let loader = AudioAssetLoader {};
        app.register_asset_loader::<AudioSource>(loader)
            .add_systems(ecs::Schedule::PreUpdate, init_audio_bundle_streams)
            .add_systems(ecs::Schedule::Update, look_for_event)
            .insert_resource(GlobalAudio::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecs::prelude::*;
    use tracing_test::traced_test;
    use util::tracing;

    use app::{
        app::AppExit,
        plugins::{Plugin, PluginSet},
        prelude::{KeyCode, KeyInput},
        window::WindowPlugin,
    };
    use asset::*;
    use ecs::{EventReader, EventWriter};

    pub fn startup(mut commands: Commands, mut asset_server: ResMut<AssetServer>) {
        commands.spawn(AudioBundle {
            playback_settings: PlaybackSettings::default(),
            handle: asset_server.load("the_tavern.wav"),
        });
    }

    fn exit_on_load(
        mut event_writer: EventWriter<AppExit>,
        load: EventReader<AssetLoaderEvent<AudioSource>>,
    ) {
        for _ in load.peak_read() {
            event_writer.send(AppExit);
        }
    }

    #[traced_test]
    #[test]
    fn it_works() {
        App::default()
            .add_plugins((
                app::window::WindowPlugin::default(),
                asset::AssetLoaderPlugin {
                    asset_folder: "../../../res/".into(),
                },
                AudioPlugin,
            ))
            .add_systems(Schedule::StartUp, startup)
            .add_systems(Schedule::Update, exit_on_load)
            .run();
    }
}
