use bevy::prelude::{AddAsset, App, Component, IntoSystemConfigs, Last, Plugin, Resource};

mod audio_source;
mod global_sound;
mod ogg;
mod spatial_sound;
mod streaming_sound;
mod wav;

#[derive(Component)]
pub struct SoundRadius(pub f32);

impl SoundRadius {
    pub fn new(radius: f32) -> Self {
        Self(radius)
    }
}

#[allow(dead_code)]
#[derive(Component, PartialEq, Copy, Clone)]
pub enum SoundGain {
    Decibel(f32), // -n .. +n
    Ratio(f32),   // 0..1
}

impl Default for SoundGain {
    fn default() -> Self {
        SoundGain::Ratio(1.0)
    }
}

#[derive(Resource)]
pub struct OddioContext {
    pub mixer: oddio::Handle<oddio::Mixer<[f32; 2]>>,
    pub spatial: oddio::Handle<oddio::SpatialScene>,
    pub sample_rate: u32,
}

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use global_sound::global_sound_system;
use ogg::OggLoader;
use spatial_sound::spatial_sound_system;
use streaming_sound::StreamingSound;
use wav::WavLoader;

pub use audio_source::{AudioSource, StreamingAudioSource};
pub use global_sound::GlobalSound;
pub use spatial_sound::SpatialSound;

use self::{
    global_sound::global_sound_gain_changed_system,
    spatial_sound::spatial_sound_gain_changed_system,
};

pub struct OddioPlugin;

impl Plugin for OddioPlugin {
    fn build(&self, app: &mut App) {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let default_config = device.default_output_config().unwrap();
        let sample_rate = default_config.sample_rate();
        // Use the device's own channel count instead of assuming stereo -
        // requesting 2 channels on e.g. a mono USB audio device fails to
        // build the stream at all.
        let channels = default_config.channels();
        let config = cpal::StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let (mut root_mixer_handle, root_mixer) = oddio::split(oddio::Mixer::new());
        let (scene_handle, scene) = oddio::split(oddio::SpatialScene::new());
        root_mixer_handle.control().play(scene);

        // Scratch buffer used to render stereo audio before downmixing to
        // the device's actual channel layout, for any non-stereo device.
        let mut stereo_scratch: Vec<[f32; 2]> = Vec::new();
        let channels_usize = channels as usize;

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if channels_usize == 2 {
                        let frames = oddio::frame_stereo(data);
                        oddio::run(&root_mixer, sample_rate.0, frames);
                    } else {
                        let num_frames = data.len() / channels_usize.max(1);
                        stereo_scratch.resize(num_frames, [0.0; 2]);
                        oddio::run(&root_mixer, sample_rate.0, &mut stereo_scratch);

                        for (frame_index, stereo_frame) in stereo_scratch.iter().enumerate() {
                            let mono = 0.5 * (stereo_frame[0] + stereo_frame[1]);
                            let base = frame_index * channels_usize;
                            for channel_sample in &mut data[base..base + channels_usize] {
                                *channel_sample = mono;
                            }
                        }
                    }
                },
                move |err| {
                    eprintln!("{}", err);
                },
                None,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "Failed to build audio stream using output device's own {} channel(s): {}",
                    channels, error
                )
            });
        stream.play().unwrap();

        app.insert_non_send_resource(stream)
            .insert_resource(OddioContext {
                mixer: root_mixer_handle,
                spatial: scene_handle,
                sample_rate: sample_rate.0,
            })
            .add_asset::<AudioSource>()
            .init_asset_loader::<OggLoader>()
            .init_asset_loader::<WavLoader>()
            .add_systems(
                Last,
                (
                    spatial_sound_gain_changed_system.before(spatial_sound_system),
                    spatial_sound_system,
                    global_sound_gain_changed_system.before(global_sound_system),
                    global_sound_system,
                ),
            );
    }
}
