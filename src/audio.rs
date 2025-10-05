use std::ops::Deref as _;

use bevy::prelude::*;
use bevy_seedling::{
    firewheel::diff::{Diff, Patch},
    node::RegisterNode as _,
    prelude::ChannelCount,
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
    app.register_node::<AmbisonicNode>();
    app.register_node::<AmbisonicDecodeNode>();

    let context = audionimbus::Context::try_new(&audionimbus::ContextSettings::default()).unwrap();

    let settings = audionimbus::AudioSettings {
        frame_size: FRAME_SIZE,
        sampling_rate: SAMPLING_RATE,
    };

    let mut scene =
        audionimbus::Scene::try_new(&context, &audionimbus::SceneSettings::default()).unwrap();

    let walls = audionimbus::StaticMesh::try_new(
        &scene,
        &audionimbus::StaticMeshSettings {
            vertices: &[
                // Floor
                audionimbus::Point::new(-2.0, 0.0, -2.0),
                audionimbus::Point::new(2.0, 0.0, -2.0),
                audionimbus::Point::new(2.0, 0.0, 2.0),
                audionimbus::Point::new(-2.0, 0.0, 2.0),
                // Ceiling
                audionimbus::Point::new(-2.0, 4.0, -2.0),
                audionimbus::Point::new(2.0, 4.0, -2.0),
                audionimbus::Point::new(2.0, 4.0, 2.0),
                audionimbus::Point::new(-2.0, 4.0, 2.0),
                // Back wall
                audionimbus::Point::new(-2.0, 0.0, -2.0),
                audionimbus::Point::new(2.0, 0.0, -2.0),
                audionimbus::Point::new(2.0, 4.0, -2.0),
                audionimbus::Point::new(-2.0, 4.0, -2.0),
                // Left wall
                audionimbus::Point::new(-2.0, 0.0, -2.0),
                audionimbus::Point::new(-2.0, 0.0, 2.0),
                audionimbus::Point::new(-2.0, 4.0, 2.0),
                audionimbus::Point::new(-2.0, 4.0, -2.0),
            ],
            triangles: &[
                // Floor
                audionimbus::Triangle::new(0, 1, 2),
                audionimbus::Triangle::new(0, 2, 3),
                // Ceiling
                audionimbus::Triangle::new(4, 6, 5),
                audionimbus::Triangle::new(4, 7, 6),
                // Back wall
                audionimbus::Triangle::new(8, 9, 10),
                audionimbus::Triangle::new(8, 10, 11),
                // Left wall
                audionimbus::Triangle::new(12, 14, 13),
                audionimbus::Triangle::new(12, 15, 14),
            ],
            material_indices: &[0, 0, 0, 0, 0, 0, 0, 0],
            materials: &[audionimbus::Material::WOOD],
        },
    )
    .unwrap();
    scene.add_static_mesh(&walls);
    scene.commit();

    let mut simulator = audionimbus::Simulator::builder(
        audionimbus::SceneParams::Default,
        SAMPLING_RATE,
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
    simulator.set_scene(&scene);

    // Listener source used for reverb.
    let listener_source = audionimbus::Source::try_new(
        &simulator,
        &audionimbus::SourceSettings {
            flags: audionimbus::SimulationFlags::REFLECTIONS,
        },
    )
    .unwrap();
    simulator.add_source(&listener_source);
    app.insert_resource(ListenerSource {
        source: listener_source,
    });
    simulator.commit();

    app.insert_resource(Audio {
        context,
        settings,
        scene,
        simulator,
    });

    app.add_systems(
        PostUpdate,
        prepare_seedling_data.after(TransformSystems::Propagate),
    );
}

pub(crate) const FRAME_SIZE: usize = 1024;
pub(crate) const SAMPLING_RATE: usize = 48000;
pub(crate) const NUM_CHANNELS: usize = 2;
pub(crate) const AMBISONICS_ORDER: usize = 2;
pub(crate) const AMBISONICS_NUM_CHANNELS: usize = (AMBISONICS_ORDER + 1).pow(2);
pub(crate) const GAIN_FACTOR_DIRECT: f32 = 1.0;
pub(crate) const GAIN_FACTOR_REFLECTIONS: f32 = 0.3;
pub(crate) const GAIN_FACTOR_REVERB: f32 = 0.1;

#[derive(Diff, Patch, Debug, Clone, Component)]
pub(crate) struct AmbisonicNode {
    pub(crate) source_position: Vec3,
    pub(crate) listener_position: Vec3,
    pub(crate) settings: AudionimbusAudioSettings,
    #[diff(skip)]
    pub(crate) context: audionimbus::Context,
    pub(crate) simulation_outputs: Option<AudionimbusSimulationOutputs>,
    pub(crate) reverb_effect_params: Option<AudionimbusReflectionEffectParams>,
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
        _cx: ConstructProcessorContext,
    ) -> impl AudioNodeProcessor {
        AmbisonicProcessor {
            params: self.clone(),
            ambisonics_encode_effect: audionimbus::AmbisonicsEncodeEffect::try_new(
                &self.context,
                &self.settings.clone().into(),
                &audionimbus::AmbisonicsEncodeEffectSettings {
                    max_order: AMBISONICS_ORDER,
                },
            )
            .unwrap(),
            direct_effect: audionimbus::DirectEffect::try_new(
                &self.context,
                &self.settings.clone().into(),
                &audionimbus::DirectEffectSettings { num_channels: 1 },
            )
            .unwrap(),
            reflection_effect: audionimbus::ReflectionEffect::try_new(
                &self.context,
                &self.settings.clone().into(),
                &audionimbus::ReflectionEffectSettings::Convolution {
                    impulse_response_size: 2 * SAMPLING_RATE,
                    num_channels: AMBISONICS_NUM_CHANNELS,
                },
            )
            .unwrap(),
            simulation_outputs: self.simulation_outputs.clone(),
            reverb_effect_params: self
                .reverb_effect_params
                .as_ref()
                .map(|params| params.into()),
        }
    }
}

struct AmbisonicProcessor {
    params: AmbisonicNode,
    ambisonics_encode_effect: audionimbus::AmbisonicsEncodeEffect,
    direct_effect: audionimbus::DirectEffect,
    reflection_effect: audionimbus::ReflectionEffect,
    reverb_effect_params: Option<audionimbus::ReflectionEffectParams>,
    simulation_outputs: Option<AudionimbusSimulationOutputs>,
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

        // Firewheel will inform you if an input channel is silent. If they're
        // all silent, we can simply skip processing and save CPU time.
        if proc_info.in_silence_mask.all_channels_silent(inputs.len()) {
            // All inputs are silent.
            return ProcessStatus::ClearAllOutputs;
        }

        let (Some(simulation_outputs), Some(reverb_effect_params)) = (
            self.simulation_outputs.as_ref(),
            self.reverb_effect_params.as_ref(),
        ) else {
            return ProcessStatus::ClearAllOutputs;
        };

        let source_position = self.params.source_position;

        let direct_effect_params = &simulation_outputs.direct;
        let reflection_effect_params = &simulation_outputs.reflections;

        let mut flat_inputs = [0.0; FRAME_SIZE];
        flat_inputs.copy_from_slice(&inputs[0]);

        let mut channel_ptrs = [std::ptr::null_mut(); AMBISONICS_NUM_CHANNELS];
        let input_buffer =
            audionimbus::AudioBuffer::try_with_data(flat_inputs, &mut channel_ptrs).unwrap();

        let mut direct_container = [0.0; FRAME_SIZE];
        let mut channel_ptrs = [std::ptr::null_mut(); 1];
        let direct_buffer =
            audionimbus::AudioBuffer::try_with_data(&mut direct_container, &mut channel_ptrs)
                .unwrap();
        let _effect_state =
            self.direct_effect
                .apply(&direct_effect_params.into(), &input_buffer, &direct_buffer);

        let listener_position = self.params.listener_position;
        let direction = source_position - listener_position;
        let direction = audionimbus::Direction::new(direction.x, direction.y, direction.z);

        let mut ambisonics_encode_container = [0.0; FRAME_SIZE * AMBISONICS_NUM_CHANNELS];
        let settings = audionimbus::AudioBufferSettings {
            num_channels: Some(AMBISONICS_NUM_CHANNELS),
            ..Default::default()
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

        let mut reflection_container = vec![0.0; FRAME_SIZE * AMBISONICS_NUM_CHANNELS];
        let settings = audionimbus::AudioBufferSettings {
            num_channels: Some(AMBISONICS_NUM_CHANNELS),
            ..Default::default()
        };
        let mut channel_ptrs = [std::ptr::null_mut(); AMBISONICS_NUM_CHANNELS];
        let reflection_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
            &mut reflection_container,
            &mut channel_ptrs,
            settings,
        )
        .unwrap();
        let _effect_state = self.reflection_effect.apply(
            &reflection_effect_params.into(),
            &input_buffer,
            &reflection_buffer,
        );

        let mut reverb_container = vec![0.0; FRAME_SIZE * AMBISONICS_NUM_CHANNELS];
        let settings = audionimbus::AudioBufferSettings {
            num_channels: Some(AMBISONICS_NUM_CHANNELS),
            ..Default::default()
        };
        let mut channel_ptrs = [std::ptr::null_mut(); AMBISONICS_NUM_CHANNELS];
        let reverb_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
            &mut reverb_container,
            &mut channel_ptrs,
            settings,
        )
        .unwrap();
        let _effect_state =
            self.reflection_effect
                .apply(&reverb_effect_params, &input_buffer, &reverb_buffer);

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
            for (output, sample) in outputs[i].iter_mut().zip(channel) {
                *output = sample;
            }
        });

        ProcessStatus::outputs_not_silent()
    }
}

#[derive(Diff, Patch, Debug, Clone, Component)]
pub(crate) struct AmbisonicDecodeNode {
    pub(crate) listener_orientation: AudionimbusCoordinateSystem,
    #[diff(skip)]
    pub(crate) context: audionimbus::Context,
    pub(crate) settings: AudionimbusAudioSettings,
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
        _cx: ConstructProcessorContext,
    ) -> impl AudioNodeProcessor {
        let hrtf = audionimbus::Hrtf::try_new(
            &self.context,
            &self.settings.clone().into(),
            &audionimbus::HrtfSettings {
                volume_normalization: audionimbus::VolumeNormalization::RootMeanSquared,
                ..Default::default()
            },
        )
        .unwrap();

        AmbisonicDecodeProcessor {
            params: self.clone(),
            listener_orientation: self.listener_orientation.into(),
            hrtf: hrtf.clone(),
            ambisonics_decode_effect: audionimbus::AmbisonicsDecodeEffect::try_new(
                &self.context,
                &self.settings.clone().into(),
                &audionimbus::AmbisonicsDecodeEffectSettings {
                    max_order: AMBISONICS_ORDER,
                    speaker_layout: audionimbus::SpeakerLayout::Stereo,
                    hrtf: &hrtf,
                },
            )
            .unwrap(),
        }
    }
}

struct AmbisonicDecodeProcessor {
    params: AmbisonicDecodeNode,
    listener_orientation: audionimbus::CoordinateSystem,
    hrtf: audionimbus::Hrtf,
    ambisonics_decode_effect: audionimbus::AmbisonicsDecodeEffect,
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

        // Firewheel will inform you if an input channel is silent. If they're
        // all silent, we can simply skip processing and save CPU time.
        if proc_info.in_silence_mask.all_channels_silent(inputs.len()) {
            // All inputs are silent.
            return ProcessStatus::ClearAllOutputs;
        }

        let mut mix_container = [0.0; AMBISONICS_NUM_CHANNELS];
        for channel in 0..AMBISONICS_NUM_CHANNELS {
            mix_container[(channel * FRAME_SIZE)..(channel * FRAME_SIZE + FRAME_SIZE)]
                .copy_from_slice(inputs[channel]);
        }
        let mut channel_ptrs = [std::ptr::null_mut(); AMBISONICS_NUM_CHANNELS];
        let settings = audionimbus::AudioBufferSettings {
            num_channels: Some(AMBISONICS_NUM_CHANNELS),
            ..Default::default()
        };
        let mix_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
            &mut mix_container,
            &mut channel_ptrs,
            settings,
        )
        .unwrap();

        let mut staging_container = vec![0.0; FRAME_SIZE * NUM_CHANNELS];
        let mut channel_ptrs = [std::ptr::null_mut(); NUM_CHANNELS];
        let settings = audionimbus::AudioBufferSettings {
            num_channels: Some(NUM_CHANNELS),
            ..Default::default()
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
            orientation: self.listener_orientation,
            binaural: false,
        };
        let _effect_state = self.ambisonics_decode_effect.apply(
            &ambisonics_decode_effect_params,
            &mix_buffer,
            &staging_buffer,
        );

        for i in 0..NUM_CHANNELS {
            outputs[i].copy_from_slice(&staging_container[i * FRAME_SIZE..(i + 1) * FRAME_SIZE]);
        }

        ProcessStatus::outputs_not_silent()
    }
}

#[derive(Resource)]
pub(crate) struct Audio {
    pub(crate) context: audionimbus::Context,
    pub(crate) settings: audionimbus::AudioSettings,
    pub(crate) scene: audionimbus::Scene,
    pub(crate) simulator: audionimbus::Simulator<audionimbus::Direct, audionimbus::Reflections>,
}

#[derive(Resource)]
pub(crate) struct ListenerSource {
    // Special source used for reverb.
    pub(crate) source: audionimbus::Source,
}

#[derive(Component, Deref, DerefMut)]
pub(crate) struct AudionimbusSource(pub(crate) audionimbus::Source);

fn prepare_seedling_data(
    mut nodes: Query<(
        &mut AmbisonicNode,
        &mut AmbisonicDecodeNode,
        &mut AudionimbusSource,
        &GlobalTransform,
    )>,
    camera: Single<&GlobalTransform, With<Camera3d>>,
    mut listener_source: ResMut<ListenerSource>,
    mut audio: ResMut<Audio>,
) {
    let camera_transform = camera.into_inner().compute_transform();
    let listener_position = camera_transform.translation;
    let listener_orientation: audionimbus::CoordinateSystem =
        AudionimbusCoordinateSystem::from(camera_transform).into();

    // Listener source to simulate reverb.
    listener_source.source.set_inputs(
        audionimbus::SimulationFlags::REFLECTIONS,
        audionimbus::SimulationInputs {
            source: audionimbus::CoordinateSystem {
                origin: audionimbus::Vector3::new(
                    listener_position.x,
                    listener_position.y,
                    listener_position.z,
                ),
                ..Default::default()
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
    audio.simulator.set_shared_inputs(
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
    audio.simulator.run_direct();
    audio.simulator.run_reflections();

    let reverb_simulation_outputs = listener_source
        .source
        .get_outputs(audionimbus::SimulationFlags::REFLECTIONS);
    let reverb_effect_params = reverb_simulation_outputs.reflections();

    for (mut node, mut decode_node, mut source, transform) in nodes.iter_mut() {
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
                    ..Default::default()
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

        *node = AmbisonicNode {
            source_position,
            listener_position,
            settings: audio.settings.into(),
            context: audio.context.clone(),
            simulation_outputs: Some((&simulation_outputs).into()),
            reverb_effect_params: Some(reverb_effect_params.deref().into()),
        };
        *decode_node = AmbisonicDecodeNode {
            listener_orientation: listener_orientation.into(),
            context: audio.context.clone(),
            settings: audio.settings.into(),
        };
    }
}
