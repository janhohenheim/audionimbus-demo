use arrayvec::ArrayVec;
use firewheel::{
    collector::OwnedGc,
    diff::{Diff, Patch, RealtimeClone},
};

#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub struct AudionimbusSimulationOutputs {
    pub direct: AudionimbusDirectEffectParams,
    pub reflections: AudionimbusReflectionEffectParams,
}

impl From<audionimbus::SimulationOutputs> for AudionimbusSimulationOutputs {
    fn from(outputs: audionimbus::SimulationOutputs) -> Self {
        Self {
            direct: outputs.direct().into(),
            reflections: outputs.reflections().into(),
        }
    }
}

/// Parameters for applying a reflection effect to an audio buffer.
#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub struct AudionimbusReflectionEffectParams {
    /// Type of reflection effect algorithm to use.
    pub reflection_effect_type: AudionimbusReflectionEffectType,

    /// The impulse response.
    #[diff(skip)]
    pub impulse_response: OwnedGc<ReflectionEffectIR>,

    /// 3-band reverb decay times (RT60).
    pub reverb_times: [f32; 3],

    /// 3-band EQ coefficients applied to the parametric part to ensure smooth transition.
    pub equalizer: AudionimbusEqualizer<3>,

    /// Samples after which parametric part starts.
    pub delay: u32,

    /// Number of IR channels to process.
    /// May be less than the number of channels specified when creating the effect, in which case CPU usage will be reduced.
    pub num_channels: u32,

    /// Number of IR samples per channel to process.
    /// May be less than the number of samples specified when creating the effect, in which case CPU usage will be reduced.
    pub impulse_response_size: u32,

    /// The TrueAudio Next device to use for convolution processing.
    //pub true_audio_next_device: TrueAudioNextDevice,

    /// The TrueAudio Next slot index to use for convolution processing.
    /// The slot identifies the IR to use.
    pub true_audio_next_slot: u32,
}

#[derive(Debug, Clone, RealtimeClone, PartialEq)]
pub struct ReflectionEffectIR(pub audionimbus_sys::IPLReflectionEffectIR);

unsafe impl Send for ReflectionEffectIR {}

/// Type of reflection effect algorithm to use.
#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub enum AudionimbusReflectionEffectType {
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

/// Parameters for applying a direct effect to an audio buffer.
#[derive(Debug, Clone, RealtimeClone, Default, Diff, Patch, PartialEq)]
pub struct AudionimbusDirectEffectParams {
    /// Optional distance attenuation, with a value between 0.0 and 1.0.
    pub distance_attenuation: Option<f32>,

    /// Optional air absorption.
    pub air_absorption: Option<AudionimbusEqualizer<3>>,

    /// Optional directivity term, with a value between 0.0 and 1.0.
    pub directivity: Option<f32>,

    /// Optional occlusion factor, with a value between 0.0 and 1.0.
    pub occlusion: Option<f32>,

    /// Optional transmission.
    pub transmission: Option<AudionimbusTransmission>,
}

#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub enum AudionimbusTransmission {
    /// Frequency-independent transmission.
    FrequencyIndependent(AudionimbusEqualizer<3>),

    /// Frequency-dependent transmission.
    FrequencyDependent(AudionimbusEqualizer<3>),
}

#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub struct AudionimbusEqualizer<const N: usize>(pub [f32; N]);

#[derive(Debug, Clone, Diff, Patch)]
pub struct AudionimbusAudioSettings {
    pub sampling_rate: u32,
    pub frame_size: u32,
}

impl Default for AudionimbusAudioSettings {
    fn default() -> Self {
        audionimbus::AudioSettings::default().into()
    }
}

impl From<AudionimbusAudioSettings> for audionimbus::AudioSettings {
    fn from(settings: AudionimbusAudioSettings) -> Self {
        Self {
            sampling_rate: settings.sampling_rate as usize,
            frame_size: settings.frame_size as usize,
        }
    }
}

impl From<audionimbus::AudioSettings> for AudionimbusAudioSettings {
    fn from(settings: audionimbus::AudioSettings) -> Self {
        Self {
            sampling_rate: settings.sampling_rate as u32,
            frame_size: settings.frame_size as u32,
        }
    }
}
