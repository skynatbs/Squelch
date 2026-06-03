//! Squelch egui application — squad setup UI backed by the async Backend.

use eframe::CreationContext;
use egui::{Align, Color32, FontId, Layout, RichText, Vec2};
use squelch_audio::PttState;

use crate::{
    backend::{Backend, BackendCmd},
    config::AppConfig,
};

// ── Squad draft ────────────────────────────────────────────────────────────

/// A squad being assembled in the TeamSetup screen.
#[derive(Debug, Clone, Default)]
pub struct SquadDraft {
    pub name: String,
    pub members: Vec<String>,
    pub leader: Option<String>,
}

// ── Screen state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Login,
    SquadSetup,
    TeamSetup,
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

    // Team setup state
    squads: Vec<SquadDraft>,
    new_squad_name: String,
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
            squads: Vec::new(),
            new_squad_name: String::new(),
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

            // Transition to TeamSetup (not Running) when room is active
            if st.room_id.is_some() && self.screen == Screen::SquadSetup {
                self.squads.clear();
                self.screen = Screen::TeamSetup;
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

    fn show_team_setup(&mut self, ui: &mut egui::Ui, st: &crate::backend::BackendState) {
        // Collect all members: own user_id + peers
        let mut all_members: Vec<String> = Vec::new();
        if let Some(uid) = &st.user_id {
            all_members.push(uid.clone());
        }
        for peer in &st.peers {
            if !all_members.contains(peer) {
                all_members.push(peer.clone());
            }
        }

        // Collect already-assigned members (across all squads)
        let assigned: Vec<String> = self
            .squads
            .iter()
            .flat_map(|s| s.members.iter().cloned())
            .collect();

        let unassigned: Vec<String> = all_members
            .iter()
            .filter(|m| !assigned.contains(m))
            .cloned()
            .collect();

        // ── Pending mutations accumulated during UI traversal ──
        // We use index-based mutations to avoid borrow conflicts.
        let mut create_squad: Option<String> = None;
        let mut remove_from_squad: Option<(usize, String)> = None;
        let mut set_leader: Option<(usize, String)> = None;
        let mut add_to_squad: Option<(String, usize)> = None; // (member, squad_idx)
        let mut go_running = false;
        let mut go_squad_setup = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);

            // ── Title & room info ──────────────────────────────────────────
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Team Setup")
                        .font(FontId::proportional(22.0))
                        .strong(),
                );
                ui.add_space(4.0);
                if let Some(room_id) = &st.room_id {
                    ui.label(RichText::new("Room ID:").color(Color32::GRAY).small());
                    let mut room_copy = room_id.clone();
                    ui.add(
                        egui::TextEdit::singleline(&mut room_copy)
                            .desired_width(360.0)
                            .interactive(false),
                    );
                }
            });

            ui.add_space(12.0);
            ui.separator();

            // ── Online members ─────────────────────────────────────────────
            ui.add_space(8.0);
            ui.label(RichText::new("Online Members").strong());
            ui.add_space(4.0);

            if all_members.is_empty() {
                ui.label(
                    RichText::new("No members online yet.")
                        .color(Color32::GRAY)
                        .small(),
                );
            } else {
                for member in &all_members {
                    let is_assigned = assigned.contains(member);
                    ui.horizontal(|ui| {
                        let label = if is_assigned {
                            RichText::new(format!("  {member}")).color(Color32::GRAY)
                        } else {
                            RichText::new(format!("  {member}"))
                        };
                        ui.label(label);

                        if !is_assigned {
                            for (idx, squad) in self.squads.iter().enumerate() {
                                let btn_text = format!("+ {}", squad.name);
                                if ui.small_button(btn_text).clicked() {
                                    add_to_squad = Some((member.clone(), idx));
                                }
                            }
                            if self.squads.is_empty() {
                                ui.label(
                                    RichText::new("(create a squad below)")
                                        .color(Color32::GRAY)
                                        .small(),
                                );
                            }
                        } else {
                            // Find which squad this member belongs to
                            let squad_name = self
                                .squads
                                .iter()
                                .find(|s| s.members.contains(member))
                                .map(|s| s.name.as_str())
                                .unwrap_or_default()
                                .to_owned();
                            ui.label(
                                RichText::new(format!("[{squad_name}]"))
                                    .color(Color32::GRAY)
                                    .small(),
                            );
                        }
                    });
                }
            }

            ui.add_space(12.0);
            ui.separator();

            // ── Squads section ─────────────────────────────────────────────
            ui.add_space(8.0);
            ui.label(RichText::new("Squads").strong());
            ui.add_space(4.0);

            if self.squads.is_empty() {
                ui.label(
                    RichText::new("No squads yet. Create one below.")
                        .color(Color32::GRAY)
                        .small(),
                );
            }

            for (squad_idx, squad) in self.squads.iter().enumerate() {
                let leader_name = squad.leader.clone().unwrap_or_default();

                egui::Frame::new()
                    .inner_margin(egui::Margin::same(6))
                    .stroke(egui::Stroke::new(1.0, Color32::DARK_GRAY))
                    .corner_radius(egui::CornerRadius::same(4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("■ {}", squad.name))
                                    .strong()
                                    .color(Color32::LIGHT_BLUE),
                            );
                            if !leader_name.is_empty() {
                                ui.label(
                                    RichText::new(format!("  ★ Leader: {leader_name}"))
                                        .color(Color32::GOLD)
                                        .small(),
                                );
                            }
                        });

                        if squad.members.is_empty() {
                            ui.label(
                                RichText::new("  (no members yet)")
                                    .color(Color32::GRAY)
                                    .small(),
                            );
                        } else {
                            for member in &squad.members {
                                ui.horizontal(|ui| {
                                    let is_leader =
                                        squad.leader.as_deref() == Some(member.as_str());
                                    let label = if is_leader {
                                        RichText::new(format!("  ★ {member}")).color(Color32::GOLD)
                                    } else {
                                        RichText::new(format!("    {member}"))
                                    };
                                    ui.label(label);

                                    // let-chains require nightly; keep as nested ifs on stable.
                                    #[allow(clippy::collapsible_if)]
                                    if !is_leader {
                                        if ui.small_button("★ Make Leader").clicked() {
                                            set_leader = Some((squad_idx, member.clone()));
                                        }
                                    }
                                    if ui.small_button("✕ Remove").clicked() {
                                        remove_from_squad = Some((squad_idx, member.clone()));
                                    }
                                });
                            }
                        }

                        // Allow adding unassigned members directly from squad frame too
                        if !unassigned.is_empty() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Add:").color(Color32::GRAY).small());
                                for member in &unassigned {
                                    if ui.small_button(member.as_str()).clicked() {
                                        add_to_squad = Some((member.clone(), squad_idx));
                                    }
                                }
                            });
                        }
                    });

                ui.add_space(4.0);
            }

            ui.add_space(8.0);

            // ── Create squad input ─────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.label("New squad name:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_squad_name)
                        .desired_width(180.0)
                        .hint_text("Alpha, Bravo, …"),
                );
                let can_create = !self.new_squad_name.trim().is_empty()
                    && !self
                        .squads
                        .iter()
                        .any(|s| s.name == self.new_squad_name.trim());
                ui.add_enabled_ui(can_create, |ui| {
                    if ui.button("Create Squad").clicked() {
                        create_squad = Some(self.new_squad_name.trim().to_owned());
                    }
                });
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // Status / error
            if let Some(err) = &st.error {
                ui.label(RichText::new(err).color(Color32::RED).small());
                ui.add_space(4.0);
            } else if !st.status.is_empty() {
                ui.label(RichText::new(&st.status).color(Color32::GRAY).small());
                ui.add_space(4.0);
            }

            // ── Navigation buttons ─────────────────────────────────────────
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                if ui
                    .add_sized([130.0, 36.0], egui::Button::new("← Back"))
                    .clicked()
                {
                    go_squad_setup = true;
                }
                ui.add_space(8.0);
                if ui
                    .add_sized([180.0, 36.0], egui::Button::new("→ Start Session"))
                    .clicked()
                {
                    go_running = true;
                }
            });

            ui.add_space(8.0);
        });

        // ── Apply mutations ────────────────────────────────────────────────

        if let Some(name) = create_squad {
            self.squads.push(SquadDraft {
                name,
                members: Vec::new(),
                leader: None,
            });
            self.new_squad_name.clear();
        }

        // let-chains require nightly; keep as nested ifs on stable.
        #[allow(clippy::collapsible_if)]
        if let Some((squad_idx, member)) = remove_from_squad {
            if let Some(squad) = self.squads.get_mut(squad_idx) {
                squad.members.retain(|m| m != &member);
                if squad.leader.as_deref() == Some(member.as_str()) {
                    squad.leader = None;
                }
            }
        }

        #[allow(clippy::collapsible_if)]
        if let Some((squad_idx, member)) = set_leader {
            if let Some(squad) = self.squads.get_mut(squad_idx) {
                squad.leader = Some(member);
            }
        }

        if let Some((member, squad_idx)) = add_to_squad {
            // Remove from any existing squad first
            for squad in &mut self.squads {
                squad.members.retain(|m| m != &member);
                if squad.leader.as_deref() == Some(member.as_str()) {
                    squad.leader = None;
                }
            }
            #[allow(clippy::collapsible_if)]
            if let Some(squad) = self.squads.get_mut(squad_idx) {
                if !squad.members.contains(&member) {
                    squad.members.push(member);
                }
            }
        }

        if go_running {
            self.screen = Screen::Running;
        } else if go_squad_setup {
            self.backend.send(BackendCmd::LeaveSession);
            self.squads.clear();
            self.room_id_input.clear();
            self.screen = Screen::SquadSetup;
        }
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

            // If disbanded remotely, go back to TeamSetup
            if st.room_id.is_none() && self.screen == Screen::Running {
                self.screen = Screen::TeamSetup;
            }

            ui.add_space(20.0);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                if ui
                    .add_sized([150.0, 36.0], egui::Button::new("Leave Session"))
                    .clicked()
                {
                    self.backend.send(BackendCmd::LeaveSession);
                    // Go back to TeamSetup (not SquadSetup)
                    self.screen = Screen::TeamSetup;
                }
                // Disband is shown separately — destructive action
                ui.add_space(8.0);
                let disband_btn =
                    egui::Button::new(RichText::new("Disband Squad").color(Color32::RED));
                if ui.add_sized([150.0, 36.0], disband_btn).clicked() {
                    self.backend.send(BackendCmd::DisbandSquad);
                    self.squads.clear();
                    self.room_id_input.clear();
                    self.screen = Screen::SquadSetup;
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
                Screen::TeamSetup => self.show_team_setup(ui, &st),
                Screen::Running => self.show_running(ui, ctx, &st),
            }
        });
    }
}
