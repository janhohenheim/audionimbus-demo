use std::{num::NonZeroU32, ops::Deref as _};

use bevy::prelude::*;
use bevy_seedling::{
    context::StreamStartEvent,
    firewheel::diff::{Diff, Patch},
    node::RegisterNode as _,
    prelude::*,
};
use firewheel::{
    channel_config::ChannelConfig,
    event::ProcEvents,
    node::{
        AudioNode, AudioNodeInfo, AudioNodeProcessor, ConstructProcessorContext, EmptyConfig,
        ProcBuffers, ProcExtra, ProcInfo, ProcessStatus,
    },
};
use itertools::izip;

use crate::wrappers::*;

pub(super) fn plugin(app: &mut App) {
    app.register_node::<AmbisonicNode>()
        .register_node::<AmbisonicDecodeNode>();

    app.add_systems(PreStartup, setup_audionimbus);

    app.add_systems(
        PostUpdate,
        prepare_seedling_data.after(TransformSystems::Propagate),
    );
    app.add_observer(late_init);
}

pub(crate) fn setup_audionimbus(mut commands: Commands) {
    let context = audionimbus::Context::try_new(&audionimbus::ContextSettings::default()).unwrap();

    commands.insert_resource(AudionimbusContext(context));
}

#[derive(PoolLabel, PartialEq, Eq, Debug, Hash, Clone)]
pub(crate) struct AudionimbusPool;

#[derive(Event)]
pub(crate) struct AudionimbusReady;

fn late_init(
    stream_start: On<StreamStartEvent>,
    mut commands: Commands,
    context: Res<AudionimbusContext>,
) {
    let sample_rate = stream_start.sample_rate;
    let mut simulator = audionimbus::Simulator::builder(
        audionimbus::SceneParams::Default,
        sample_rate.get() as usize,
        FRAME_SIZE,
    )
    .with_direct(audionimbus::DirectSimulationSettings {
        max_num_occlusion_samples: 16,
    })
    .with_reflections(audionimbus::ReflectionsSimulationSettings::Convolution {
        max_num_rays: 2048,
        num_diffuse_samples: 8,
        max_duration: 2.0,
        max_order: AMBISONICS_ORDER,
        max_num_sources: 8,
        num_threads: 1,
    })
    .try_build(&context)
    .unwrap();
    let listener_source = audionimbus::Source::try_new(
        &simulator,
        &audionimbus::SourceSettings {
            flags: audionimbus::SimulationFlags::REFLECTIONS,
        },
    )
    .unwrap();
    simulator.add_source(&listener_source);
    simulator.commit();
    commands.insert_resource(ListenerSource(listener_source));
    commands.insert_resource(AudionimbusSimulator(simulator));

    let ambisonic_node = AmbisonicNode::new(context.clone());
    let ambisonic_decode_node = AmbisonicDecodeNode::new(context.clone());

    commands
        .spawn((
            SamplerPool(AudionimbusPool),
            VolumeNode::default(),
            VolumeNodeConfig {
                channels: NonZeroChannelCount::new(AMBISONICS_NUM_CHANNELS as u32).unwrap(),
            },
            sample_effects![ambisonic_node],
        ))
        // we only need one decoder
        .chain_node(ambisonic_decode_node);

    commands.trigger(AudionimbusReady);
}

pub(crate) const FRAME_SIZE: usize = 256;
pub(crate) const AMBISONICS_ORDER: usize = 2;
pub(crate) const AMBISONICS_NUM_CHANNELS: usize = (AMBISONICS_ORDER + 1).pow(2);
pub(crate) const GAIN_FACTOR_DIRECT: f32 = 1.0;
pub(crate) const GAIN_FACTOR_REFLECTIONS: f32 = 0.3;
pub(crate) const GAIN_FACTOR_REVERB: f32 = 0.1;

#[derive(Diff, Patch, Debug, Clone, Component)]
pub(crate) struct AmbisonicNode {
    pub(crate) source_position: Vec3,
    pub(crate) listener_position: Vec3,
    #[diff(skip)]
    pub(crate) context: audionimbus::Context,
    pub(crate) simulation_outputs: Option<AudionimbusSimulationOutputs>,
    pub(crate) reverb_effect_params: Option<AudionimbusReflectionEffectParams>,
}

impl AmbisonicNode {
    pub(crate) fn new(context: audionimbus::Context) -> Self {
        Self {
            context,
            source_position: default(),
            listener_position: default(),
            simulation_outputs: default(),
            reverb_effect_params: default(),
        }
    }
}

impl AudioNode for AmbisonicNode {
    type Configuration = EmptyConfig;

    fn info(&self, _config: &Self::Configuration) -> AudioNodeInfo {
        AudioNodeInfo::new()
            .debug_name("ambisonic node")
            // 1 -> 9
            .channel_config(ChannelConfig {
                num_inputs: ChannelCount::MONO,
                num_outputs: ChannelCount::new(AMBISONICS_NUM_CHANNELS as u32).unwrap(),
            })
    }

    fn construct_processor(
        &self,
        _config: &Self::Configuration,
        cx: ConstructProcessorContext,
    ) -> impl AudioNodeProcessor {
        let settings = audionimbus::AudioSettings {
            sampling_rate: cx.stream_info.sample_rate.get() as usize,
            frame_size: FRAME_SIZE,
        };
        AmbisonicProcessor {
            params: self.clone(),
            ambisonics_encode_effect: audionimbus::AmbisonicsEncodeEffect::try_new(
                &self.context,
                &settings,
                &audionimbus::AmbisonicsEncodeEffectSettings {
                    max_order: AMBISONICS_ORDER,
                },
            )
            .unwrap(),
            direct_effect: audionimbus::DirectEffect::try_new(
                &self.context,
                &settings,
                &audionimbus::DirectEffectSettings { num_channels: 1 },
            )
            .unwrap(),
            reflection_effect: audionimbus::ReflectionEffect::try_new(
                &self.context,
                &settings,
                &audionimbus::ReflectionEffectSettings::Convolution {
                    impulse_response_size: 2 * settings.sampling_rate,
                    num_channels: AMBISONICS_NUM_CHANNELS,
                },
            )
            .unwrap(),
            input_buffer: Vec::with_capacity(FRAME_SIZE),
            output_buffer: std::array::from_fn(|_| {
                Vec::with_capacity(cx.stream_info.max_block_frames.get() as usize * 2)
            }),
            max_block_frames: cx.stream_info.max_block_frames,
            started_draining: false,
        }
    }
}

struct AmbisonicProcessor {
    params: AmbisonicNode,
    ambisonics_encode_effect: audionimbus::AmbisonicsEncodeEffect,
    direct_effect: audionimbus::DirectEffect,
    reflection_effect: audionimbus::ReflectionEffect,
    input_buffer: Vec<f32>,
    output_buffer: [Vec<f32>; AMBISONICS_NUM_CHANNELS],
    max_block_frames: NonZeroU32,
    started_draining: bool,
}

impl AudioNodeProcessor for AmbisonicProcessor {
    fn process(
        &mut self,
        proc_info: &ProcInfo,
        ProcBuffers { inputs, outputs }: ProcBuffers,
        events: &mut ProcEvents,
        _: &mut ProcExtra,
    ) -> ProcessStatus {
        for patch in events.drain_patches::<AmbisonicNode>() {
            self.params.apply(patch);
        }

        // Don't early return on silent inputs: there is probably reverb left

        for frame in inputs[0].iter().take(proc_info.frames).copied() {
            self.input_buffer.push(frame);
            if self.input_buffer.len() != self.input_buffer.capacity() {
                continue;
            }
            // Buffer full, let's work!

            let (Some(simulation_outputs), Some(reverb_effect_params)) = (
                self.params.simulation_outputs.as_ref(),
                self.params.reverb_effect_params.as_ref(),
            ) else {
                self.input_buffer.clear();
                return ProcessStatus::ClearAllOutputs;
            };

            let source_position = self.params.source_position;

            let direct_effect_params = &simulation_outputs.direct;
            let reflection_effect_params = &simulation_outputs.reflections;

            let mut channel_ptrs = [std::ptr::null_mut(); 1];
            let mut input_container = [0.0; FRAME_SIZE];
            input_container.copy_from_slice(&self.input_buffer);
            let input_buffer =
                audionimbus::AudioBuffer::try_with_data(&input_container, &mut channel_ptrs)
                    .unwrap();

            let mut direct_container = [0.0; FRAME_SIZE];
            let mut channel_ptrs = [std::ptr::null_mut(); 1];
            let direct_buffer =
                audionimbus::AudioBuffer::try_with_data(&mut direct_container, &mut channel_ptrs)
                    .unwrap();
            let _effect_state = self.direct_effect.apply(
                &direct_effect_params.clone().into(),
                &input_buffer,
                &direct_buffer,
            );

            let listener_position = self.params.listener_position;
            let direction = source_position - listener_position;
            let direction = audionimbus::Direction::new(direction.x, direction.y, direction.z);

            let mut ambisonics_encode_container = [0.0; FRAME_SIZE * AMBISONICS_NUM_CHANNELS];
            let settings = audionimbus::AudioBufferSettings {
                num_channels: Some(AMBISONICS_NUM_CHANNELS),
                ..default()
            };
            let mut channel_ptrs = [std::ptr::null_mut(); AMBISONICS_NUM_CHANNELS];
            let ambisonics_encode_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
                &mut ambisonics_encode_container,
                &mut channel_ptrs,
                settings,
            )
            .unwrap();
            let ambisonics_encode_effect_params = audionimbus::AmbisonicsEncodeEffectParams {
                direction,
                order: AMBISONICS_ORDER,
            };
            let _effect_state = self.ambisonics_encode_effect.apply(
                &ambisonics_encode_effect_params,
                &direct_buffer,
                &ambisonics_encode_buffer,
            );

            let mut reflection_container = [0.0; FRAME_SIZE * AMBISONICS_NUM_CHANNELS];
            let settings = audionimbus::AudioBufferSettings {
                num_channels: Some(AMBISONICS_NUM_CHANNELS),
                ..default()
            };
            let mut channel_ptrs = [std::ptr::null_mut(); AMBISONICS_NUM_CHANNELS];
            let reflection_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
                &mut reflection_container,
                &mut channel_ptrs,
                settings,
            )
            .unwrap();
            let _effect_state = self.reflection_effect.apply(
                &reflection_effect_params.clone().into(),
                &input_buffer,
                &reflection_buffer,
            );

            let mut reverb_container = [0.0; FRAME_SIZE * AMBISONICS_NUM_CHANNELS];
            let settings = audionimbus::AudioBufferSettings {
                num_channels: Some(AMBISONICS_NUM_CHANNELS),
                ..default()
            };
            let mut channel_ptrs = [std::ptr::null_mut(); AMBISONICS_NUM_CHANNELS];
            let reverb_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
                &mut reverb_container,
                &mut channel_ptrs,
                settings,
            )
            .unwrap();

            let _effect_state = self.reflection_effect.apply(
                &reverb_effect_params.clone().into(),
                &input_buffer,
                &reverb_buffer,
            );

            izip!(
                ambisonics_encode_buffer.channels(),
                reflection_buffer.channels(),
                reverb_buffer.channels()
            )
            .map(|(direct_channel, reflection_channel, reverb_channel)| {
                izip!(
                    direct_channel.iter(),
                    reflection_channel.iter(),
                    reverb_channel.iter()
                )
                .map(|(direct_sample, reflections_sample, reverb_sample)| {
                    (direct_sample * GAIN_FACTOR_DIRECT
                        + reflections_sample * GAIN_FACTOR_REFLECTIONS
                        + reverb_sample * GAIN_FACTOR_REVERB)
                        / (GAIN_FACTOR_DIRECT + GAIN_FACTOR_REFLECTIONS + GAIN_FACTOR_REVERB)
                })
            })
            .enumerate()
            .for_each(|(i, channel)| {
                self.output_buffer[i].extend(channel);
            });
            self.input_buffer.clear();
        }

        if !self.started_draining {
            if (self.output_buffer[0].len() as f32) < self.max_block_frames.get() as f32 * 1.5 {
                return ProcessStatus::ClearAllOutputs;
            }
            self.started_draining = true;
        }

        let output_len = proc_info.frames;
        for (src, dst) in self.output_buffer.iter_mut().zip(outputs.iter_mut()) {
            for (i, out) in src.drain(..output_len).enumerate() {
                dst[i] = out;
            }
        }
        ProcessStatus::OutputsModified
    }
}

#[derive(Diff, Patch, Debug, Clone, Component)]
pub(crate) struct AmbisonicDecodeNode {
    pub(crate) listener_orientation: AudionimbusCoordinateSystem,
    #[diff(skip)]
    pub(crate) context: audionimbus::Context,
}

impl AmbisonicDecodeNode {
    pub(crate) fn new(context: audionimbus::Context) -> Self {
        Self {
            context,
            listener_orientation: default(),
        }
    }
}

impl AudioNode for AmbisonicDecodeNode {
    type Configuration = EmptyConfig;

    fn info(&self, _config: &Self::Configuration) -> AudioNodeInfo {
        AudioNodeInfo::new()
            .debug_name("ambisonic decode node")
            // 9 -> 2
            .channel_config(ChannelConfig {
                num_inputs: ChannelCount::new(AMBISONICS_NUM_CHANNELS as u32).unwrap(),
                num_outputs: ChannelCount::STEREO,
            })
    }

    fn construct_processor(
        &self,
        _config: &Self::Configuration,
        cx: ConstructProcessorContext,
    ) -> impl AudioNodeProcessor {
        let settings = audionimbus::AudioSettings {
            sampling_rate: cx.stream_info.sample_rate.get() as usize,
            frame_size: FRAME_SIZE,
        };
        let buffer_size = cx.stream_info.max_block_frames.get() as usize;
        let hrtf = audionimbus::Hrtf::try_new(
            &self.context,
            &settings,
            &audionimbus::HrtfSettings {
                volume_normalization: audionimbus::VolumeNormalization::RootMeanSquared,
                ..default()
            },
        )
        .unwrap();

        AmbisonicDecodeProcessor {
            params: self.clone(),
            hrtf: hrtf.clone(),
            ambisonics_decode_effect: audionimbus::AmbisonicsDecodeEffect::try_new(
                &self.context,
                &settings,
                &audionimbus::AmbisonicsDecodeEffectSettings {
                    max_order: AMBISONICS_ORDER,
                    speaker_layout: audionimbus::SpeakerLayout::Stereo,
                    hrtf: &hrtf,
                },
            )
            .unwrap(),
            input_buffer: std::array::from_fn(|_| Vec::with_capacity(FRAME_SIZE)),
            output_buffer: std::array::from_fn(|_| Vec::with_capacity(buffer_size.max(FRAME_SIZE))),
            max_block_frames: cx.stream_info.max_block_frames,
            started_draining: false,
        }
    }
}

struct AmbisonicDecodeProcessor {
    params: AmbisonicDecodeNode,
    hrtf: audionimbus::Hrtf,
    ambisonics_decode_effect: audionimbus::AmbisonicsDecodeEffect,
    input_buffer: [Vec<f32>; AMBISONICS_NUM_CHANNELS],
    output_buffer: [Vec<f32>; 2],
    max_block_frames: NonZeroU32,
    started_draining: bool,
}

impl AudioNodeProcessor for AmbisonicDecodeProcessor {
    fn process(
        &mut self,
        proc_info: &ProcInfo,
        ProcBuffers { inputs, outputs }: ProcBuffers,
        events: &mut ProcEvents,
        _: &mut ProcExtra,
    ) -> ProcessStatus {
        for patch in events.drain_patches::<AmbisonicDecodeNode>() {
            self.params.apply(patch);
        }

        if proc_info.in_silence_mask.all_channels_silent(inputs.len()) {
            return ProcessStatus::ClearAllOutputs;
        }

        for frame in 0..proc_info.frames {
            for (dst, src) in self.input_buffer.iter_mut().zip(inputs) {
                dst.push(src[frame]);
            }
            if self.input_buffer[0].len() != self.input_buffer[0].capacity() {
                continue;
            }
            // Buffer full

            let mut mix_container = [0.0; AMBISONICS_NUM_CHANNELS * FRAME_SIZE];
            for channel in 0..AMBISONICS_NUM_CHANNELS {
                mix_container[(channel * FRAME_SIZE)..(channel * FRAME_SIZE + FRAME_SIZE)]
                    .copy_from_slice(&self.input_buffer[channel]);
            }
            let mut channel_ptrs = [std::ptr::null_mut(); AMBISONICS_NUM_CHANNELS];
            let settings = audionimbus::AudioBufferSettings {
                num_channels: Some(AMBISONICS_NUM_CHANNELS),
                ..default()
            };
            let mix_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
                &mut mix_container,
                &mut channel_ptrs,
                settings,
            )
            .unwrap();

            let mut staging_container = [0.0; FRAME_SIZE * 2];
            let mut channel_ptrs = [std::ptr::null_mut(); 2];
            let settings = audionimbus::AudioBufferSettings {
                num_channels: Some(outputs.len()),
                ..default()
            };
            let staging_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
                &mut staging_container,
                &mut channel_ptrs,
                settings,
            )
            .unwrap();

            let ambisonics_decode_effect_params = audionimbus::AmbisonicsDecodeEffectParams {
                order: AMBISONICS_ORDER,
                hrtf: &self.hrtf,
                orientation: self.params.listener_orientation.into(),
                binaural: false,
            };
            let _effect_state = self.ambisonics_decode_effect.apply(
                &ambisonics_decode_effect_params,
                &mix_buffer,
                &staging_buffer,
            );

            let left = &staging_container[..FRAME_SIZE];
            let right = &staging_container[FRAME_SIZE..];
            self.output_buffer[0].extend(left);
            self.output_buffer[1].extend(right);
            for buff in &mut self.input_buffer {
                buff.clear();
            }
        }

        if !self.started_draining {
            if (self.output_buffer[0].len() as f32) < self.max_block_frames.get() as f32 * 1.5 {
                return ProcessStatus::ClearAllOutputs;
            }
            self.started_draining = true;
        }

        let output_len = outputs[0].len();
        for (src, dst) in self.output_buffer.iter_mut().zip(outputs.iter_mut()) {
            for (i, out) in src.drain(..output_len).enumerate() {
                dst[i] = out;
            }
        }
        ProcessStatus::OutputsModified
    }
}

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct AudionimbusContext(pub(crate) audionimbus::Context);

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct AudionimbusSimulator(
    pub(crate) audionimbus::Simulator<audionimbus::Direct, audionimbus::Reflections>,
);

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct ListenerSource(pub(crate) audionimbus::Source);

#[derive(Component, Deref, DerefMut)]
pub(crate) struct AudionimbusSource(pub(crate) audionimbus::Source);

fn prepare_seedling_data(
    mut nodes: Query<(&mut AudionimbusSource, &GlobalTransform, &SampleEffects)>,
    mut ambisonic_node: Query<&mut AmbisonicNode>,
    mut decode_node: Single<&mut AmbisonicDecodeNode>,
    camera: Single<&GlobalTransform, With<Camera3d>>,
    mut listener_source: ResMut<ListenerSource>,
    mut simulator: ResMut<AudionimbusSimulator>,
) -> Result {
    let camera_transform = camera.into_inner().compute_transform();
    let listener_position = camera_transform.translation;
    let listener_orientation: audionimbus::CoordinateSystem =
        AudionimbusCoordinateSystem::from(camera_transform).into();

    // Listener source to simulate reverb.
    listener_source.set_inputs(
        audionimbus::SimulationFlags::REFLECTIONS,
        audionimbus::SimulationInputs {
            source: audionimbus::CoordinateSystem {
                origin: audionimbus::Vector3::new(
                    listener_position.x,
                    listener_position.y,
                    listener_position.z,
                ),
                ..default()
            },
            direct_simulation: Some(audionimbus::DirectSimulationParameters {
                distance_attenuation: Some(audionimbus::DistanceAttenuationModel::Default),
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

    let simulation_flags =
        audionimbus::SimulationFlags::DIRECT | audionimbus::SimulationFlags::REFLECTIONS;
    simulator.set_shared_inputs(
        simulation_flags,
        &audionimbus::SimulationSharedInputs {
            listener: listener_orientation,
            num_rays: 2048,
            num_bounces: 8,
            duration: 2.0,
            order: AMBISONICS_ORDER,
            irradiance_min_distance: 1.0,
            pathing_visualization_callback: None,
        },
    );
    simulator.run_direct();
    simulator.run_reflections();

    let reverb_simulation_outputs =
        listener_source.get_outputs(audionimbus::SimulationFlags::REFLECTIONS);
    let reverb_effect_params = reverb_simulation_outputs.reflections();

    decode_node.listener_orientation = listener_orientation.into();

    for (mut source, transform, effects) in nodes.iter_mut() {
        let transform = transform.compute_transform();
        let source_position = transform.translation;

        source.set_inputs(
            simulation_flags,
            audionimbus::SimulationInputs {
                source: audionimbus::CoordinateSystem {
                    origin: audionimbus::Vector3::new(
                        source_position.x,
                        source_position.y,
                        source_position.z,
                    ),
                    ..default()
                },
                direct_simulation: Some(audionimbus::DirectSimulationParameters {
                    distance_attenuation: Some(audionimbus::DistanceAttenuationModel::Default),
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

        let simulation_outputs = source.get_outputs(simulation_flags);

        let mut node = ambisonic_node.get_effect_mut(effects)?;
        node.source_position = source_position;
        node.listener_position = listener_position;
        node.simulation_outputs = Some(simulation_outputs.into());
        node.reverb_effect_params = Some(reverb_effect_params.deref().clone().into());
    }

    Ok(())
}
