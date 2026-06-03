//! Squelch egui application — Lobby-First UX (ADR-0008).

use eframe::CreationContext;
use egui::{Align, Color32, FontId, Layout, RichText, Vec2};
use squelch_audio::PttState;

use crate::{
    backend::{Backend, BackendCmd},
    config::AppConfig,
};

// ── Squad model (local UI state) ──────────────────────────────────────────

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
    SquadSetup, // room ID input + PTT config
    RoomView,   // live lobby + squad view (ADR-0008)
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

    // Room view state
    squads: Vec<SquadDraft>,
    new_squad_name: String,
    /// Which squad the local player is currently in (None = Lobby)
    my_squad: Option<usize>,
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
            my_squad: None,
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

            if let Some(err) = &st.error {
                ui.label(RichText::new(err).color(Color32::RED));
                ui.add_space(6.0);
            } else if st.status != "Not connected" {
                ui.label(RichText::new(&st.status).color(Color32::GRAY));
                ui.add_space(6.0);
            }

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
                RichText::new("Join or Create a Room")
                    .font(FontId::proportional(20.0))
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
                            .hint_text("e.g. CapsLock, F1"),
                    );
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Join a room → you land in the Lobby (open mic with everyone).\n\
                     Assign yourself to a squad to switch to squad audio.",
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

            // Transition to RoomView when room is active
            if st.room_id.is_some() && self.screen == Screen::SquadSetup {
                self.squads.clear();
                self.my_squad = None;
                self.screen = Screen::RoomView;
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
                    "Create Room"
                } else {
                    "Join Room"
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

    fn show_room_view(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        st: &crate::backend::BackendState,
    ) {
        // Collect all members
        let mut all_members: Vec<String> = Vec::new();
        if let Some(uid) = &st.user_id {
            all_members.push(uid.clone());
        }
        for peer in &st.peers {
            if !all_members.contains(peer) {
                all_members.push(peer.clone());
            }
        }

        let my_uid = st.user_id.clone().unwrap_or_default();

        // Who is assigned to a squad
        let assigned: Vec<String> = self
            .squads
            .iter()
            .flat_map(|s| s.members.iter().cloned())
            .collect();

        let lobby_members: Vec<String> = all_members
            .iter()
            .filter(|m| !assigned.contains(m))
            .cloned()
            .collect();

        // Pending mutations
        let mut create_squad: Option<String> = None;
        let mut join_squad: Option<usize> = None; // self joins
        let mut leave_squad: bool = false;
        let mut assign_to_squad: Option<(String, usize)> = None; // leader assigns other
        let mut remove_from_squad: Option<(usize, String)> = None;
        let mut set_leader: Option<(usize, String)> = None;
        let mut disband_squad: Option<usize> = None;
        let mut do_leave_session: bool = false;
        let mut do_disband_room: bool = false;

        // Find which squad I'm in
        let my_squad_idx = self.squads.iter().position(|s| s.members.contains(&my_uid));
        let i_am_leader = my_squad_idx
            .and_then(|i| self.squads.get(i))
            .map(|s| s.leader.as_deref() == Some(my_uid.as_str()))
            .unwrap_or(false);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Header ──────────────────────────────────────────────────────
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label(
                        RichText::new("🎙 Squelch")
                            .strong()
                            .font(FontId::proportional(18.0)),
                    );
                    ui.add_space(8.0);

                    // Audio indicator
                    let ptt_on = self.ptt.is_active();
                    if ptt_on {
                        ui.label(
                            RichText::new("● LEADER NET")
                                .color(Color32::RED)
                                .small()
                                .strong(),
                        );
                    } else if st.audio_active {
                        ui.label(RichText::new("🎙").color(Color32::GREEN).small());
                    } else {
                        ui.label(RichText::new("⚠ no audio").color(Color32::YELLOW).small());
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let leave_btn = egui::Button::new(
                            RichText::new("Leave Room").color(Color32::LIGHT_GRAY),
                        );
                        if ui.add(leave_btn).clicked() {
                            do_leave_session = true;
                        }
                        ui.add_space(4.0);
                        let disband_btn =
                            egui::Button::new(RichText::new("Disband").color(Color32::RED));
                        if ui.add(disband_btn).clicked() {
                            do_disband_room = true;
                        }
                    });
                });
            });

            // Room ID
            if let Some(room_id) = &st.room_id {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Room:").color(Color32::GRAY).small());
                    let mut rid = room_id.clone();
                    ui.add(
                        egui::TextEdit::singleline(&mut rid)
                            .desired_width(300.0)
                            .interactive(false),
                    );
                    if ui.small_button("📋").clicked() {
                        ctx.copy_text(room_id.clone());
                    }
                });
            }

            ui.separator();

            // ── Lobby ────────────────────────────────────────────────────────
            ui.add_space(6.0);
            let lobby_label = if lobby_members.is_empty() {
                RichText::new("Lobby  (empty)").color(Color32::GRAY)
            } else {
                RichText::new(format!("Lobby  ({} online, open mic)", lobby_members.len()))
                    .color(Color32::WHITE)
                    .strong()
            };
            ui.label(lobby_label);
            ui.add_space(4.0);

            if lobby_members.is_empty() {
                ui.label(
                    RichText::new("  Everyone is in a squad.")
                        .color(Color32::GRAY)
                        .small(),
                );
            } else {
                for member in &lobby_members {
                    let is_me = member == &my_uid;
                    ui.horizontal(|ui| {
                        let prefix = if is_me { "▶ " } else { "  " };
                        ui.label(RichText::new(format!("{prefix}{member}")).color(if is_me {
                            Color32::LIGHT_BLUE
                        } else {
                            Color32::WHITE
                        }));

                        // Self: join any squad
                        if is_me {
                            for (idx, squad) in self.squads.iter().enumerate() {
                                if ui.small_button(format!("→ {}", squad.name)).clicked() {
                                    join_squad = Some(idx);
                                }
                            }
                        }

                        // Leader of a squad can pull lobby members in
                        // let-chains require nightly; keep as nested ifs on stable.
                        #[allow(clippy::collapsible_if)]
                        if !is_me && i_am_leader {
                            if let Some(my_idx) = my_squad_idx {
                                let squad_name = self.squads[my_idx].name.clone();
                                if ui.small_button(format!("+ {squad_name}")).clicked() {
                                    assign_to_squad = Some((member.clone(), my_idx));
                                }
                            }
                        }
                    });
                }
            }

            // Leave squad button for me
            if my_squad_idx.is_some() {
                ui.add_space(4.0);
                if ui.small_button("← Return to Lobby").clicked() {
                    leave_squad = true;
                }
            }

            ui.add_space(8.0);
            ui.separator();

            // ── Squads ───────────────────────────────────────────────────────
            ui.add_space(6.0);
            ui.label(RichText::new("Squads").strong());
            ui.add_space(4.0);

            if self.squads.is_empty() {
                ui.label(
                    RichText::new("  No squads yet.")
                        .color(Color32::GRAY)
                        .small(),
                );
            }

            for (squad_idx, squad) in self.squads.iter().enumerate() {
                let is_my_squad = my_squad_idx == Some(squad_idx);
                let frame_color = if is_my_squad {
                    Color32::from_rgb(30, 60, 30)
                } else {
                    Color32::from_gray(30)
                };

                egui::Frame::new()
                    .inner_margin(egui::Margin::same(6))
                    .fill(frame_color)
                    .corner_radius(egui::CornerRadius::same(4))
                    .show(ui, |ui| {
                        // Squad header
                        ui.horizontal(|ui| {
                            let name_color = if is_my_squad {
                                Color32::GREEN
                            } else {
                                Color32::LIGHT_BLUE
                            };
                            ui.label(
                                RichText::new(format!("■ {}", squad.name))
                                    .color(name_color)
                                    .strong(),
                            );

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                // Disband: only squad leader
                                let squad_leader = squad.leader.as_deref().unwrap_or_default();
                                if squad_leader == my_uid.as_str() {
                                    let db_btn = egui::Button::new(
                                        RichText::new("Disband").color(Color32::RED).small(),
                                    );
                                    if ui.add(db_btn).clicked() {
                                        disband_squad = Some(squad_idx);
                                    }
                                }
                            });
                        });

                        // Members
                        if squad.members.is_empty() {
                            ui.label(RichText::new("  (empty)").color(Color32::GRAY).small());
                        } else {
                            for member in &squad.members {
                                let is_leader = squad.leader.as_deref() == Some(member.as_str());
                                let is_me = member == &my_uid;
                                ui.horizontal(|ui| {
                                    let icon = if is_leader { "★" } else { "  " };
                                    let color = if is_leader {
                                        Color32::GOLD
                                    } else if is_me {
                                        Color32::LIGHT_BLUE
                                    } else {
                                        Color32::WHITE
                                    };
                                    ui.label(
                                        RichText::new(format!("{icon} {member}")).color(color),
                                    );

                                    // Make leader (squad leader only, on non-leader members)
                                    #[allow(clippy::collapsible_if)]
                                    if i_am_leader && Some(squad_idx) == my_squad_idx && !is_leader
                                    {
                                        if ui
                                            .small_button("★")
                                            .on_hover_text("Make Leader")
                                            .clicked()
                                        {
                                            set_leader = Some((squad_idx, member.clone()));
                                        }
                                    }
                                    // Remove (squad leader can remove others; anyone can remove themselves)
                                    #[allow(clippy::collapsible_if)]
                                    if (i_am_leader && Some(squad_idx) == my_squad_idx) || is_me {
                                        if ui
                                            .small_button("✕")
                                            .on_hover_text("Remove from squad")
                                            .clicked()
                                        {
                                            remove_from_squad = Some((squad_idx, member.clone()));
                                        }
                                    }
                                });
                            }
                        }

                        // Non-member in lobby: join this squad
                        #[allow(clippy::collapsible_if)]
                        if my_squad_idx.is_none() && !squad.members.contains(&my_uid) {
                            if ui.small_button(format!("→ Join {}", squad.name)).clicked() {
                                join_squad = Some(squad_idx);
                            }
                        }
                    });

                ui.add_space(4.0);
            }

            // ── Create squad ─────────────────────────────────────────────────
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_squad_name)
                        .desired_width(160.0)
                        .hint_text("New squad name…"),
                );
                let can_create = !self.new_squad_name.trim().is_empty()
                    && !self
                        .squads
                        .iter()
                        .any(|s| s.name == self.new_squad_name.trim());
                ui.add_enabled_ui(can_create, |ui| {
                    if ui.button("+ Create Squad").clicked() {
                        create_squad = Some(self.new_squad_name.trim().to_owned());
                    }
                });
            });

            // Status
            if let Some(err) = &st.error {
                ui.add_space(4.0);
                ui.label(RichText::new(err).color(Color32::RED).small());
            }
        });

        // ── Apply mutations ──────────────────────────────────────────────────

        if let Some(name) = create_squad {
            self.squads.push(SquadDraft {
                name,
                members: Vec::new(),
                leader: None,
            });
            self.new_squad_name.clear();
        }

        if let Some(idx) = join_squad {
            // Remove self from any squad first
            for s in &mut self.squads {
                s.members.retain(|m| m != &my_uid);
            }
            if let Some(squad) = self.squads.get_mut(idx) {
                squad.members.push(my_uid.clone());
                if squad.leader.is_none() {
                    squad.leader = Some(my_uid.clone());
                }
            }
        }

        if leave_squad {
            for s in &mut self.squads {
                if s.leader.as_deref() == Some(my_uid.as_str()) && s.members.len() > 1 {
                    // Transfer leadership to next member
                    let next = s
                        .members
                        .iter()
                        .find(|m| m.as_str() != my_uid.as_str())
                        .cloned();
                    s.leader = next;
                }
                s.members.retain(|m| m != &my_uid);
            }
        }

        if let Some((member, idx)) = assign_to_squad {
            for s in &mut self.squads {
                s.members.retain(|m| m != &member);
            }
            #[allow(clippy::collapsible_if)]
            if let Some(squad) = self.squads.get_mut(idx) {
                if !squad.members.contains(&member) {
                    squad.members.push(member);
                }
            }
        }

        #[allow(clippy::collapsible_if)]
        if let Some((idx, member)) = remove_from_squad {
            if let Some(squad) = self.squads.get_mut(idx) {
                if squad.leader.as_deref() == Some(member.as_str()) {
                    squad.leader = squad
                        .members
                        .iter()
                        .find(|m| m.as_str() != member.as_str())
                        .cloned();
                }
                squad.members.retain(|m| m != &member);
            }
        }

        #[allow(clippy::collapsible_if)]
        if let Some((idx, member)) = set_leader {
            if let Some(squad) = self.squads.get_mut(idx) {
                squad.leader = Some(member);
            }
        }

        #[allow(clippy::collapsible_if)]
        if let Some(idx) = disband_squad {
            if idx < self.squads.len() {
                self.squads.remove(idx);
            }
        }

        if do_leave_session {
            self.backend.send(BackendCmd::LeaveSession);
            self.squads.clear();
            self.my_squad = None;
            self.room_id_input.clear();
            self.screen = Screen::SquadSetup;
        }

        if do_disband_room {
            self.backend.send(BackendCmd::DisbandSquad);
            self.squads.clear();
            self.my_squad = None;
            self.room_id_input.clear();
            self.screen = Screen::SquadSetup;
        }

        // Remote disband
        if st.room_id.is_none() && self.screen == Screen::RoomView {
            self.squads.clear();
            self.my_squad = None;
            self.screen = Screen::SquadSetup;
        }
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
                name: format!("Squelch — {}", self.cfg.username),
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
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        let st = self.backend.snapshot();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_size(Vec2::new(480.0, 560.0));
            match self.screen.clone() {
                Screen::Login => self.show_login(ui, &st),
                Screen::SquadSetup => self.show_squad_setup(ui, &st),
                Screen::RoomView => self.show_room_view(ui, ctx, &st),
            }
        });
    }
}
