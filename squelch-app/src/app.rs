//! Squelch egui application — squad setup UI backed by the async Backend.

use eframe::CreationContext;
use egui::{Align, Color32, FontId, Layout, RichText, Vec2};
use squelch_audio::PttState;

use crate::{
    backend::{Backend, BackendCmd},
    config::AppConfig,
};

// ── Screen state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Login,
    SquadSetup,
    Running,
}

// ── App ───────────────────────────────────────────────────────────────────

pub struct SquelchApp {
    screen: Screen,
    ptt: PttState,
    cfg: AppConfig,
    backend: Backend,

    // Login inputs
    homeserver_input: String,
    username_input: String,
    password_input: String,

    // Squad setup inputs
    room_id_input: String,
    ptt_key_input: String,
}

impl SquelchApp {
    pub fn new(_cc: &CreationContext<'_>, ptt: PttState, cfg: AppConfig, backend: Backend) -> Self {
        let homeserver_input = cfg.homeserver.clone();
        let username_input = cfg.username.clone();
        let ptt_key_input = cfg.ptt_key.clone().unwrap_or_else(|| "CapsLock".into());

        Self {
            screen: Screen::Login,
            ptt,
            cfg,
            backend,
            homeserver_input,
            username_input,
            password_input: String::new(),
            room_id_input: String::new(),
            ptt_key_input,
        }
    }

    // ── Screens ───────────────────────────────────────────────────────────

    fn show_login(&mut self, ui: &mut egui::Ui, st: &crate::backend::BackendState) {
        ui.vertical_centered(|ui| {
            ui.add_space(16.0);
            ui.label(
                RichText::new("🎙 Squelch")
                    .font(FontId::proportional(28.0))
                    .strong(),
            );
            ui.label(
                RichText::new("Tactical voice communication for gaming squads")
                    .color(Color32::GRAY),
            );
            ui.add_space(24.0);

            egui::Grid::new("login_grid")
                .num_columns(2)
                .spacing([12.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Homeserver");
                    ui.text_edit_singleline(&mut self.homeserver_input);
                    ui.end_row();

                    ui.label("Username");
                    ui.text_edit_singleline(&mut self.username_input);
                    ui.end_row();

                    ui.label("Password");
                    ui.add(egui::TextEdit::singleline(&mut self.password_input).password(true));
                    ui.end_row();
                });

            ui.add_space(12.0);

            // Show async status / error
            if let Some(err) = &st.error {
                ui.label(RichText::new(err).color(Color32::RED));
                ui.add_space(6.0);
            } else if st.status != "Not connected" {
                ui.label(RichText::new(&st.status).color(Color32::GRAY));
                ui.add_space(6.0);
            }

            // Transition to squad setup when logged in
            if st.logged_in && self.screen == Screen::Login {
                self.screen = Screen::SquadSetup;
            }

            let logging_in = !st.logged_in && st.status.contains("Logging");
            ui.add_enabled_ui(!logging_in, |ui| {
                if ui
                    .add_sized([180.0, 36.0], egui::Button::new("Sign in"))
                    .clicked()
                {
                    self.do_login();
                }
            });

            ui.add_space(8.0);
            ui.label(
                RichText::new("Your password is never stored.")
                    .color(Color32::GRAY)
                    .small(),
            );
        });
    }

    fn show_squad_setup(&mut self, ui: &mut egui::Ui, st: &crate::backend::BackendState) {
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            if let Some(uid) = &st.user_id {
                ui.label(RichText::new(uid).color(Color32::GRAY).small());
            }
            ui.label(
                RichText::new("Squad Setup")
                    .font(FontId::proportional(22.0))
                    .strong(),
            );
            ui.add_space(16.0);

            egui::Grid::new("squad_grid")
                .num_columns(2)
                .spacing([12.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Room ID");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.room_id_input)
                            .hint_text("leave blank to create a new room"),
                    );
                    ui.end_row();

                    ui.label("PTT Key");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ptt_key_input)
                            .hint_text("e.g. CapsLock, F1, ctrl+F1"),
                    );
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Duo: you and your partner hear each other always.\n\
                     Leader Net: hold PTT to reach the other duo's leader.",
                )
                .color(Color32::GRAY)
                .small(),
            );
            ui.add_space(12.0);

            if let Some(err) = &st.error {
                ui.label(RichText::new(err).color(Color32::RED));
                ui.add_space(6.0);
            } else if !st.status.is_empty() {
                ui.label(RichText::new(&st.status).color(Color32::GRAY));
                ui.add_space(6.0);
            }

            // Transition to running when room is active
            if st.room_id.is_some() && self.screen == Screen::SquadSetup {
                self.screen = Screen::Running;
            }

            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                if ui
                    .add_sized([130.0, 36.0], egui::Button::new("← Sign out"))
                    .clicked()
                {
                    self.backend.send(BackendCmd::Logout);
                    self.screen = Screen::Login;
                    self.password_input.clear();
                }
                ui.add_space(8.0);
                let btn_label = if self.room_id_input.trim().is_empty() {
                    "Create Squad"
                } else {
                    "Join Squad"
                };
                if ui
                    .add_sized([160.0, 36.0], egui::Button::new(btn_label))
                    .clicked()
                {
                    self.do_start_squad();
                }
            });
        });
    }

    fn show_running(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        st: &crate::backend::BackendState,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(16.0);
            ui.label(
                RichText::new("✓ Squad Active")
                    .font(FontId::proportional(22.0))
                    .color(Color32::GREEN)
                    .strong(),
            );
            ui.add_space(8.0);

            // Room ID — copyable
            if let Some(room_id) = &st.room_id {
                ui.label(
                    RichText::new("Share this Room ID with your squad:")
                        .color(Color32::GRAY)
                        .small(),
                );
                ui.add_space(4.0);
                let mut room_id_copy = room_id.clone();
                ui.add(
                    egui::TextEdit::singleline(&mut room_id_copy)
                        .desired_width(360.0)
                        .interactive(false),
                );
                if ui.small_button("📋 Copy").clicked() {
                    ctx.copy_text(room_id.clone());
                }
            }

            ui.add_space(12.0);

            // PTT indicator
            let ptt_on = self.ptt.is_active();
            let (label, color) = if ptt_on {
                ("● LEADER NET — TRANSMITTING", Color32::RED)
            } else {
                ("○ Leader Net standby", Color32::DARK_GRAY)
            };
            ui.label(RichText::new(label).color(color).strong());

            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "PTT key: {}",
                    self.cfg.ptt_key.as_deref().unwrap_or("CapsLock")
                ))
                .color(Color32::GRAY)
                .small(),
            );

            // Audio status
            ui.add_space(8.0);
            if st.audio_active {
                ui.label(
                    RichText::new("🎙 Microphone active")
                        .color(Color32::GREEN)
                        .small(),
                );
            } else {
                ui.label(
                    RichText::new("⚠ No audio device")
                        .color(Color32::YELLOW)
                        .small(),
                );
            }

            // Status / error
            ui.add_space(8.0);
            if let Some(err) = &st.error {
                ui.label(RichText::new(err).color(Color32::RED).small());
            } else {
                ui.label(RichText::new(&st.status).color(Color32::GRAY).small());
            }

            // If disbanded remotely, go back to setup
            if st.room_id.is_none() && self.screen == Screen::Running {
                self.screen = Screen::SquadSetup;
            }

            ui.add_space(20.0);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                if ui
                    .add_sized([150.0, 36.0], egui::Button::new("Leave Session"))
                    .clicked()
                {
                    self.backend.send(BackendCmd::LeaveSession);
                    self.screen = Screen::SquadSetup;
                }
                // Disband is shown separately — destructive action
                ui.add_space(8.0);
                let disband_btn =
                    egui::Button::new(RichText::new("Disband Squad").color(Color32::RED));
                if ui.add_sized([150.0, 36.0], disband_btn).clicked() {
                    self.backend.send(BackendCmd::DisbandSquad);
                    self.screen = Screen::SquadSetup;
                    self.room_id_input.clear();
                }
            });
        });
    }

    // ── Actions ───────────────────────────────────────────────────────────

    fn do_login(&mut self) {
        if self.homeserver_input.trim().is_empty()
            || self.username_input.trim().is_empty()
            || self.password_input.is_empty()
        {
            return;
        }

        self.cfg.homeserver = self.homeserver_input.trim().to_owned();
        self.cfg.username = self.username_input.trim().to_owned();
        let _ = self.cfg.save();

        self.backend.send(BackendCmd::Login {
            homeserver: self.cfg.homeserver.clone(),
            username: self.cfg.username.clone(),
            password: self.password_input.clone(),
        });
    }

    fn do_start_squad(&mut self) {
        if self.ptt_key_input.trim().is_empty() {
            return;
        }
        self.cfg.ptt_key = Some(self.ptt_key_input.trim().to_owned());
        let _ = self.cfg.save();

        if self.room_id_input.trim().is_empty() {
            self.backend.send(BackendCmd::CreateRoom {
                name: format!("Squelch Squad — {}", self.cfg.username),
            });
        } else {
            self.backend.send(BackendCmd::JoinRoom {
                room_id: self.room_id_input.trim().to_owned(),
            });
        }
    }
}

// ── eframe::App ───────────────────────────────────────────────────────────

impl eframe::App for SquelchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Always repaint while running (PTT indicator + async status updates)
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        let st = self.backend.snapshot();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_size(Vec2::new(460.0, 540.0));
            match self.screen.clone() {
                Screen::Login => self.show_login(ui, &st),
                Screen::SquadSetup => self.show_squad_setup(ui, &st),
                Screen::Running => self.show_running(ui, ctx, &st),
            }
        });
    }
}
