use std::ops::Deref as _;

use bevy::{math::Vec3, transform::components::Transform};
use firewheel::diff::{Diff, Patch, RealtimeClone};

#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub(crate) struct AudionimbusSimulationOutputs {
    pub(crate) direct: AudionimbusDirectEffectParams,
    pub(crate) reflections: AudionimbusReflectionEffectParams,
}

impl From<&audionimbus::SimulationOutputs> for AudionimbusSimulationOutputs {
    fn from(outputs: &audionimbus::SimulationOutputs) -> Self {
        Self {
            direct: outputs.direct().deref().into(),
            reflections: outputs.reflections().deref().into(),
        }
    }
}

/// Parameters for applying a reflection effect to an audio buffer.
#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub(crate) struct AudionimbusReflectionEffectParams {
    /// Type of reflection effect algorithm to use.
    pub(crate) reflection_effect_type: AudionimbusReflectionEffectType,

    /// The impulse response.
    #[diff(skip)]
    pub(crate) impulse_response: ReflectionEffectIR,

    /// 3-band reverb decay times (RT60).
    pub(crate) reverb_times: [f32; 3],

    /// 3-band EQ coefficients applied to the parametric part to ensure smooth transition.
    pub(crate) equalizer: AudionimbusEqualizer<3>,

    /// Samples after which parametric part starts.
    pub(crate) delay: u64,

    /// Number of IR channels to process.
    /// May be less than the number of channels specified when creating the effect, in which case CPU usage will be reduced.
    pub(crate) num_channels: u64,

    /// Number of IR samples per channel to process.
    /// May be less than the number of samples specified when creating the effect, in which case CPU usage will be reduced.
    pub(crate) impulse_response_size: u64,

    /// The TrueAudio Next device to use for convolution processing.
    #[diff(skip)]
    pub(crate) true_audio_next_device: audionimbus::TrueAudioNextDevice,

    /// The TrueAudio Next slot index to use for convolution processing.
    /// The slot identifies the IR to use.
    pub(crate) true_audio_next_slot: u64,
}

impl From<&AudionimbusReflectionEffectParams> for audionimbus::ReflectionEffectParams {
    fn from(params: &AudionimbusReflectionEffectParams) -> Self {
        Self {
            num_channels: params.num_channels as usize,
            impulse_response_size: params.impulse_response_size as usize,
            true_audio_next_device: params.true_audio_next_device.clone().into(),
            true_audio_next_slot: params.true_audio_next_slot as usize,
            reflection_effect_type: params.reflection_effect_type.clone().into(),
            impulse_response: params.impulse_response.0,
            reverb_times: params.reverb_times,
            equalizer: params.equalizer.clone().into(),
            delay: params.delay as usize,
        }
    }
}

impl From<&audionimbus::ReflectionEffectParams> for AudionimbusReflectionEffectParams {
    fn from(value: &audionimbus::ReflectionEffectParams) -> Self {
        Self {
            num_channels: value.num_channels as u64,
            impulse_response_size: value.impulse_response_size as u64,
            true_audio_next_device: value.true_audio_next_device.clone().into(),
            true_audio_next_slot: value.true_audio_next_slot as u64,
            reflection_effect_type: value.reflection_effect_type.into(),
            impulse_response: ReflectionEffectIR(value.impulse_response),
            reverb_times: value.reverb_times,
            equalizer: value.equalizer.into(),
            delay: value.delay as u64,
        }
    }
}

#[derive(Debug, Clone, RealtimeClone, PartialEq)]
pub(crate) struct ReflectionEffectIR(pub(crate) audionimbus_sys::IPLReflectionEffectIR);
unsafe impl Send for ReflectionEffectIR {}
unsafe impl Sync for ReflectionEffectIR {}

/// Type of reflection effect algorithm to use.
#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub(crate) enum AudionimbusReflectionEffectType {
    /// Multi-channel convolution reverb.
    /// Reflections reaching the listener are encoded in an Impulse Response (IR), which is a filter that records each reflection as it arrives.
    /// This algorithm renders reflections with the most detail, but may result in significant CPU usage.
    /// Using a reflection mixer with this algorithm provides a reduction in CPU usage.
    Convolution,

    /// Parametric (or artificial) reverb, using feedback delay networks.
    /// The reflected sound field is reduced to a few numbers that describe how reflected energy decays over time.
    /// This is then used to drive an approximate model of reverberation in an indoor space.
    /// This algorithm results in lower CPU usage, but cannot render individual echoes, especially in outdoor spaces.
    /// A reflection mixer cannot be used with this algorithm.
    Parametric,

    /// A hybrid of convolution and parametric reverb.
    /// The initial portion of the IR is rendered using convolution reverb, but the later part is used to estimate a parametric reverb.
    /// The point in the IR where this transition occurs can be controlled.
    /// This algorithm allows a trade-off between rendering quality and CPU usage.
    /// An reflection mixer cannot be used with this algorithm.
    Hybrid,

    /// Multi-channel convolution reverb, using AMD TrueAudio Next for GPU acceleration.
    /// This algorithm is similar to [`Self::Convolution`], but uses the GPU instead of the CPU for processing, allowing significantly more sources to be processed.
    /// A reflection mixer must be used with this algorithm, because the GPU will process convolution reverb at a single point in your audio processing pipeline.
    TrueAudioNext,
}

impl From<audionimbus::ReflectionEffectType> for AudionimbusReflectionEffectType {
    fn from(value: audionimbus::ReflectionEffectType) -> Self {
        match value {
            audionimbus::ReflectionEffectType::Convolution => Self::Convolution,
            audionimbus::ReflectionEffectType::Parametric => Self::Parametric,
            audionimbus::ReflectionEffectType::Hybrid => Self::Hybrid,
            audionimbus::ReflectionEffectType::TrueAudioNext => Self::TrueAudioNext,
        }
    }
}

impl From<AudionimbusReflectionEffectType> for audionimbus::ReflectionEffectType {
    fn from(value: AudionimbusReflectionEffectType) -> Self {
        match value {
            AudionimbusReflectionEffectType::Convolution => Self::Convolution,
            AudionimbusReflectionEffectType::Parametric => Self::Parametric,
            AudionimbusReflectionEffectType::Hybrid => Self::Hybrid,
            AudionimbusReflectionEffectType::TrueAudioNext => Self::TrueAudioNext,
        }
    }
}

/// Parameters for applying a direct effect to an audio buffer.
#[derive(Debug, Clone, RealtimeClone, Default, Diff, Patch, PartialEq)]
pub(crate) struct AudionimbusDirectEffectParams {
    /// Optional distance attenuation, with a value between 0.0 and 1.0.
    pub(crate) distance_attenuation: Option<f32>,

    /// Optional air absorption.
    pub(crate) air_absorption: Option<AudionimbusEqualizer<3>>,

    /// Optional directivity term, with a value between 0.0 and 1.0.
    pub(crate) directivity: Option<f32>,

    /// Optional occlusion factor, with a value between 0.0 and 1.0.
    pub(crate) occlusion: Option<f32>,

    /// Optional transmission.
    pub(crate) transmission: Option<AudionimbusTransmission>,
}

impl From<&AudionimbusDirectEffectParams> for audionimbus::DirectEffectParams {
    fn from(params: &AudionimbusDirectEffectParams) -> Self {
        Self {
            distance_attenuation: params.distance_attenuation,
            air_absorption: params.air_absorption.clone().map(|eq| eq.into()),
            directivity: params.directivity,
            occlusion: params.occlusion,
            transmission: params.transmission.clone().map(|trans| trans.into()),
        }
    }
}

impl From<&audionimbus::DirectEffectParams> for AudionimbusDirectEffectParams {
    fn from(params: &audionimbus::DirectEffectParams) -> Self {
        Self {
            distance_attenuation: params.distance_attenuation,
            air_absorption: params.air_absorption.map(|eq| eq.into()),
            directivity: params.directivity,
            occlusion: params.occlusion,
            transmission: params.transmission.map(|trans| trans.into()),
        }
    }
}

#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub(crate) enum AudionimbusTransmission {
    /// Frequency-independent transmission.
    FrequencyIndependent(AudionimbusEqualizer<3>),

    /// Frequency-dependent transmission.
    FrequencyDependent(AudionimbusEqualizer<3>),
}

impl From<AudionimbusTransmission> for audionimbus::Transmission {
    fn from(transmission: AudionimbusTransmission) -> Self {
        match transmission {
            AudionimbusTransmission::FrequencyIndependent(eq) => {
                Self::FrequencyIndependent(eq.into())
            }
            AudionimbusTransmission::FrequencyDependent(eq) => Self::FrequencyDependent(eq.into()),
        }
    }
}

impl From<audionimbus::Transmission> for AudionimbusTransmission {
    fn from(transmission: audionimbus::Transmission) -> Self {
        match transmission {
            audionimbus::Transmission::FrequencyIndependent(eq) => {
                Self::FrequencyIndependent(eq.into())
            }
            audionimbus::Transmission::FrequencyDependent(eq) => {
                Self::FrequencyDependent(eq.into())
            }
        }
    }
}

#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub(crate) struct AudionimbusEqualizer<const N: usize>(pub(crate) [f32; N]);

impl<const N: usize> From<AudionimbusEqualizer<N>> for audionimbus::Equalizer<N> {
    fn from(eq: AudionimbusEqualizer<N>) -> Self {
        Self(eq.0)
    }
}

impl<const N: usize> From<audionimbus::Equalizer<N>> for AudionimbusEqualizer<N> {
    fn from(eq: audionimbus::Equalizer<N>) -> Self {
        Self(eq.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Diff, Patch)]
pub(crate) struct AudionimbusCoordinateSystem {
    /// Unit vector pointing to the right (local +x axis).
    pub(crate) right: Vec3,

    /// Unit vector pointing upwards (local +y axis).
    pub(crate) up: Vec3,

    /// Unit vector pointing forwards (local -z axis).
    pub(crate) ahead: Vec3,

    /// The origin, relative to the canonical coordinate system.
    pub(crate) origin: Vec3,
}

impl From<AudionimbusCoordinateSystem> for audionimbus::CoordinateSystem {
    fn from(system: AudionimbusCoordinateSystem) -> Self {
        Self {
            right: audionimbus::Vector3 {
                x: system.right.x,
                y: system.right.y,
                z: system.right.z,
            },
            up: audionimbus::Vector3 {
                x: system.up.x,
                y: system.up.y,
                z: system.up.z,
            },
            ahead: audionimbus::Vector3 {
                x: system.ahead.x,
                y: system.ahead.y,
                z: system.ahead.z,
            },
            origin: audionimbus::Point {
                x: system.origin.x,
                y: system.origin.y,
                z: system.origin.z,
            },
        }
    }
}

impl From<audionimbus::CoordinateSystem> for AudionimbusCoordinateSystem {
    fn from(system: audionimbus::CoordinateSystem) -> Self {
        Self {
            right: Vec3::new(system.right.x, system.right.y, system.right.z),
            up: Vec3::new(system.up.x, system.up.y, system.up.z),
            ahead: Vec3::new(system.ahead.x, system.ahead.y, system.ahead.z),
            origin: Vec3::new(system.origin.x, system.origin.y, system.origin.z),
        }
    }
}

impl From<Transform> for AudionimbusCoordinateSystem {
    fn from(transform: Transform) -> Self {
        let listener_position = transform.translation;

        let listener_orientation_right = transform.right();
        let listener_orientation_up = transform.up();
        let listener_orientation_ahead = transform.forward();
        Self {
            right: listener_orientation_right.into(),
            up: listener_orientation_up.into(),
            ahead: listener_orientation_ahead.into(),
            origin: listener_position,
        }
    }
}
