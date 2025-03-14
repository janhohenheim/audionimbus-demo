use bevy::input::{keyboard::KeyCode, mouse::MouseButton};
use bevy::prelude::*;

#[derive(Resource, Debug)]
pub struct Controls {
    pub move_forward: ButtonInput,
    pub move_backward: ButtonInput,
    pub strafe_left: ButtonInput,
    pub strafe_right: ButtonInput,
}

impl Controls {
    pub fn new() -> Self {
        Controls {
            move_forward: ButtonInput::Keyboard(KeyCode::KeyW),
            move_backward: ButtonInput::Keyboard(KeyCode::KeyS),
            strafe_left: ButtonInput::Keyboard(KeyCode::KeyA),
            strafe_right: ButtonInput::Keyboard(KeyCode::KeyD),
        }
    }
}

#[derive(Debug)]
pub enum ButtonInput {
    Keyboard(KeyCode),
    Mouse(MouseButton),
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Controls::new());
    }
}
