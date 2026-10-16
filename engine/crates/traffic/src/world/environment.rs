//! Time of day and weather, with their effects on driving.

use crate::math::scalar::{clamp, fmod, smoothstep};
use crate::math::Rng;

fn bell(h: f64, c: f64, w: f64) -> f64 {
    (-((h - c) * (h - c)) / (2.0 * w * w)).exp()
}

/// Weather state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weather {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Fog,
}

/// Time of day and weather, with derived daylight, darkness, tyre grip,
/// visibility, traffic demand (rush hours) and night signal flashing.
#[derive(Clone, Debug)]
pub struct Environment {
    /// Hours 0..24.
    pub hour: f64,
    /// Simulated hours per simulated second (default: 1 game minute per second).
    pub time_scale: f64,
    pub weather: Weather,
    /// 0..1 rain intensity (smoothed).
    pub rain: f64,
    /// 0..1 fog density (smoothed).
    pub fog: f64,
    /// Road wetness lags rain.
    pub wetness: f64,
    weather_timer: f64,
    pub auto_weather: bool,
    rng: Rng,
    /// 0 (night) .. 1 (day).
    pub daylight: f64,
    pub dark: bool,
    /// Tyre grip multiplier from road wetness.
    pub grip: f64,
    /// Visibility (m).
    pub visibility: f64,
    /// Traffic demand multiplier by hour.
    pub traffic_density: f64,
    /// Late night: minor signals switch to flashing operation.
    pub night_flash: bool,
}

impl Environment {
    pub fn new(seed: i64, start_hour: f64) -> Self {
        let mut e = Environment {
            hour: start_hour,
            time_scale: 1.0 / 60.0,
            weather: Weather::Clear,
            rain: 0.0,
            fog: 0.0,
            wetness: 0.0,
            weather_timer: 600.0,
            auto_weather: true,
            rng: Rng::new(seed),
            daylight: 1.0,
            dark: false,
            grip: 1.0,
            visibility: 1200.0,
            traffic_density: 1.0,
            night_flash: false,
        };
        e.refresh();
        e
    }

    /// Recompute the derived values.
    pub fn refresh(&mut self) {
        let h = self.hour;
        self.daylight = smoothstep(5.5, 7.5, h) * (1.0 - smoothstep(18.5, 20.5, h));
        self.dark = self.daylight < 0.45 || self.fog > 0.6;
        self.grip = 1.0 - 0.32 * self.wetness;
        let night = 1.0 - self.daylight;
        self.visibility = clamp(1200.0 * (1.0 - self.fog * 0.9) * (1.0 - self.rain * 0.5) * (1.0 - night * 0.4), 40.0, 1200.0);
        self.traffic_density = clamp(0.25 + 0.75 * bell(h, 8.2, 1.4) + 0.85 * bell(h, 17.6, 1.7) + 0.45 * bell(h, 13.0, 2.5) + 0.25 * bell(h, 21.5, 1.5), 0.2, 1.1);
        self.night_flash = (1.0..5.0).contains(&h);
    }

    pub fn set_weather(&mut self, w: Weather) {
        self.weather = w;
        self.weather_timer = 900.0;
    }

    pub fn update(&mut self, dt: f64) {
        self.hour = fmod(self.hour + dt * self.time_scale, 24.0);
        if self.auto_weather {
            self.weather_timer -= dt;
            if self.weather_timer <= 0.0 {
                self.weather = self.rng.weighted(&[(Weather::Clear, 45.0), (Weather::Cloudy, 25.0), (Weather::Rain, 18.0), (Weather::Storm, 6.0), (Weather::Fog, 6.0)]);
                self.weather_timer = self.rng.range(400.0, 1200.0);
            }
        }
        let target_rain = match self.weather {
            Weather::Rain => 0.6,
            Weather::Storm => 1.0,
            _ => 0.0,
        };
        let target_fog = match self.weather {
            Weather::Fog => 0.8,
            Weather::Storm => 0.3,
            _ => 0.0,
        };
        self.rain += (target_rain - self.rain) * (dt * 0.1).min(1.0);
        self.fog += (target_fog - self.fog) * (dt * 0.05).min(1.0);
        let wet_target = if self.rain > 0.05 { (self.rain * 1.5).min(1.0) } else { 0.0 };
        self.wetness += (wet_target - self.wetness) * (dt * if wet_target > self.wetness { 0.08 } else { 0.01 }).min(1.0);
        self.refresh();
    }
}
