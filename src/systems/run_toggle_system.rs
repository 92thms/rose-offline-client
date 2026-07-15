use bevy::prelude::{Input, KeyCode, Res};
use bevy_egui::EguiContexts;

use rose_game_common::messages::client::ClientMessage;

use crate::resources::GameConnection;

/// Pressing Tab toggles between walk and run, matching the in-game help
/// text (walk animations already exist, just weren't wired to a key).
pub fn run_toggle_system(
    mut egui_context: EguiContexts,
    keyboard: Res<Input<KeyCode>>,
    game_connection: Option<Res<GameConnection>>,
) {
    if egui_context.ctx_mut().wants_keyboard_input() {
        return;
    }

    if !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }

    if let Some(game_connection) = game_connection.as_ref() {
        game_connection
            .client_message_tx
            .send(ClientMessage::RunToggle)
            .ok();
    }
}
