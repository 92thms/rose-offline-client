use std::path::Path;

use bevy::{
    input::Input,
    math::{EulerRot, Vec3},
    prelude::{
        Camera3d, Commands, Entity, KeyCode, Local, NextState, Query, Res, ResMut, Resource,
        Transform, With,
    },
};
use bevy_egui::{egui, EguiContexts};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::PlayerCharacter,
    resources::{AppState, DebugInspector, GameConnection, WorldConnection},
    save_config,
    systems::{FreeCamera, OrbitCamera},
    ui::UiStateWindows,
    Config, UserCheat,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DebugCameraType {
    Orbit,
    Free,
}

impl Default for DebugCameraType {
    fn default() -> Self {
        Self::Orbit
    }
}

#[derive(Default, Resource)]
pub struct UiStateDebugWindows {
    pub debug_ui_open: bool,

    pub camera_info_open: bool,
    pub client_entity_list_open: bool,
    pub command_viewer_open: bool,
    pub debug_render_open: bool,
    pub dialog_list_open: bool,
    pub effect_list_open: bool,
    pub item_list_open: bool,
    pub npc_list_open: bool,
    pub object_inspector_open: bool,
    pub physics_open: bool,
    pub skill_list_open: bool,
    pub zone_list_open: bool,
    pub zone_lighting_open: bool,
    pub zone_time_open: bool,
}

#[derive(Default)]
pub struct UiStateDebugMenu {
    selected_camera_type: DebugCameraType,
    cheats_manage_open: bool,
    new_cheat_name: String,
    new_cheat_command: String,
}

#[allow(clippy::too_many_arguments)]
pub fn ui_debug_menu_system(
    mut commands: Commands,
    mut egui_context: EguiContexts,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_state_debug_menu: Local<UiStateDebugMenu>,
    query_cameras: Query<(Entity, &Transform), With<Camera3d>>,
    query_player: Query<Entity, With<PlayerCharacter>>,
    game_connection: Option<Res<GameConnection>>,
    world_connection: Option<Res<WorldConnection>>,
    keyboard: Res<Input<KeyCode>>,
    mut debug_inspector: ResMut<DebugInspector>,
    mut app_state_next: ResMut<NextState<AppState>>,
    mut config: ResMut<Config>,
) {
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::D) {
        ui_state_debug_windows.debug_ui_open = !ui_state_debug_windows.debug_ui_open;
    }

    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    let ctx = egui_context.ctx_mut();
    egui::TopBottomPanel::top("ui_debug_menu").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            let player_entity = query_player.get_single().ok();

            ui.menu_button("App", |ui| {
                if ui.button("Model Viewer").clicked() {
                    app_state_next.set(AppState::ModelViewer);
                }

                if ui.button("Zone Viewer").clicked() {
                    app_state_next.set(AppState::ZoneViewer);
                }

                ui.separator();

                ui.add_enabled_ui(
                    world_connection.is_none() && game_connection.is_none(),
                    |ui| {
                        if ui.button("Game Login").clicked() {
                            app_state_next.set(AppState::GameLogin);
                        }
                    },
                );

                ui.add_enabled_ui(
                    world_connection.is_some() && game_connection.is_none(),
                    |ui| {
                        if ui.button("Game Character Select").clicked() {
                            app_state_next.set(AppState::GameCharacterSelect);
                        }
                    },
                );

                ui.add_enabled_ui(game_connection.is_some(), |ui| {
                    if ui.button("Game").clicked() {
                        app_state_next.set(AppState::Game);
                    }
                });

                ui.set_enabled(true);
            });

            ui.menu_button("Camera", |ui| {
                let previous_camera_type = ui_state_debug_menu.selected_camera_type;

                if player_entity.is_some() {
                    ui.selectable_value(
                        &mut ui_state_debug_menu.selected_camera_type,
                        DebugCameraType::Orbit,
                        "Orbit",
                    );
                }

                ui.selectable_value(
                    &mut ui_state_debug_menu.selected_camera_type,
                    DebugCameraType::Free,
                    "Free",
                );

                if ui_state_debug_menu.selected_camera_type != previous_camera_type {
                    for (camera_entity, camera_transform) in query_cameras.iter() {
                        match ui_state_debug_menu.selected_camera_type {
                            DebugCameraType::Orbit => {
                                if let Some(player_entity) = player_entity {
                                    commands
                                        .entity(camera_entity)
                                        .remove::<FreeCamera>()
                                        .insert(OrbitCamera::new(
                                            player_entity,
                                            Vec3::new(0.0, 1.7, 0.0),
                                            17.0,
                                        ));
                                }
                            }
                            DebugCameraType::Free => {
                                let (yaw, pitch, _roll) =
                                    camera_transform.rotation.to_euler(EulerRot::YXZ);

                                commands
                                    .entity(camera_entity)
                                    .remove::<OrbitCamera>()
                                    .insert(FreeCamera::new(
                                        camera_transform.translation,
                                        yaw.to_degrees(),
                                        pitch.to_degrees(),
                                    ));
                            }
                        }
                    }
                }
            });

            ui.menu_button("Cheats", |ui| {
                if ui.button("Move Speed 4000").clicked() {
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::Chat {
                                text: "/speed 4000".to_string(),
                            })
                            .ok();
                    }
                }

                if !config.cheats.is_empty() {
                    ui.separator();

                    for cheat in config.cheats.iter() {
                        if ui.button(&cheat.name).clicked() {
                            if let Some(game_connection) = game_connection.as_ref() {
                                game_connection
                                    .client_message_tx
                                    .send(ClientMessage::Chat {
                                        text: cheat.command.clone(),
                                    })
                                    .ok();
                            }
                        }
                    }
                }

                ui.separator();
                if ui.button("Add/remove...").clicked() {
                    ui_state_debug_menu.cheats_manage_open = true;
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(
                    &mut ui_state_debug_windows.command_viewer_open,
                    "Command Viewer",
                );
                ui.checkbox(
                    &mut ui_state_debug_windows.debug_render_open,
                    "Debug Render",
                );
                ui.checkbox(&mut ui_state_debug_windows.dialog_list_open, "Dialog List");
                ui.checkbox(&mut ui_state_debug_windows.effect_list_open, "Effect List");
                ui.checkbox(&mut ui_state_debug_windows.item_list_open, "Item List");
                ui.checkbox(&mut ui_state_debug_windows.npc_list_open, "NPC List");
                ui.checkbox(&mut ui_state_debug_windows.skill_list_open, "Skill List");
                ui.checkbox(&mut ui_state_debug_windows.zone_list_open, "Zone List");
                ui.checkbox(
                    &mut ui_state_debug_windows.zone_lighting_open,
                    "Zone Lighting",
                );
                ui.checkbox(&mut ui_state_debug_windows.zone_time_open, "Zone Time");
                ui.checkbox(
                    &mut ui_state_debug_windows.client_entity_list_open,
                    "Client Entity List",
                );

                if ui
                    .checkbox(
                        &mut ui_state_debug_windows.object_inspector_open,
                        "Object Inspector",
                    )
                    .clicked()
                {
                    if ui_state_debug_windows.object_inspector_open {
                        debug_inspector.enable_picking = true;

                        if let Some(player_entity) = player_entity {
                            debug_inspector.entity = Some(player_entity);
                        }
                    } else {
                        debug_inspector.enable_picking = false;
                    }
                }

                ui.checkbox(&mut ui_state_debug_windows.camera_info_open, "Camera Info");
                ui.checkbox(&mut ui_state_debug_windows.physics_open, "Physics");
                ui.checkbox(&mut ui_state_windows.settings_open, "Settings");
            });
        });
    });

    let mut cheats_manage_open = ui_state_debug_menu.cheats_manage_open;
    egui::Window::new("Manage Cheats")
        .open(&mut cheats_manage_open)
        .resizable(false)
        .show(egui_context.ctx_mut(), |ui| {
            let mut save = false;

            ui.label("Add a new cheat: a name for the menu button, and the chat text it sends (e.g. \"/set con 5000\").");
            egui::Grid::new("add_cheat_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut ui_state_debug_menu.new_cheat_name);
                    ui.end_row();

                    ui.label("Command");
                    ui.text_edit_singleline(&mut ui_state_debug_menu.new_cheat_command);
                    ui.end_row();
                });

            let can_add = !ui_state_debug_menu.new_cheat_name.trim().is_empty()
                && !ui_state_debug_menu.new_cheat_command.trim().is_empty();
            if ui
                .add_enabled(can_add, egui::Button::new("Add"))
                .clicked()
            {
                config.cheats.push(UserCheat {
                    name: ui_state_debug_menu.new_cheat_name.trim().to_string(),
                    command: ui_state_debug_menu.new_cheat_command.trim().to_string(),
                });
                ui_state_debug_menu.new_cheat_name.clear();
                ui_state_debug_menu.new_cheat_command.clear();
                save = true;
            }

            ui.separator();

            let mut remove_index = None;
            for (index, cheat) in config.cheats.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{} ({})", cheat.name, cheat.command));
                    if ui.small_button("Remove").clicked() {
                        remove_index = Some(index);
                    }
                });
            }

            if let Some(remove_index) = remove_index {
                config.cheats.remove(remove_index);
                save = true;
            }

            if save {
                let path = config.filesystem.config_path.clone();
                save_config(&config, Path::new(&path));
            }
        });
    ui_state_debug_menu.cheats_manage_open = cheats_manage_open;
}
