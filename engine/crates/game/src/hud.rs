//! Player-facing readouts for HUDs (shared by the native app and engine plugins).

use crate::game::Game;
use gridlock_traffic::vehicle::HeadlightSwitch;

#[derive(Clone, Debug, Default)]
pub struct HudInfo {
    pub speed_kmh: f32,
    pub rpm: f32,
    pub redline: f32,
    pub gear: String,
    /// Posted limit where the player is (km/h), 0 if unknown.
    pub limit_kmh: f32,
    pub street: String,
    pub money: f32,
    pub points: u32,
    pub max_points: u32,
    /// Police attention 0..
    pub heat: f32,
    pub left_indicator: bool,
    pub right_indicator: bool,
    pub hazards: bool,
    /// 0 off, 1 auto, 2 on.
    pub headlights: u8,
    pub high_beam: bool,
    pub seatbelt: bool,
    pub cruise_kmh: Option<f32>,
    pub limiter: bool,
    pub abs_active: bool,
    pub tcs_active: bool,
    pub esc_active: bool,
    pub engine_health: f32,
    pub engine_running: bool,
    pub hour: f32,
}

impl Game {
    pub fn hud(&self) -> HudInfo {
        let mut h = HudInfo::default();
        let Some(pid) = self.traffic.player_id() else {
            return h;
        };
        let Some(v) = self.traffic.vehicle(pid) else {
            return h;
        };
        if let Some(pv) = self
            .player_vehicle()
            .and_then(|vh| self.physics.vehicle(vh))
        {
            h.speed_kmh = pv.telemetry.speed.abs() * 3.6;
            h.rpm = pv.powertrain.rpm();
            h.redline = pv.desc.engine.redline;
            h.abs_active = pv.telemetry.abs_active;
            h.tcs_active = pv.telemetry.tcs_active;
            h.esc_active = pv.telemetry.esc_active;
            h.engine_health = pv.damage.engine;
            h.engine_running = pv.powertrain.running;
        }
        if let Some(pd) = &self.traffic.player_driver {
            h.gear = pd.gear_text(v);
            h.seatbelt = pd.seatbelt;
            h.cruise_kmh = if pd.cruise.on {
                Some(pd.cruise.target as f32 * 3.6)
            } else {
                None
            };
            h.limiter = pd.limiter_on;
        }
        let net = self.traffic.net();
        if let Some(hit) = net.nearest_lanes(v.x, v.y, 12.0).first() {
            let lane = net.lane(hit.lane);
            h.limit_kmh = (lane.speed_limit * 3.6).round() as f32;
            h.street = net.road(lane.road).name.clone();
        }
        let rec = &self.traffic.record;
        h.money = rec.money as f32;
        h.points = rec.points;
        h.max_points = rec.max_points;
        h.heat = rec.heat as f32;
        let l = &v.lights;
        h.left_indicator = l.left_lamp_on();
        h.right_indicator = l.right_lamp_on();
        h.hazards = l.hazard;
        h.headlights = match l.headlight_switch {
            HeadlightSwitch::Off => 0,
            HeadlightSwitch::Auto => 1,
            HeadlightSwitch::Low => 2,
        };
        h.high_beam = l.high_beam;
        h.hour = self.traffic.env().hour as f32;
        h
    }
}
