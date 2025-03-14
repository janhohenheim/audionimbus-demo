use crate::controls;
use bevy::ecs::event::{Event, EventWriter};
use bevy::input::{keyboard::KeyCode, mouse::MouseButton, ButtonInput};
use bevy::prelude::*;

#[derive(Event, Clone, Copy, Default)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
}

pub struct Plugin;

impl Plugin {
    fn process_inputs(
        controls: Res<controls::Controls>,
        keyboard_input: Res<ButtonInput<KeyCode>>,
        mouse_input: Res<ButtonInput<MouseButton>>,
        mut input_event_writer: EventWriter<Input>,
    ) {
        let mut forward = 0.0;

        match controls.move_forward {
            controls::ButtonInput::Keyboard(key_code) => {
                if keyboard_input.pressed(key_code) {
                    forward += 1.0;
                }
            }
            controls::ButtonInput::Mouse(mouse_button) => {
                if mouse_input.pressed(mouse_button) {
                    forward += 1.0;
                }
            }
        };

        match controls.move_backward {
            controls::ButtonInput::Keyboard(key_code) => {
                if keyboard_input.pressed(key_code) {
                    forward -= 1.0;
                }
            }
            controls::ButtonInput::Mouse(mouse_button) => {
                if mouse_input.pressed(mouse_button) {
                    forward -= 1.0;
                }
            }
        };

        let mut strafe = 0.0;

        match controls.strafe_right {
            controls::ButtonInput::Keyboard(key_code) => {
                if keyboard_input.pressed(key_code) {
                    strafe += 1.0;
                }
            }
            controls::ButtonInput::Mouse(mouse_button) => {
                if mouse_input.pressed(mouse_button) {
                    strafe += 1.0;
                }
            }
        };

        match controls.strafe_left {
            controls::ButtonInput::Keyboard(key_code) => {
                if keyboard_input.pressed(key_code) {
                    strafe -= 1.0;
                }
            }
            controls::ButtonInput::Mouse(mouse_button) => {
                if mouse_input.pressed(mouse_button) {
                    strafe -= 1.0;
                }
            }
        };

        input_event_writer.send(Input { forward, strafe });
    }
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Input>()
            .add_systems(Update, Self::process_inputs);
    }
}
