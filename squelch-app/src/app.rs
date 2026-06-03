//! Squelch egui application — squad setup UI.
//!
//! # Screen flow
//!
//! ```
//! Login ──(credentials ok)──→ Squad ──(setup complete)──→ [tray / running]
//!                               │
//!                         ← Back (logout)
//! ```
//!
//! The window shows only during setup. Once the squad is configured and
//! audio is running the user dismisses the window. A tray icon (post-MVP)
//! will allow returning to the setup screen.

use eframe::CreationContext;
use egui::{Align, Color32, FontId, Layout, RichText, Vec2};
use squelch_audio::PttState;
use tracing::info;

use crate::config::AppConfig;

// ── Screen state ──────────────────────────────────────────────────────────

/// Which screen is currently shown.
#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Login,
    SquadSetup,
    Running,
}

// ── App ───────────────────────────────────────────────────────────────────

/// The main egui application state.
pub struct SquelchApp {
    screen: Screen,
    ptt: PttState,
    cfg: AppConfig,

    // Login screen fields
    homeserver_input: String,
    username_input: String,
    password_input: String,
    login_error: Option<String>,

    // Squad setup fields
    room_id_input: String,
    ptt_key_input: String,
    squad_error: Option<String>,

    // Runtime status
    status_msg: Option<String>,
}

impl SquelchApp {
    pub fn new(_cc: &CreationContext<'_>, ptt: PttState, cfg: AppConfig) -> Self {
        let homeserver_input = cfg.homeserver.clone();
        let username_input = cfg.username.clone();
        let ptt_key_input = cfg.ptt_key.clone().unwrap_or_else(|| "CapsLock".into());

        Self {
            screen: Screen::Login,
            ptt,
            cfg,
            homeserver_input,
            username_input,
            password_input: String::new(),
            login_error: None,
            room_id_input: String::new(),
            ptt_key_input,
            squad_error: None,
            status_msg: None,
        }
    }

    // ── Screens ───────────────────────────────────────────────────────────

    fn show_login(&mut self, ui: &mut egui::Ui) {
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
                    let pwd = egui::TextEdit::singleline(&mut self.password_input).password(true);
                    ui.add(pwd);
                    ui.end_row();
                });

            ui.add_space(16.0);

            if let Some(err) = &self.login_error {
                ui.label(RichText::new(err).color(Color32::RED));
                ui.add_space(8.0);
            }

            if ui
                .add_sized([180.0, 36.0], egui::Button::new("Sign in"))
                .clicked()
            {
                self.do_login();
            }

            ui.add_space(8.0);
            ui.label(RichText::new("Your password never leaves your device.").color(Color32::GRAY));
        });
    }

    fn show_squad_setup(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
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
                            .hint_text("!abc123:matrix.org  or  leave blank to create"),
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
                     Leader Net: press PTT to reach the other duo's leader.",
                )
                .color(Color32::GRAY),
            );
            ui.add_space(16.0);

            if let Some(err) = &self.squad_error {
                ui.label(RichText::new(err).color(Color32::RED));
                ui.add_space(8.0);
            }

            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                if ui
                    .add_sized([140.0, 36.0], egui::Button::new("← Back"))
                    .clicked()
                {
                    self.screen = Screen::Login;
                }
                ui.add_space(8.0);
                if ui
                    .add_sized([180.0, 36.0], egui::Button::new("Start Squad"))
                    .clicked()
                {
                    self.do_start_squad();
                }
            });
        });
    }

    fn show_running(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.label(
                RichText::new("✓ Squad Active")
                    .font(FontId::proportional(24.0))
                    .color(Color32::GREEN)
                    .strong(),
            );
            ui.add_space(12.0);

            let ptt_on = self.ptt.is_active();
            let ptt_label = if ptt_on {
                RichText::new("● PTT ON").color(Color32::RED).strong()
            } else {
                RichText::new("○ PTT OFF").color(Color32::GRAY)
            };
            ui.label(ptt_label);

            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "PTT key: {}",
                    self.cfg.ptt_key.as_deref().unwrap_or("CapsLock")
                ))
                .color(Color32::GRAY),
            );

            if let Some(msg) = &self.status_msg {
                ui.add_space(8.0);
                ui.label(RichText::new(msg).color(Color32::GRAY));
            }

            ui.add_space(24.0);
            if ui
                .add_sized([160.0, 36.0], egui::Button::new("Leave Squad"))
                .clicked()
            {
                self.do_leave_squad();
            }
        });
    }

    // ── Actions ───────────────────────────────────────────────────────────

    fn do_login(&mut self) {
        self.login_error = None;

        if self.homeserver_input.trim().is_empty() {
            self.login_error = Some("Homeserver URL is required.".into());
            return;
        }
        if self.username_input.trim().is_empty() {
            self.login_error = Some("Username is required.".into());
            return;
        }
        if self.password_input.is_empty() {
            self.login_error = Some("Password is required.".into());
            return;
        }

        // Save homeserver + username (not password) to config
        self.cfg.homeserver = self.homeserver_input.trim().to_owned();
        self.cfg.username = self.username_input.trim().to_owned();
        let _ = self.cfg.save();

        info!(
            homeserver = %self.cfg.homeserver,
            username   = %self.cfg.username,
            "credentials entered — proceeding to squad setup"
        );

        // Phase 4: actual Matrix login is async — wired in post-MVP integration.
        // For now we transition to the squad setup screen immediately.
        self.screen = Screen::SquadSetup;
    }

    fn do_start_squad(&mut self) {
        self.squad_error = None;

        if self.ptt_key_input.trim().is_empty() {
            self.squad_error = Some("PTT key is required.".into());
            return;
        }

        self.cfg.ptt_key = Some(self.ptt_key_input.trim().to_owned());
        let _ = self.cfg.save();

        info!(
            room_id = %self.room_id_input,
            ptt_key = %self.ptt_key_input,
            "squad started"
        );

        self.status_msg = Some(format!(
            "Logged in as {}@{}",
            self.cfg.username,
            self.cfg
                .homeserver
                .trim_start_matches("https://")
                .trim_start_matches("http://")
        ));

        self.screen = Screen::Running;
    }

    fn do_leave_squad(&mut self) {
        info!("leaving squad");
        self.screen = Screen::Login;
        self.status_msg = None;
        self.room_id_input.clear();
    }
}

// ── eframe::App ───────────────────────────────────────────────────────────

impl eframe::App for SquelchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Repaint continuously while running so the PTT indicator stays live
        if self.screen == Screen::Running {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_size(Vec2::new(460.0, 520.0));
            match self.screen.clone() {
                Screen::Login => self.show_login(ui),
                Screen::SquadSetup => self.show_squad_setup(ui),
                Screen::Running => self.show_running(ui),
            }
        });
    }
}
