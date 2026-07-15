#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;

use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
enum RunMode {
    Game,
    ModelViewer,
    ZoneViewer,
}

impl Default for RunMode {
    fn default() -> Self {
        RunMode::Game
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct LauncherConfig {
    client_path: String,
    data_idx: String,
    data_aruavfs_idx: String,
    data_titanvfs_idx: String,
    data_iroseph_idx: String,
    data_path: String,
    ip: String,
    port: String,
    username: String,
    password: String,
    auto_login: bool,
    server_id: String,
    channel_id: String,
    character_name: String,
    disable_vsync: bool,
    passthrough_terrain_textures: bool,
    disable_sound: bool,
    fullscreen: bool,
    width: String,
    height: String,
    run_mode: RunMode,
    zone_id: String,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            client_path: String::new(),
            data_idx: String::new(),
            data_aruavfs_idx: String::new(),
            data_titanvfs_idx: String::new(),
            data_iroseph_idx: String::new(),
            data_path: String::new(),
            ip: "127.0.0.1".to_string(),
            port: "29000".to_string(),
            username: String::new(),
            password: String::new(),
            auto_login: false,
            server_id: String::new(),
            channel_id: String::new(),
            character_name: String::new(),
            disable_vsync: false,
            passthrough_terrain_textures: false,
            disable_sound: false,
            fullscreen: false,
            width: "1920".to_string(),
            height: "1080".to_string(),
            run_mode: RunMode::Game,
            zone_id: String::new(),
        }
    }
}

fn config_file_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
        .join("launcher_config.toml")
}

fn load_config() -> LauncherConfig {
    let path = config_file_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => {
            let mut config = LauncherConfig::default();
            autodetect_paths(&mut config);
            config
        }
    }
}

fn save_config(config: &LauncherConfig) -> std::io::Result<()> {
    let contents = toml::to_string_pretty(config).unwrap_or_default();
    std::fs::write(config_file_path(), contents)
}

/// Strips the `\\?\` verbatim-path prefix Windows' `canonicalize()` adds,
/// purely so paths look normal when shown in the UI.
fn clean_path(path: PathBuf) -> String {
    let text = path.to_string_lossy().to_string();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}

/// Best-effort guess at the client exe, data.idx and loose 3Ddata override
/// folder location. Runs on first launch (no saved config yet) and can also
/// be re-triggered from the Options tab.
fn autodetect_paths(config: &mut LauncherConfig) {
    let mut search_dir = None;

    if let Ok(exe) = std::env::current_exe() {
        if let Some(launcher_dir) = exe.parent() {
            let candidates = [
                launcher_dir.join("rose-offline-client.exe"),
                launcher_dir.join("../../../target/release/rose-offline-client.exe"),
                launcher_dir.join("../../target/release/rose-offline-client.exe"),
            ];
            for candidate in candidates {
                if let Ok(canonical) = candidate.canonicalize() {
                    if canonical.is_file() {
                        config.client_path = clean_path(canonical.clone());
                        search_dir = canonical.parent().map(|p| p.to_path_buf());
                        break;
                    }
                }
            }
        }
    }

    let search_dir = search_dir.unwrap_or_else(|| PathBuf::from("."));

    let data_idx_candidate = search_dir.join("data.idx");
    if data_idx_candidate.is_file() {
        if let Ok(canonical) = data_idx_candidate.canonicalize() {
            config.data_idx = clean_path(canonical);
        }
    }

    // The original client ships a handful of UI dialog XMLs (e.g. the login
    // screen) as loose files next to data.idx rather than inside the VFS
    // archives - without --data-path pointing at the folder that contains
    // this "3Ddata" directory, those dialogs silently fail to load.
    let data_path_candidate = search_dir.join("3Ddata");
    if data_path_candidate.is_dir() {
        if let Ok(canonical) = search_dir.canonicalize() {
            config.data_path = clean_path(canonical);
        }
    }
}

#[derive(PartialEq)]
enum Tab {
    Start,
    Options,
}

struct LauncherApp {
    config: LauncherConfig,
    tab: Tab,
    status: String,
}

impl LauncherApp {
    fn new() -> Self {
        Self {
            config: load_config(),
            tab: Tab::Start,
            status: String::new(),
        }
    }

    fn build_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        let c = &self.config;

        macro_rules! push_opt {
            ($flag:expr, $value:expr) => {
                if !$value.trim().is_empty() {
                    args.push($flag.to_string());
                    args.push($value.trim().to_string());
                }
            };
        }

        push_opt!("--data-idx", c.data_idx);
        push_opt!("--data-aruavfs-idx", c.data_aruavfs_idx);
        push_opt!("--data-titanvfs-idx", c.data_titanvfs_idx);
        push_opt!("--data-iroseph-idx", c.data_iroseph_idx);
        push_opt!("--data-path", c.data_path);
        push_opt!("--ip", c.ip);
        push_opt!("--port", c.port);
        push_opt!("--username", c.username);
        push_opt!("--password", c.password);
        push_opt!("--server-id", c.server_id);
        push_opt!("--channel-id", c.channel_id);
        push_opt!("--character-name", c.character_name);

        if c.auto_login {
            args.push("--auto-login".to_string());
        }
        if c.disable_vsync {
            args.push("--disable-vsync".to_string());
        }
        if c.passthrough_terrain_textures {
            args.push("--passthrough-terrain-textures".to_string());
        }
        if c.disable_sound {
            args.push("--disable-sound".to_string());
        }

        if c.fullscreen {
            args.push("--fullscreen".to_string());
        } else {
            push_opt!("--width", c.width);
            push_opt!("--height", c.height);
        }

        match c.run_mode {
            RunMode::Game => {}
            RunMode::ModelViewer => args.push("--model-viewer".to_string()),
            RunMode::ZoneViewer => {
                args.push("--zone-viewer".to_string());
                push_opt!("--zone", c.zone_id);
            }
        }

        args
    }

    fn launch(&mut self) {
        if let Err(error) = save_config(&self.config) {
            self.status = format!("Konnte Einstellungen nicht speichern: {error}");
            return;
        }

        let client_path = self.config.client_path.trim();
        if client_path.is_empty() {
            self.status =
                "Bitte im Optionen-Tab den Pfad zu rose-offline-client.exe setzen.".to_string();
            return;
        }

        let exe = PathBuf::from(client_path);
        if !exe.is_file() {
            self.status = format!("Client-Exe nicht gefunden: {}", exe.display());
            return;
        }

        let args = self.build_args();
        let mut command = Command::new(&exe);
        command.args(&args);
        if let Some(dir) = exe.parent() {
            command.current_dir(dir);
        }

        match command.spawn() {
            Ok(_) => {
                self.status = format!("Gestartet: {} {}", exe.display(), args.join(" "));
            }
            Err(error) => {
                self.status = format!("Start fehlgeschlagen: {error}");
            }
        }
    }

    fn browse_file(target: &mut String, filter_name: &str, filter_exts: &[&str]) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(filter_name, filter_exts)
            .pick_file()
        {
            *target = path.to_string_lossy().to_string();
        }
    }

    fn browse_folder(target: &mut String) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            *target = path.to_string_lossy().to_string();
        }
    }

    fn ui_start(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.heading("ROSE Offline Client");
        ui.add_space(12.0);

        egui::Grid::new("start_grid")
            .num_columns(2)
            .spacing([8.0, 8.0])
            .show(ui, |ui| {
                ui.label("Benutzername:");
                ui.text_edit_singleline(&mut self.config.username);
                ui.end_row();

                ui.label("Passwort:");
                ui.add(egui::TextEdit::singleline(&mut self.config.password).password(true));
                ui.end_row();

                ui.label("Charaktername:");
                ui.text_edit_singleline(&mut self.config.character_name);
                ui.end_row();

                ui.label("Automatisch einloggen:");
                ui.checkbox(&mut self.config.auto_login, "");
                ui.end_row();
            });

        ui.add_space(16.0);

        if ui
            .add_sized([160.0, 40.0], egui::Button::new("Start"))
            .clicked()
        {
            self.launch();
        }

        ui.add_space(12.0);
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
    }

    fn ui_options(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.heading("Pfade");
            ui.add_space(4.0);

            if ui.button("Auto-erkennen").clicked() {
                autodetect_paths(&mut self.config);
            }
            ui.label(
                egui::RichText::new(
                    "\"Entpackte Daten\" muss auf den Ordner zeigen, der den 3Ddata-Unterordner \
                     enthält (nicht auf 3Ddata selbst) - sonst fehlen Dialoge wie der Login-Screen.",
                )
                .small()
                .weak(),
            );
            ui.add_space(4.0);

            egui::Grid::new("paths_grid")
                .num_columns(3)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Client (rose-offline-client.exe):");
                    ui.text_edit_singleline(&mut self.config.client_path);
                    if ui.button("Durchsuchen...").clicked() {
                        Self::browse_file(
                            &mut self.config.client_path,
                            "Programm",
                            &["exe"],
                        );
                    }
                    ui.end_row();

                    ui.label("data.idx (129en irose):");
                    ui.text_edit_singleline(&mut self.config.data_idx);
                    if ui.button("Durchsuchen...").clicked() {
                        Self::browse_file(&mut self.config.data_idx, "data.idx", &["idx"]);
                    }
                    ui.end_row();

                    ui.label("Entpackte Daten (optional):");
                    ui.text_edit_singleline(&mut self.config.data_path);
                    if ui.button("Ordner wählen...").clicked() {
                        Self::browse_folder(&mut self.config.data_path);
                    }
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.collapsing("Erweitert (aruarose / titanrose / iRosePH)", |ui| {
                egui::Grid::new("advanced_paths_grid")
                    .num_columns(3)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("aruarose data.idx:");
                        ui.text_edit_singleline(&mut self.config.data_aruavfs_idx);
                        if ui.button("Durchsuchen...").clicked() {
                            Self::browse_file(
                                &mut self.config.data_aruavfs_idx,
                                "data.idx",
                                &["idx"],
                            );
                        }
                        ui.end_row();

                        ui.label("titanrose data.idx:");
                        ui.text_edit_singleline(&mut self.config.data_titanvfs_idx);
                        if ui.button("Durchsuchen...").clicked() {
                            Self::browse_file(
                                &mut self.config.data_titanvfs_idx,
                                "data.idx",
                                &["idx"],
                            );
                        }
                        ui.end_row();

                        ui.label("iRosePH data.idx:");
                        ui.text_edit_singleline(&mut self.config.data_iroseph_idx);
                        if ui.button("Durchsuchen...").clicked() {
                            Self::browse_file(
                                &mut self.config.data_iroseph_idx,
                                "data.idx",
                                &["idx"],
                            );
                        }
                        ui.end_row();
                    });
            });

            ui.add_space(16.0);
            ui.heading("Server");
            ui.add_space(4.0);

            egui::Grid::new("server_grid")
                .num_columns(2)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Server IP:");
                    ui.text_edit_singleline(&mut self.config.ip);
                    ui.end_row();

                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.config.port);
                    ui.end_row();

                    ui.label("Server-ID (Auto-Login):");
                    ui.text_edit_singleline(&mut self.config.server_id);
                    ui.end_row();

                    ui.label("Channel-ID (Auto-Login):");
                    ui.text_edit_singleline(&mut self.config.channel_id);
                    ui.end_row();
                });

            ui.add_space(16.0);
            ui.heading("Grafik / Sound");
            ui.add_space(4.0);

            ui.checkbox(&mut self.config.fullscreen, "Vollbild");
            ui.add_enabled_ui(!self.config.fullscreen, |ui| {
                egui::Grid::new("resolution_grid")
                    .num_columns(2)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Breite:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.width)
                                .desired_width(80.0),
                        );
                        ui.end_row();

                        ui.label("Höhe:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.height)
                                .desired_width(80.0),
                        );
                        ui.end_row();
                    });
            });

            ui.add_space(8.0);
            ui.checkbox(&mut self.config.disable_vsync, "V-Sync deaktivieren");
            ui.checkbox(
                &mut self.config.passthrough_terrain_textures,
                "Terrain-Texturen durchreichen (nur 129_129en Standard-Assets)",
            );
            ui.checkbox(&mut self.config.disable_sound, "Sound deaktivieren");

            ui.add_space(16.0);
            ui.heading("Modus");
            ui.add_space(4.0);
            ui.radio_value(&mut self.config.run_mode, RunMode::Game, "Spiel");
            ui.radio_value(
                &mut self.config.run_mode,
                RunMode::ModelViewer,
                "Model-Viewer",
            );
            ui.horizontal(|ui| {
                ui.radio_value(
                    &mut self.config.run_mode,
                    RunMode::ZoneViewer,
                    "Zone-Viewer",
                );
                if self.config.run_mode == RunMode::ZoneViewer {
                    ui.label("Zone-ID:");
                    ui.add(egui::TextEdit::singleline(&mut self.config.zone_id).desired_width(60.0));
                }
            });

            ui.add_space(20.0);
            if ui.button("Einstellungen speichern").clicked() {
                match save_config(&self.config) {
                    Ok(()) => self.status = "Einstellungen gespeichert.".to_string(),
                    Err(error) => self.status = format!("Speichern fehlgeschlagen: {error}"),
                }
            }
        });
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Start, "Start");
                ui.selectable_value(&mut self.tab, Tab::Options, "Optionen");
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Start => self.ui_start(ui),
            Tab::Options => self.ui_options(ui),
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = save_config(&self.config);
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([520.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "ROSE Offline Client Launcher",
        options,
        Box::new(|_cc| Box::new(LauncherApp::new())),
    )
}
