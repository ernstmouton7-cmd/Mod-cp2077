#![allow(deprecated)]
//! In-game HUD (gauges, signals, money/points/heat, toasts) and the pause menu (egui).

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, LayerId, Order, Pos2, Rect, Shape, Stroke, Vec2,
};
use gridlock_game::{GameEvent, HudInfo};
use gridlock_input::{button_label, Device, PadBinding, PadInput, PadKind, ACTIONS};
use gridlock_render::Quality;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct Settings {
    pub quality: Quality,
    pub render_scale: f32,
    pub fov: f32,
    pub rumble: f32,
    pub steer_deadzone: f32,
    pub steer_exponent: f32,
    pub steer_sensitivity: f32,
    pub steer_assist: f32,
    pub abs: bool,
    pub tcs: bool,
    pub esc: bool,
    pub auto_clutch: bool,
    pub show_fps: bool,
    pub hour: f32,
    pub hour_changed: bool,
    pub quality_changed: bool,
    pub quit: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            quality: Quality::High,
            render_scale: 1.0,
            fov: 62.0,
            rumble: 0.8,
            steer_deadzone: 0.08,
            steer_exponent: 1.55,
            steer_sensitivity: 1.0,
            steer_assist: 0.55,
            abs: true,
            tcs: true,
            esc: true,
            auto_clutch: true,
            show_fps: true,
            hour: 17.5,
            hour_changed: false,
            quality_changed: false,
            quit: false,
        }
    }
}

struct Toast {
    text: String,
    color: Color32,
    ttl: f32,
}

#[derive(Default)]
pub struct Hud {
    toasts: VecDeque<Toast>,
    pub menu_open: bool,
    tab: usize,
    pub fps: f32,
    pub frame_ms: f32,
    pub sim_ms: f32,
    pub camera_label: &'static str,
}

const AMBER: Color32 = Color32::from_rgb(255, 176, 32);
const GREEN: Color32 = Color32::from_rgb(70, 230, 120);
const RED: Color32 = Color32::from_rgb(240, 60, 50);
const TEXT: Color32 = Color32::from_rgb(235, 238, 242);
const DIM: Color32 = Color32::from_rgba_premultiplied(150, 156, 166, 200);
const PANEL: Color32 = Color32::from_rgba_premultiplied(8, 10, 14, 170);

impl Hud {
    pub fn push(&mut self, text: impl Into<String>, color: Color32) {
        self.toasts.push_back(Toast {
            text: text.into(),
            color,
            ttl: 4.0,
        });
        while self.toasts.len() > 6 {
            self.toasts.pop_front();
        }
    }

    pub fn on_events(&mut self, events: &[GameEvent], player: Option<u32>) {
        use gridlock_game::traffic::Event;
        for e in events {
            match e {
                GameEvent::Traffic(Event::Violation { vehicle, label, .. })
                    if Some(vehicle.0) == player =>
                {
                    self.push(format!("Violation: {label}"), RED)
                }
                GameEvent::Traffic(Event::Speech { text, .. }) => {
                    self.push(format!("\u{201c}{text}\u{201d}"), TEXT)
                }
                GameEvent::Traffic(Event::RageStart { target, .. }) if Some(target.0) == player => {
                    self.push("A driver is coming after you", RED)
                }
                GameEvent::Traffic(Event::Collision { a, b, .. })
                    if Some(a.0) == player || Some(b.0) == player =>
                {
                    self.push("Collision", AMBER)
                }
                GameEvent::Damage {
                    vehicle,
                    event: gridlock_game::physics::deform::DamageEvent::PartDetached { .. },
                } if Some(vehicle.0) == player => self.push("Part torn off", AMBER),
                GameEvent::Damage {
                    vehicle,
                    event: gridlock_game::physics::deform::DamageEvent::WheelDetached { .. },
                } if Some(vehicle.0) == player => self.push("You lost a wheel!", RED),
                GameEvent::Damage {
                    vehicle,
                    event: gridlock_game::physics::deform::DamageEvent::TyrePuncture { .. },
                } if Some(vehicle.0) == player => self.push("Puncture", AMBER),
                GameEvent::Prop(gridlock_game::physics::props::PropEvent::HydrantBurst {
                    ..
                }) => self.push("Hydrant burst", AMBER),
                _ => {}
            }
        }
    }

    pub fn draw(
        &mut self,
        ctx: &egui::Context,
        dt: f32,
        h: &HudInfo,
        device: Device,
        pad: PadKind,
        settings: &mut Settings,
    ) {
        let screen = ctx.content_rect();
        let p = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("hud")));
        let s = (screen.height() / 1080.0).clamp(0.6, 2.0);

        // Speedometer (bottom right).
        let c = Pos2::new(screen.right() - 170.0 * s, screen.bottom() - 150.0 * s);
        let r = 115.0 * s;
        p.circle_filled(c, r + 10.0 * s, PANEL);
        let arc =
            |p: &egui::Painter, from: f32, to: f32, radius: f32, width: f32, color: Color32| {
                let a0 = std::f32::consts::PI * 0.75;
                let span = std::f32::consts::PI * 1.5;
                let n = 48;
                let pts: Vec<Pos2> = (0..=n)
                    .map(|k| {
                        let t = from + (to - from) * k as f32 / n as f32;
                        let a = a0 + span * t;
                        c + Vec2::new(a.cos(), a.sin()) * radius
                    })
                    .collect();
                p.add(Shape::line(pts, Stroke::new(width, color)));
            };
        arc(&p, 0.0, 1.0, r, 3.0 * s, DIM);
        let max_kmh = 260.0;
        arc(
            &p,
            0.0,
            (h.speed_kmh / max_kmh).min(1.0),
            r,
            7.0 * s,
            if h.limit_kmh > 0.0 && h.speed_kmh > h.limit_kmh + 5.0 {
                RED
            } else {
                TEXT
            },
        );
        let rpm_t = if h.redline > 0.0 {
            (h.rpm / h.redline).min(1.1)
        } else {
            0.0
        };
        arc(
            &p,
            0.0,
            rpm_t.min(1.0),
            r - 16.0 * s,
            4.0 * s,
            if rpm_t > 0.92 { RED } else { AMBER },
        );
        p.text(
            c + Vec2::new(0.0, -6.0 * s),
            Align2::CENTER_CENTER,
            format!("{:.0}", h.speed_kmh),
            FontId::proportional(54.0 * s),
            TEXT,
        );
        p.text(
            c + Vec2::new(0.0, 30.0 * s),
            Align2::CENTER_CENTER,
            "km/h",
            FontId::proportional(15.0 * s),
            DIM,
        );
        p.text(
            c + Vec2::new(0.0, 62.0 * s),
            Align2::CENTER_CENTER,
            &h.gear,
            FontId::monospace(30.0 * s),
            AMBER,
        );
        // Speed-limit sign.
        if h.limit_kmh > 0.0 {
            let lc = Pos2::new(c.x - r - 64.0 * s, c.y + 40.0 * s);
            p.circle_filled(lc, 32.0 * s, Color32::WHITE);
            p.circle_stroke(lc, 28.0 * s, Stroke::new(6.0 * s, RED));
            p.text(
                lc,
                Align2::CENTER_CENTER,
                format!("{:.0}", h.limit_kmh),
                FontId::proportional(24.0 * s),
                Color32::BLACK,
            );
        }
        // Indicators and warning lights above the dial.
        let blink = (ctx.input(|i| i.time) * 2.0).fract() < 0.5;
        let y = c.y - r - 30.0 * s;
        let lit = |on: bool, col: Color32| {
            if on {
                col
            } else {
                Color32::from_rgba_premultiplied(60, 64, 70, 160)
            }
        };
        p.text(
            Pos2::new(c.x - 60.0 * s, y),
            Align2::CENTER_CENTER,
            "\u{25C0}",
            FontId::proportional(28.0 * s),
            lit((h.left_indicator || h.hazards) && blink, GREEN),
        );
        p.text(
            Pos2::new(c.x + 60.0 * s, y),
            Align2::CENTER_CENTER,
            "\u{25B6}",
            FontId::proportional(28.0 * s),
            lit((h.right_indicator || h.hazards) && blink, GREEN),
        );
        let lights = [
            ("ABS", h.abs_active, AMBER),
            ("TCS", h.tcs_active, AMBER),
            ("ESC", h.esc_active, AMBER),
            ("BELT", !h.seatbelt, RED),
            ("HI", h.high_beam, Color32::from_rgb(80, 150, 255)),
            ("ENG", h.engine_health < 0.6 || !h.engine_running, RED),
        ];
        for (i, (label, on, col)) in lights.iter().enumerate() {
            let x = c.x - 100.0 * s + i as f32 * 40.0 * s;
            p.text(
                Pos2::new(x, c.y + r + 2.0 * s),
                Align2::CENTER_CENTER,
                *label,
                FontId::monospace(12.0 * s),
                lit(*on, *col),
            );
        }
        if let Some(cr) = h.cruise_kmh {
            p.text(
                Pos2::new(c.x, c.y + 88.0 * s),
                Align2::CENTER_CENTER,
                format!("CRUISE {cr:.0}"),
                FontId::monospace(13.0 * s),
                GREEN,
            );
        } else if h.limiter {
            p.text(
                Pos2::new(c.x, c.y + 88.0 * s),
                Align2::CENTER_CENTER,
                "LIMITER",
                FontId::monospace(13.0 * s),
                GREEN,
            );
        }

        // Status panel (bottom left).
        let panel = Rect::from_min_size(
            Pos2::new(screen.left() + 24.0 * s, screen.bottom() - 150.0 * s),
            Vec2::new(360.0 * s, 126.0 * s),
        );
        p.rect_filled(panel, CornerRadius::same(10), PANEL);
        let hour = h.hour.rem_euclid(24.0);
        p.text(
            panel.left_top() + Vec2::new(16.0, 14.0) * s,
            Align2::LEFT_TOP,
            if h.street.is_empty() {
                "—".to_string()
            } else {
                h.street.clone()
            },
            FontId::proportional(24.0 * s),
            TEXT,
        );
        p.text(
            panel.right_top() + Vec2::new(-16.0, 16.0) * s,
            Align2::RIGHT_TOP,
            format!("{:02}:{:02}", hour as u32, ((hour.fract()) * 60.0) as u32),
            FontId::monospace(18.0 * s),
            DIM,
        );
        p.text(
            panel.left_top() + Vec2::new(16.0, 52.0) * s,
            Align2::LEFT_TOP,
            format!("${:.0}", h.money),
            FontId::proportional(22.0 * s),
            GREEN,
        );
        p.text(
            panel.left_top() + Vec2::new(150.0, 54.0) * s,
            Align2::LEFT_TOP,
            format!("Licence {}/{}", h.points, h.max_points),
            FontId::proportional(18.0 * s),
            if h.points >= h.max_points.saturating_sub(3) {
                RED
            } else {
                TEXT
            },
        );
        // Heat stars.
        let stars = h.heat.clamp(0.0, 5.0).ceil() as usize;
        for k in 0..5 {
            let on = k < stars;
            p.text(
                panel.left_top() + Vec2::new(16.0 + k as f32 * 26.0, 86.0) * s,
                Align2::LEFT_TOP,
                "\u{2605}",
                FontId::proportional(22.0 * s),
                if on {
                    if blink {
                        RED
                    } else {
                        Color32::from_rgb(80, 120, 255)
                    }
                } else {
                    Color32::from_rgba_premultiplied(70, 70, 80, 140)
                },
            );
        }
        let lights_label = ["Lights off", "Lights auto", "Lights on"][h.headlights.min(2) as usize];
        p.text(
            panel.left_top() + Vec2::new(160.0, 90.0) * s,
            Align2::LEFT_TOP,
            lights_label,
            FontId::proportional(15.0 * s),
            DIM,
        );

        // Toasts (top centre).
        for t in self.toasts.iter_mut() {
            t.ttl -= dt;
        }
        self.toasts.retain(|t| t.ttl > 0.0);
        for (i, t) in self.toasts.iter().enumerate() {
            let a = (t.ttl / 0.5).min(1.0);
            let pos = Pos2::new(
                screen.center().x,
                screen.top() + (40.0 + i as f32 * 34.0) * s,
            );
            let galley_w = (t.text.len() as f32 * 9.5 + 32.0) * s;
            p.rect_filled(
                Rect::from_center_size(pos, Vec2::new(galley_w, 28.0 * s)),
                CornerRadius::same(8),
                PANEL.gamma_multiply(a),
            );
            p.text(
                pos,
                Align2::CENTER_CENTER,
                &t.text,
                FontId::proportional(18.0 * s),
                t.color.gamma_multiply(a),
            );
        }

        // Top-left: perf and hints.
        if settings.show_fps {
            p.text(
                screen.left_top() + Vec2::new(14.0, 12.0),
                Align2::LEFT_TOP,
                format!(
                    "{:.0} fps  {:.1} ms  sim {:.2} ms  cam {}",
                    self.fps, self.frame_ms, self.sim_ms, self.camera_label
                ),
                FontId::monospace(13.0),
                DIM,
            );
        }
        let hint = match device {
            Device::Gamepad => format!(
                "{} pause   {} camera   hold {} for more",
                button_label(pad, gridlock_input::PadButton::Menu),
                button_label(pad, gridlock_input::PadButton::North),
                button_label(pad, gridlock_input::PadButton::View)
            ),
            Device::Keyboard => "Esc pause   V camera   F1 controls".to_string(),
        };
        p.text(
            screen.left_top() + Vec2::new(14.0, 30.0),
            Align2::LEFT_TOP,
            hint,
            FontId::proportional(13.0),
            DIM,
        );

        if self.menu_open {
            self.menu(ctx, pad, settings);
        }
    }

    fn menu(&mut self, ctx: &egui::Context, pad: PadKind, st: &mut Settings) {
        egui::Window::new("Gridlock").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).default_width(560.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, name) in ["Driving", "Controller", "Graphics", "Controls"].iter().enumerate() {
                    if ui.selectable_label(self.tab == i, *name).clicked() {
                        self.tab = i;
                    }
                }
            });
            ui.separator();
            match self.tab {
                0 => {
                    ui.checkbox(&mut st.abs, "ABS (anti-lock brakes)");
                    ui.checkbox(&mut st.tcs, "Traction control");
                    ui.checkbox(&mut st.esc, "Stability control");
                    ui.checkbox(&mut st.auto_clutch, "Auto clutch (manual gearbox)");
                    ui.add(egui::Slider::new(&mut st.steer_assist, 0.0..=1.0).text("Steering assist (speed-sensitive + counter-steer)"));
                    if ui.add(egui::Slider::new(&mut st.hour, 0.0..=24.0).text("Time of day")).changed() {
                        st.hour_changed = true;
                    }
                }
                1 => {
                    ui.add(egui::Slider::new(&mut st.steer_deadzone, 0.0..=0.3).text("Stick deadzone"));
                    ui.add(egui::Slider::new(&mut st.steer_exponent, 1.0..=3.0).text("Steering curve (1 = linear)"));
                    ui.add(egui::Slider::new(&mut st.steer_sensitivity, 0.5..=1.6).text("Steering sensitivity"));
                    ui.add(egui::Slider::new(&mut st.rumble, 0.0..=1.0).text("Rumble strength"));
                    ui.label("Rumble: heavy motor = impacts, kerbs, rough roads, limiter; light motor = slides, wheelspin, shifts. On Windows the Xbox trigger motors pulse with ABS (LT) and traction control (RT).");
                }
                2 => {
                    egui::ComboBox::from_label("Quality").selected_text(format!("{:?}", st.quality)).show_ui(ui, |ui| {
                        for q in [Quality::Low, Quality::Medium, Quality::High, Quality::Ultra] {
                            if ui.selectable_value(&mut st.quality, q, format!("{q:?}")).changed() {
                                st.quality_changed = true;
                            }
                        }
                    });
                    if ui.add(egui::Slider::new(&mut st.render_scale, 0.5..=2.0).text("Render scale")).changed() {
                        st.quality_changed = true;
                    }
                    ui.add(egui::Slider::new(&mut st.fov, 45.0..=90.0).text("Field of view"));
                    ui.checkbox(&mut st.show_fps, "Show frame stats");
                }
                _ => {
                    egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                        egui::Grid::new("controls").striped(true).show(ui, |ui| {
                            for d in ACTIONS {
                                ui.label(d.label);
                                ui.label(d.keys.join(" / "));
                                ui.label(d.pad.iter().map(|b| pad_text(pad, b)).collect::<Vec<_>>().join(", "));
                                ui.end_row();
                            }
                        });
                    });
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Resume").clicked() {
                    self.menu_open = false;
                }
                if ui.button("Quit").clicked() {
                    st.quit = true;
                }
            });
        });
    }
}

fn pad_text(kind: PadKind, b: &PadBinding) -> String {
    let base = match b.input {
        PadInput::Button(x) => button_label(kind, x).to_string(),
        PadInput::AxisPos(a) | PadInput::AxisNeg(a) => {
            gridlock_input::axis_label(kind, a).to_string()
        }
        PadInput::Chord(x, y) => format!("{} + {}", button_label(kind, x), button_label(kind, y)),
    };
    let base = match b.gesture {
        gridlock_input::Gesture::Hold => format!("hold {base}"),
        _ => base,
    };
    match b.layer {
        gridlock_input::Layer::Modifier => format!(
            "{} + {base}",
            button_label(kind, gridlock_input::PadButton::View)
        ),
        _ => base,
    }
}
