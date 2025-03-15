use bevy::prelude::*;
use bevy::utils::Duration;
use rodio::{OutputStream, Sink, Source};

const FRAME_SIZE: usize = 1024;
const SAMPLING_RATE: usize = 48000;
const NUM_CHANNELS: usize = 2;

#[derive(Resource)]
pub struct Audio {
    pub context: audionimbus::Context,
    pub settings: audionimbus::AudioSettings,
    pub hrtf: audionimbus::Hrtf,
    pub binaural_effect: audionimbus::BinauralEffect,
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

pub struct Plugin;

impl Plugin {
    fn process_frame(
        time: Res<Time>,
        mut audio: ResMut<Audio>,
        mut sine_wave_params: ResMut<SineWaveParams>,
    ) {
        audio.timer.tick(time.delta());

        for _ in 0..audio.timer.times_finished_this_tick() {
            let phase_increment =
                2.0 * std::f32::consts::PI * sine_wave_params.frequency / SAMPLING_RATE as f32;
            let delta_time = FRAME_SIZE as f32 / SAMPLING_RATE as f32; // Duration of a frame, in seconds.
            let speed = 5.0; // Speed of the sound source, in m/s.
            let distance_traveled = speed * delta_time; // Distance traveled over a frame.
            let radius = 1.0; // Radius of the sound source's circular path, in meters.

            let sine_wave: Vec<audionimbus::Sample> = (0..FRAME_SIZE)
                .map(|_| {
                    let sample = sine_wave_params.amplitude * sine_wave_params.phase.sin();
                    sine_wave_params.phase =
                        (sine_wave_params.phase + phase_increment) % (2.0 * std::f32::consts::PI);
                    sample
                })
                .collect();

            let input_buffer = audionimbus::AudioBuffer::try_with_data(&sine_wave).unwrap();

            let mut staging_container = vec![0.0; FRAME_SIZE * NUM_CHANNELS];
            let staging_buffer = audionimbus::AudioBuffer::try_with_data_and_settings(
                &mut staging_container,
                &audionimbus::AudioBufferSettings {
                    num_channels: Some(NUM_CHANNELS),
                    ..Default::default()
                },
            )
            .unwrap();

            // Update the direction of the sound source.
            sine_wave_params.angle = (sine_wave_params.angle + distance_traveled / radius)
                .rem_euclid(std::f32::consts::TAU);
            let x = sine_wave_params.angle.cos() * radius;
            let z = sine_wave_params.angle.sin() * radius;
            let direction = audionimbus::Direction::new(x, 0.0, z);

            let binaural_effect_params = audionimbus::BinauralEffectParams {
                direction,
                interpolation: audionimbus::HrtfInterpolation::Nearest,
                spatial_blend: 1.0,
                hrtf: &audio.hrtf,
                peak_delays: None,
            };
            let _effect_state = audio.binaural_effect.apply(
                &binaural_effect_params,
                &input_buffer,
                &staging_buffer,
            );

            let mut output = vec![0.0; FRAME_SIZE * NUM_CHANNELS];
            staging_buffer.interleave(&audio.context, &mut output);
            let source = AudioFrame::new(output, 2);

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

        let hrtf =
            audionimbus::Hrtf::try_new(&context, &settings, &audionimbus::HrtfSettings::default())
                .unwrap();

        let binaural_effect = audionimbus::BinauralEffect::try_new(
            &context,
            &settings,
            &audionimbus::BinauralEffectSettings { hrtf: &hrtf },
        )
        .unwrap();

        app.insert_resource(Audio {
            context,
            settings,
            hrtf,
            binaural_effect,
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

        app.add_systems(Update, Self::process_frame);
    }
}
