use std::{
    fmt::Debug,
    future::Future,
    io::Cursor,
    ops::Range,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

#[cfg(target_arch = "wasm32")]
use app::input::mouse_and_key::KeyInput;
use app::{
    app::{App, Schedule},
    plugins::Plugin,
};
use asset::{Asset, AssetLoaderError, Assets, Handle, LoadedAsset};
use asset::{AssetApp, AssetLoader};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, SampleFormat, StreamConfig,
};
#[cfg(target_arch = "wasm32")]
use ecs::EventReader;
use ecs::{
    Commands, Entity, Query, Res, ResMut, WinnyBundle, WinnyComponent, WinnyResource, Without,
};
use hound::{WavReader, WavSpec};
use rand::Rng;
use render::RenderContext;
use util::tracing::{error, info, trace};
// use wav::WavFormat;

pub mod prelude;
// pub mod wav;

#[derive(Debug)]
pub enum Error {
    PlayStream,
    PauseStream,
    HostNA,
    SupportedOutputConfigNA,
    OutputConfigNotSupported,
    BuildStream,
    WavReader,
}

macro_rules! map_stream_err {
    ($err:expr, $f:expr) => {
        $f.map_err(|err| {
            error!("{:?}", err);
            $err
        })
    };
}

#[derive(WinnyResource)]
pub struct GlobalAudio {
    pub volume: f32,
    #[cfg(target_arch = "wasm32")]
    pub wasm_initialized: bool,
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
            #[cfg(target_arch = "wasm32")]
            wasm_initialized: false,
        }
    }
}

fn device() -> Result<Device, Error> {
    let host = cpal::default_host();
    let device = map_stream_err!(Error::HostNA, host.default_output_device().ok_or(()))?;
    Ok(device)
}

fn config(device: &Device) -> Result<StreamConfig, Error> {
    let supported_output_configs = map_stream_err!(
        Error::SupportedOutputConfigNA,
        device.supported_output_configs()
    )?;
    let config = map_stream_err!(
        Error::SupportedOutputConfigNA,
        supported_output_configs
            .into_iter()
            .find(|config| {
                // info!("{:?}", config);
                config.sample_format() == SampleFormat::F32 && config.channels() == 2
            })
            .ok_or(())
    )?;
    let config = config.with_sample_rate(cpal::SampleRate(44100)).into();
    trace!("Retrieving audio context: {:?}", config);
    Ok(config)
}

pub struct AudioSource {
    bytes: Arc<[u8]>,
    spec: WavSpec,
}

impl Asset for AudioSource {}

impl AudioSource {
    pub fn new(
        mut reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
    ) -> Result<Self, AssetLoaderError> {
        let bytes = reader
            .read_all()
            .map_err(|_| AssetLoaderError::FailedToParse)?;
        let reader = hound::WavReader::new(Cursor::new(&bytes))
            .map_err(|_| AssetLoaderError::FailedToBuild)?;
        let spec = reader.spec();
        let bytes = bytes.into();

        // let (bytes, format) = wav::load_from_bytes(reader).map_err(|e| {
        //     error!("{:?}", e);
        //     AssetLoaderError::from(e)
        // })?;

        Ok(Self { bytes, spec })
    }

    pub fn stream(
        &self,
        device: Device,
        config: StreamConfig,
        global_audio: &GlobalAudio,
        playback_settings: PlaybackSettings,
    ) -> Result<StreamHandle, Error> {
        let volume = global_audio.volume * playback_settings.volume;
        // info!("volume for stream: {volume}");
        let bytes = self.bytes.clone();
        let mut reader = WavReader::new(Cursor::new(bytes)).map_err(|_| Error::WavReader)?;
        let spec = self.spec.clone();

        let resample_ratio = spec.sample_rate as f32 / config.sample_rate.0 as f32;
        trace!("resampling stream: {}", resample_ratio);
        let (eos_tx, eos_rx) = channel();
        // let mut sample_cursor = 0.0;
        // let mut stream_offset = 0;

        let stream = map_stream_err!(
            Error::BuildStream,
            device.build_output_stream(
                &config,
                move |output: &mut [f32], _: &cpal::OutputCallbackInfo| resampling_stream_f32(
                    output,
                    &mut reader,
                    // resample_ratio,
                    // &mut stream_offset,
                    volume,
                    // &mut sample_cursor,
                    // &spec,
                    &eos_tx,
                    &playback_settings,
                ),
                move |err| error!("Error in audio stream: {}", err),
                None,
            )
        )
        .unwrap();
        stream.play().map_err(|_| Error::PlayStream).unwrap();
        Ok(StreamHandle(stream, eos_rx))
    }
}

// #[allow(dead_code)]
// fn resampling_stream_i16(
//     output: &mut [i16],
//     data: &[u8],
//     resample_ratio: f32,
//     stream_offset: &mut usize,
//     volume: f32,
//     sample_cursor: &mut f32,
//     format: &,
//     eos_tx: &Sender<()>,
//     playback_settings: &PlaybackSettings,
// ) {
//     let bytes_per_sample = format.bits_per_sample as usize / 8;
//     let mut byte_offset = 0;
//     let mut end_of_stream = false;
//     let mut samples_read = 0;
//
//     for frame in output.chunks_mut(format.channels as usize) {
//         let stream_index = byte_offset + *stream_offset;
//         for sample in frame.iter_mut() {
//             if stream_index >= data.len() {
//                 *sample = 0;
//                 end_of_stream = true;
//             } else {
//                 let s1 = i16::from_le_bytes([data[stream_index], data[stream_index + 1]]);
//                 let s2 = i16::from_le_bytes([data[stream_index + 2], data[stream_index + 3]]);
//
//                 let mut s = lerp(s1 as f32, s2 as f32, 1.0 - *sample_cursor) as i16;
//                 s = (volume * s as f32) as i16;
//                 *sample = s;
//             }
//
//             *sample_cursor += resample_ratio as f32;
//             if *sample_cursor >= 1.0 {
//                 byte_offset += bytes_per_sample;
//                 *sample_cursor = 0.0;
//                 samples_read += 1;
//             }
//         }
//     }
//
//     *stream_offset += samples_read * bytes_per_sample;
//
//     if end_of_stream {
//         if playback_settings.loop_track {
//             *stream_offset = 0;
//         } else {
//             if eos_tx.send(()).is_err() {
//                 error!("audio stream reciever closed");
//             }
//         }
//     }
// }

#[allow(dead_code)]
fn resampling_stream_f32(
    output: &mut [f32],
    reader: &mut WavReader<Cursor<Arc<[u8]>>>,
    // resample_ratio: f32,
    // stream_offset: &mut usize,
    volume: f32,
    // sample_cursor: &mut f32,
    // spec: &WavSpec,
    eos_tx: &Sender<()>,
    playback_settings: &PlaybackSettings,
) {
    // let bytes_per_sample = spec.bits_per_sample as usize / 8;
    // let mut byte_offset = 0;
    // let mut end_of_stream = false;
    // let mut samples_read = 0;

    for (buf, sample) in output.iter_mut().zip(reader.samples::<i32>()) {
        // println!("{spec:#?}, {sample:?}");
        let sample = sample.unwrap_or_default() as f32 / i32::MAX as f32;
        *buf = sample * playback_settings.volume * volume;
    }

    let remaining_samples = reader.samples::<f32>().len();

    // for frame in output.chunks_mut(format.channels as usize) {
    //
    //     // let stream_index = byte_offset + *stream_offset;
    //     // for sample in frame.iter_mut() {
    //     //     if stream_index >= data.len() {
    //     //         *sample = 0.0;
    //     //         end_of_stream = true;
    //     //     } else {
    //     //         let s1 = i16::from_le_bytes([data[stream_index], data[stream_index + 1]]);
    //     //         let s2 = i16::from_le_bytes([data[stream_index + 2], data[stream_index + 3]]);
    //     //         let s = lerp(s1 as f32, s2 as f32, 1.0 - *sample_cursor);
    //     //         *sample = s / i16::MAX as f32 * volume;
    //     //     }
    //     //
    //     //     *sample_cursor += resample_ratio as f32;
    //     //     if *sample_cursor >= 1.0 {
    //     //         byte_offset += bytes_per_sample;
    //     //         *sample_cursor = 0.0;
    //     //         samples_read += 1;
    //     //     }
    //     // }
    // }

    // *stream_offset += samples_read * bytes_per_sample;

    if remaining_samples == 0 {
        // if playback_settings.loop_track {
        //     *stream_offset = 0;
        // } else {
        if eos_tx.send(()).is_err() {
            error!("audio stream reciever closed");
        }
        // }
    }
}

// fn lerp(v1: f32, v2: f32, l: f32) -> f32 {
//     v1 * (1.0 - l) + v2 * l
// }

impl Debug for AudioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSource")
            .field("bytes: Vec<u8>", &self.bytes.len())
            .field("format", &self.spec)
            .finish()
    }
}

#[derive(WinnyComponent, Clone, Copy)]
pub struct PlaybackSettings {
    pub volume: f32,
    pub speed: f32,
    pub loop_track: bool,
    pub play_on_creation: bool,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            speed: 1.0,
            loop_track: false,
            play_on_creation: true,
        }
    }
}

impl PlaybackSettings {
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    pub fn with_speed_variation(mut self, speed: Range<f32>) -> Self {
        self.speed = rand::thread_rng().gen_range(speed);
        self
    }

    pub fn loop_track(mut self) -> Self {
        self.loop_track = true;
        self
    }

    pub fn play_on_creation(mut self, play: bool) -> Self {
        self.play_on_creation = play;
        self
    }
}

pub struct StreamHandle(cpal::Stream, Receiver<()>);

unsafe impl Sync for StreamHandle {}
unsafe impl Send for StreamHandle {}

pub enum StreamCommand {
    Play,
    Pause,
    Stop,
}

#[derive(WinnyComponent)]
pub struct AudioPlayback {
    handle: StreamHandle,
    // commands: Sender<StreamCommand>,
    path: String,
}

impl Drop for AudioPlayback {
    fn drop(&mut self) {
        info!("exiting audio stream: {:?}", self.path);
    }
}

impl AudioPlayback {
    pub fn new(
        source: &LoadedAsset<AudioSource>,
        playback_settings: PlaybackSettings,
        global_audio: &mut GlobalAudio,
    ) -> Result<Self, Error> {
        // let (commands_tx, commands_rx) = std::sync::mpsc::channel();

        let device = device()?;
        let config = config(&device)?;

        info!("spawning audio playback: {:?}", source.path);
        let handle = source.stream(device, config, global_audio, playback_settings)?;

        Ok(Self {
            handle,
            // commands: commands_tx,
            path: source.path.clone(),
        })
    }

    pub fn play(&self) {
        info!("playing stream: {}", self.path);
        if let Err(e) = self.handle.0.play() {
            error!("{e}");
        }
    }

    pub fn pause(&self) {
        info!("pausing stream: {}", self.path);
        if let Err(e) = self.handle.0.pause() {
            error!("{e}");
        }
    }

    pub fn stop(self) {}
}

#[derive(WinnyBundle, Clone)]
pub struct AudioBundle {
    pub handle: Handle<AudioSource>,
    pub playback_settings: PlaybackSettings,
}

#[cfg(not(target_arch = "wasm32"))]
fn init_audio_bundle_streams(
    mut commands: Commands,
    bundles: Query<(Entity, Handle<AudioSource>, PlaybackSettings), Without<AudioPlayback>>,
    sources: Res<Assets<AudioSource>>,
    mut global_audio: ResMut<GlobalAudio>,
) {
    for (entity, handle, playback_settings) in bundles.iter() {
        if let Some(source) = sources.get(handle) {
            match AudioPlayback::new(source, *playback_settings, &mut global_audio) {
                Ok(playback) => {
                    commands.get_entity(entity).insert(playback);
                }
                Err(e) => {
                    error!("could not create playback for audio bundle: {e:?}")
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn init_wasm_audio(mut global_audio: ResMut<GlobalAudio>, user_gestures: EventReader<KeyInput>) {
    if let Some(_) = user_gestures.peak() {
        global_audio.wasm_initialized = true;
    }
}

#[cfg(target_arch = "wasm32")]
fn init_audio_bundle_streams(
    mut commands: Commands,
    bundles: Query<(Entity, Handle<AudioSource>, PlaybackSettings), Without<AudioPlayback>>,
    sources: Res<Assets<AudioSource>>,
    mut global_audio: ResMut<GlobalAudio>,
) {
    if global_audio.wasm_initialized == false {
        return;
    }
    for (entity, handle, playback_settings) in bundles.iter() {
        if let Some(source) = sources.get(handle) {
            match AudioPlayback::new(source, *playback_settings, &mut global_audio) {
                Ok(playback) => {
                    commands.get_entity(entity).insert(playback);
                }
                Err(e) => {
                    error!("could not create playback for audio bundle: {e:?}")
                }
            }
        }
    }
}

fn flush_finished_streams(mut commands: Commands, streams: Query<(Entity, AudioPlayback)>) {
    for (e, playback) in streams.iter() {
        if playback.handle.1.try_recv().is_ok() {
            commands.get_entity(e).despawn();
        }
    }
}

struct AudioAssetLoader;

impl AssetLoader for AudioAssetLoader {
    type Asset = AudioSource;

    fn load(
        _context: RenderContext,
        reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _path: String,
        ext: &str,
    ) -> impl Future<Output = Result<Self::Asset, AssetLoaderError>> {
        async move {
            match ext {
                "wav" => AudioSource::new(reader),
                _ => Err(AssetLoaderError::UnsupportedFileExtension),
            }
        }
    }

    fn extensions(&self) -> Vec<&'static str> {
        vec!["wav"]
    }
}

// impl From<crate::wav::Error> for AssetLoaderError {
//     fn from(value: crate::wav::Error) -> Self {
//         if value == crate::wav::Error::InvalidPath {
//             Self::FileNotFound
//         } else {
//             Self::FailedToParse
//         }
//     }
// }

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&mut self, app: &mut App) {
        let loader = AudioAssetLoader {};
        #[cfg(not(target_arch = "wasm32"))]
        app.register_asset_loader::<AudioSource>(loader)
            .add_systems(
                Schedule::PreUpdate,
                (init_audio_bundle_streams, flush_finished_streams),
            )
            .insert_resource(GlobalAudio::new());
        #[cfg(target_arch = "wasm32")]
        app.register_asset_loader::<AudioSource>(loader)
            .add_systems(
                ecs::Schedule::PreUpdate,
                (
                    init_wasm_audio,
                    init_audio_bundle_streams,
                    flush_finished_streams,
                ),
            )
            .insert_resource(GlobalAudio::new());
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use ecs::prelude::*;
    // use tracing_test::traced_test;
    // use util::tracing;
    //
    // use app::{
    //     app::{App, AppExit},
    //     plugins::{Plugin, PluginSet},
    //     prelude::{KeyCode, KeyInput},
    //     window::WindowPlugin,
    // };
    // use asset::*;
    // use ecs::{EventReader, EventWriter};
    //
    // pub fn startup(mut commands: Commands, mut asset_server: ResMut<AssetServer>) {
    //     commands.spawn(AudioBundle {
    //         playback_settings: PlaybackSettings::default(),
    //         handle: asset_server.load("the_tavern.wav"),
    //     });
    // }
    //
    // fn exit_on_load(
    //     mut event_writer: EventWriter<AppExit>,
    //     load: EventReader<AssetLoaderEvent<AudioSource>>,
    // ) {
    //     for _ in load.peak_read() {
    //         event_writer.send(AppExit);
    //     }
    // }
    //
    // #[traced_test]
    // #[test]
    // fn it_works() {
    //     App::default()
    //         .add_plugins((
    //             app::window::WindowPlugin::default(),
    //             asset::AssetLoaderPlugin {
    //                 asset_folder: "../../../res/".into(),
    //             },
    //             AudioPlugin,
    //         ))
    //         .add_systems(Schedule::StartUp, startup)
    //         .add_systems(Schedule::Update, exit_on_load)
    //         .run();
    // }
}
