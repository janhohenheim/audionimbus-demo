use bevy::ecs::event::{Event, EventReader, EventWriter};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

#[derive(Event, Default, Debug)]
pub struct Delta(
    // Values must be between 0.0 and 1.0.
    pub bevy::math::Vec2,
);

pub struct Plugin;

impl Plugin {
    fn process_inputs(
        mut mouse_motion_event_reader: EventReader<MouseMotion>,
        mut input_event_writer: EventWriter<Delta>,
    ) {
        let mut delta = bevy::math::Vec2::ZERO;
        for event in mouse_motion_event_reader.read() {
            delta += event.delta;
        }

        input_event_writer.send(Delta(delta));
    }
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Delta>()
            .add_systems(Update, Self::process_inputs);
    }
}
