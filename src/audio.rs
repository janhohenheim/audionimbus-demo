use bevy::prelude::*;
use bevy::utils::Duration;
use rodio::{OutputStream, Sink, Source};

use super::character::CharacterMarker;
use super::viewpoint::Viewpoint;

pub const FRAME_SIZE: usize = 1024;
pub const SAMPLING_RATE: usize = 48000;
pub const NUM_CHANNELS: usize = 2;
pub const AMBISONICS_ORDER: usize = 2;
pub const AMBISONICS_NUM_CHANNELS: usize = (AMBISONICS_ORDER + 1).pow(2);

#[derive(Resource)]
pub struct Audio {
    pub context: audionimbus::Context,
    pub settings: audionimbus::AudioSettings,
    pub scene: audionimbus::Scene,
    pub simulator: audionimbus::Simulator,
    pub hrtf: audionimbus::Hrtf,
    pub direct_effect: audionimbus::DirectEffect,
    pub ambisonics_encode_effect: audionimbus::AmbisonicsEncodeEffect,
    pub ambisonics_decode_effect: audionimbus::AmbisonicsDecodeEffect,
    pub sink: Sink,
    pub timer: Timer,
}

pub struct AudioFrame {
    position: usize,
    data: Vec<f32>,
    channels: u16,
}

impl AudioFrame {
    pub fn new(data: Vec<f32>, channels: u16) -> Self {
        Self {
            position: 0,
            data,
            channels,
        }
    }
}

impl Iterator for AudioFrame {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.data.len() {
            let sample = self.data[self.position];
            self.position += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl Source for AudioFrame {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.data.len())
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        SAMPLING_RATE as u32
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(
            self.data.len() as f32 / (SAMPLING_RATE as f32 * self.channels as f32),
        ))
    }
}

#[derive(Resource, Debug)]
pub struct SineWaveParams {
    pub frequency: f32,
    pub amplitude: f32,
    pub phase: f32,
    pub angle: f32,
}

#[derive(Component, Debug)]
#[require(GlobalTransform)]
pub struct AudioSource {
    pub source: audionimbus::Source,
    pub data: Vec<audionimbus::Sample>, // Mono
    pub is_repeating: bool,
    pub position: usize,
}

pub struct Plugin;

impl Plugin {
    fn process_frame(
        mut commands: Commands,
        query_character: Query<(&GlobalTransform, &Viewpoint), With<CharacterMarker>>,
        mut query_audio_sources: Query<(Entity, &GlobalTransform, &mut AudioSource)>,
        time: Res<Time>,
        mut audio: ResMut<Audio>,
    ) {
        audio.timer.tick(time.delta());

        let (global_transform, viewpoint) = query_character.single();
        let listener_position = global_transform.translation() + viewpoint.translation;

        let listener_orientation_right = viewpoint.rotation * Vec3::X;
        let listener_orientation_up = viewpoint.rotation * Vec3::Y;
        let listener_orientation_ahead = viewpoint.rotation * -Vec3::Z;
        let listener_orientation = audionimbus::CoordinateSystem {
            right: audionimbus::Vector3::new(
                listener_orientation_right.x,
                listener_orientation_right.y,
                listener_orientation_right.z,
            ),
            up: audionimbus::Vector3::new(
                listener_orientation_up.x,
                listener_orientation_up.y,
                listener_orientation_up.z,
            ),
            ahead: audionimbus::Vector3::new(
                listener_orientation_ahead.x,
                listener_orientation_ahead.y,
                listener_orientation_ahead.z,
            ),
            origin: audionimbus::Point::new(
                listener_position.x,
                listener_position.y,
                listener_position.z,
            ),
        };

        let simulation_flags = audionimbus::SimulationFlags::DIRECT
            | audionimbus::SimulationFlags::REFLECTIONS
            | audionimbus::SimulationFlags::PATHING;
        audio.simulator.set_shared_inputs(
            simulation_flags,
            &audionimbus::SimulationSharedInputs {
                listener: listener_orientation,
                num_rays: 4096,
                num_bounces: 16,
                duration: 2.0,
                order: AMBISONICS_ORDER,
                irradiance_min_distance: 1.0,
                pathing_visualization_callback: None,
            },
        );
        audio.simulator.run_direct();
        audio.simulator.run_reflections();

        let times_finished_this_tick = audio.timer.times_finished_this_tick();

        for _ in 0..times_finished_this_tick {
            let mut deinterleaved_container = vec![0.0; FRAME_SIZE * NUM_CHANNELS];

            // Iterate over each audio source.
            for (entity, source_global_transform, mut audio_source) in
                query_audio_sources.iter_mut()
            {
                let frame = if audio_source.is_repeating {
                    let frame: Vec<_> = (0..FRAME_SIZE)
                        .map(|i| {
                            audio_source.data[(audio_source.position + i) % audio_source.data.len()]
                        })
                        .collect();

                    // Advance sample position.
                    audio_source.position =
                        (audio_source.position + FRAME_SIZE) % audio_source.data.len();

                    frame
                } else {
                    let frame = (0..FRAME_SIZE)
                        .map(|i| {
                            let idx = audio_source.position + i;
                            // If no more samples, fill with silence.
                            if idx < audio_source.data.len() {
                                audio_source.data[idx]
                            } else {
                                0.0
                            }
                        })
                        .collect();

                    // Advance sample position.
                    audio_source.position += FRAME_SIZE;

                    // If there are no more audio samples to play back.
                    if audio_source.position >= audio_source.data.len() {
                        // Despawn audio source.
                        commands.entity(entity).despawn();
                    }

                    frame
                };

                let source_position = source_global_transform.translation();

                audio_source.source.set_inputs(
                    simulation_flags,
                    audionimbus::SimulationInputs {
                        source: audionimbus::CoordinateSystem {
                            origin: audionimbus::Vector3::new(
                                source_position.x,
                                source_position.y,
                                source_position.z,
                            ),
                            ..Default::default()
                        },
                        direct_simulation: Some(audionimbus::DirectSimulationParameters {
                            distance_attenuation: Some(
                                audionimbus::DistanceAttenuationModel::Default,
                            ),
                            air_absorption: Some(audionimbus::AirAbsorptionModel::Default),
                            directivity: Some(audionimbus::Directivity::default()),
                            occlusion: Some(audionimbus::Occlusion {
                                transmission: Some(audionimbus::TransmissionParameters {
                                    num_transmission_rays: 8,
                                }),
                                algorithm: audionimbus::OcclusionAlgorithm::Raycast,
                            }),
                        }),
                        reflections_simulation: Some(
                            audionimbus::ReflectionsSimulationParameters::Convolution {
                                baked_data_identifier: None,
                            },
                        ),
                        pathing_simulation: None,
                    },
                );

                let simulation_outputs = audio_source.source.get_outputs(simulation_flags);
                let direct_effect_params = simulation_outputs.direct();

                let input_buffer = audionimbus::AudioBuffer::try_with_data(&frame).unwrap();

                let mut direct_container = vec![0.0; FRAME_SIZE];
                let direct_buffer =
                    audionimbus::AudioBuffer::try_with_data(&mut direct_container).unwrap();
                let _effect_state =
                    audio
                        .direct_effect
                        .apply(&direct_effect_params, &input_buffer, &direct_buffer);

                let direction = source_position - listener_position;
                let direction = audionimbus::Direction::new(direction.x, direction.y, direction.z);

                let mut ambisonics_encode_container =
                    vec![0.0; FRAME_SIZE * AMBISONICS_NUM_CHANNELS];
                let ambisonics_encode_buffer =
                    audionimbus::AudioBuffer::try_with_data_and_settings(
                        &mut ambisonics_encode_container,
                        &audionimbus::AudioBufferSettings {
                            num_channels: Some(AMBISONICS_NUM_CHANNELS),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                let ambisonics_encode_effect_params = audionimbus::AmbisonicsEncodeEffectParams {
                    direction,
                    order: AMBISONICS_ORDER,
                };
                let _effect_state = audio.ambisonics_encode_effect.apply(
                    &ambisonics_encode_effect_params,
                    &direct_buffer,
                    &ambisonics_encode_buffer,
                );

                let mut staging_container = vec![0.0; FRAME_SIZE * NUM_CHANNELS];
                let staging_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
                    &mut staging_container,
                    &audionimbus::AudioBufferSettings {
                        num_channels: Some(NUM_CHANNELS),
                        ..Default::default()
                    },
                )
                .unwrap();

                let ambisonics_decode_effect_params = audionimbus::AmbisonicsDecodeEffectParams {
                    order: AMBISONICS_ORDER,
                    hrtf: &audio.hrtf,
                    orientation: listener_orientation,
                    binaural: false,
                };
                let _effect_state = audio.ambisonics_decode_effect.apply(
                    &ambisonics_decode_effect_params,
                    &ambisonics_encode_buffer,
                    &staging_buffer,
                );

                deinterleaved_container = staging_container
                    .iter()
                    .zip(deinterleaved_container.iter())
                    .map(|(a, b)| a + b)
                    .collect();
            }

            let deinterleaved_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
                &mut deinterleaved_container,
                &audionimbus::AudioBufferSettings {
                    num_channels: Some(NUM_CHANNELS),
                    ..Default::default()
                },
            )
            .unwrap();
            let mut interleaved = vec![0.0; FRAME_SIZE * NUM_CHANNELS];
            deinterleaved_buffer.interleave(&audio.context, &mut interleaved);
            let source = AudioFrame::new(interleaved, 2);

            audio.sink.append(source);
        }
    }
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        app.insert_non_send_resource(stream);

        let context =
            audionimbus::Context::try_new(&audionimbus::ContextSettings::default()).unwrap();

        let settings = audionimbus::AudioSettings {
            frame_size: FRAME_SIZE,
            sampling_rate: SAMPLING_RATE,
        };

        let scene =
            audionimbus::Scene::try_new(&context, &audionimbus::SceneSettings::default()).unwrap();

        let simulation_settings = audionimbus::SimulationSettings {
            scene_params: audionimbus::SceneParams::Default,
            direct_simulation: Some(audionimbus::DirectSimulationSettings {
                max_num_occlusion_samples: 16,
            }),
            reflections_simulation: Some(audionimbus::ReflectionsSimulationSettings::Convolution {
                max_num_rays: 4096,
                num_diffuse_samples: 32,
                max_duration: 2.0,
                max_order: AMBISONICS_ORDER,
                max_num_sources: 8,
                num_threads: 1,
            }),
            pathing_simulation: Some(audionimbus::PathingSimulationSettings {
                num_visibility_samples: 16,
            }),
            sampling_rate: SAMPLING_RATE,
            frame_size: FRAME_SIZE,
        };
        let mut simulator = audionimbus::Simulator::try_new(&context, simulation_settings).unwrap();
        simulator.set_scene(&scene);
        simulator.commit();

        let hrtf = audionimbus::Hrtf::try_new(
            &context,
            &settings,
            &audionimbus::HrtfSettings {
                volume_normalization: audionimbus::VolumeNormalization::RootMeanSquared,
                ..Default::default()
            },
        )
        .unwrap();

        let direct_effect = audionimbus::DirectEffect::try_new(
            &context,
            &settings,
            &audionimbus::DirectEffectSettings { num_channels: 1 },
        )
        .unwrap();

        let ambisonics_encode_effect = audionimbus::AmbisonicsEncodeEffect::try_new(
            &context,
            &settings,
            &audionimbus::AmbisonicsEncodeEffectSettings {
                max_order: AMBISONICS_ORDER,
            },
        )
        .unwrap();

        let ambisonics_decode_effect = audionimbus::AmbisonicsDecodeEffect::try_new(
            &context,
            &settings,
            &audionimbus::AmbisonicsDecodeEffectSettings {
                max_order: AMBISONICS_ORDER,
                speaker_layout: audionimbus::SpeakerLayout::Stereo,
                hrtf: &hrtf,
            },
        )
        .unwrap();

        app.insert_resource(Audio {
            context,
            settings,
            scene,
            simulator,
            hrtf,
            direct_effect,
            ambisonics_encode_effect,
            ambisonics_decode_effect,
            sink,
            timer: Timer::new(
                Duration::from_secs_f32(FRAME_SIZE as f32 / SAMPLING_RATE as f32),
                TimerMode::Repeating,
            ),
        });

        app.insert_resource(SineWaveParams {
            frequency: 440.0,
            amplitude: 0.2,
            phase: 0.0,
            angle: 0.0,
        });

        app.add_systems(PostUpdate, Self::process_frame);
    }
}

pub fn sine_wave(
    frequency: f32,
    sample_rate: usize,
    amplitude: f32,
    num_samples: usize,
) -> Vec<audionimbus::Sample> {
    let mut phase: f32 = 0.0;
    let phase_increment = 2.0 * std::f32::consts::PI * frequency / sample_rate as f32;
    (0..num_samples)
        .map(|_| {
            let sample = amplitude * phase.sin();
            phase = (phase + phase_increment) % (2.0 * std::f32::consts::PI);
            sample
        })
        .collect()
}
