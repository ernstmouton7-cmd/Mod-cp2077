//! Driver archetypes and sampled personalities.

use crate::math::scalar::clamp;
use crate::math::Rng;

/// Driver archetype.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Archetype {
    Commuter,
    Cautious,
    Speeder,
    Aggressor,
    Distracted,
    Professional,
    Joyrider,
    Zen,
}

/// Stable traits of a simulated driver (0..1 unless noted).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Personality {
    pub archetype: Archetype,
    pub label: &'static str,
    pub aggression: f64,
    /// How fast anger fades.
    pub patience: f64,
    /// Respect for signals, limits and signs.
    pub lawfulness: f64,
    /// Reaction skill; low = late, harsh reactions.
    pub skill: f64,
    /// Low = phone at the wheel.
    pub attentiveness: f64,
    /// MOBIL politeness factor.
    pub politeness: f64,
    /// Desired speed relative to the limit (1.1 = 10% over).
    pub speed_factor: f64,
    /// Desired time headway (s).
    pub headway: f64,
    /// Max comfortable acceleration multiplier.
    pub accel_factor: f64,
    /// Comfortable deceleration (m/s²).
    pub comfort_decel: f64,
    /// Standstill gap (m).
    pub min_gap: f64,
    /// Chance of signalling turns/lane changes.
    pub signal_discipline: f64,
    /// Anger gain multiplier.
    pub temper: f64,
    /// Tendency to use the horn.
    pub horn_happiness: f64,
    /// Critical gap for yielding/turning across traffic (s).
    pub critical_gap: f64,
}

/// Base trait values of an archetype (before jitter).
#[derive(Clone, Copy, Debug)]
pub struct Template {
    pub label: &'static str,
    pub weight: f64,
    /// aggression, patience, lawfulness, skill, attentiveness, politeness,
    /// speed_factor, headway, accel_factor, comfort_decel, min_gap,
    /// signal_discipline, temper, horn_happiness, critical_gap
    pub base: [f64; 15],
}

/// All archetypes in sampling order.
pub const ARCHETYPES: [Archetype; 8] = [
    Archetype::Commuter,
    Archetype::Cautious,
    Archetype::Speeder,
    Archetype::Aggressor,
    Archetype::Distracted,
    Archetype::Professional,
    Archetype::Joyrider,
    Archetype::Zen,
];

/// Template for an archetype.
pub fn template(a: Archetype) -> Template {
    match a {
        Archetype::Commuter => Template {
            label: "Commuter",
            weight: 40.0,
            base: [
                0.35, 0.55, 0.75, 0.6, 0.8, 0.4, 1.03, 1.4, 1.0, 2.2, 2.2, 0.85, 1.0, 0.35, 4.5,
            ],
        },
        Archetype::Cautious => Template {
            label: "Cautious driver",
            weight: 14.0,
            base: [
                0.1, 0.85, 0.95, 0.4, 0.85, 0.8, 0.9, 2.0, 0.7, 1.8, 3.0, 0.98, 0.5, 0.1, 6.0,
            ],
        },
        Archetype::Speeder => Template {
            label: "Hurried courier",
            weight: 14.0,
            base: [
                0.6, 0.35, 0.45, 0.7, 0.75, 0.15, 1.2, 0.9, 1.3, 3.0, 1.6, 0.5, 1.3, 0.6, 3.5,
            ],
        },
        Archetype::Aggressor => Template {
            label: "Hot-head",
            weight: 8.0,
            base: [
                0.9, 0.12, 0.3, 0.65, 0.7, 0.0, 1.25, 0.7, 1.4, 3.5, 1.4, 0.25, 2.0, 0.95, 2.8,
            ],
        },
        Archetype::Distracted => Template {
            label: "Distracted driver",
            weight: 8.0,
            base: [
                0.3, 0.6, 0.7, 0.35, 0.25, 0.4, 0.97, 1.5, 0.9, 2.2, 2.2, 0.55, 0.9, 0.2, 4.5,
            ],
        },
        Archetype::Professional => Template {
            label: "Professional driver",
            weight: 8.0,
            base: [
                0.4, 0.65, 0.8, 0.9, 0.9, 0.3, 1.05, 1.2, 1.1, 2.5, 2.0, 0.9, 0.9, 0.55, 3.8,
            ],
        },
        Archetype::Joyrider => Template {
            label: "Joyrider",
            weight: 5.0,
            base: [
                0.75, 0.3, 0.2, 0.55, 0.6, 0.05, 1.4, 0.8, 1.5, 3.5, 1.5, 0.2, 1.5, 0.8, 2.5,
            ],
        },
        Archetype::Zen => Template {
            label: "Zen master",
            weight: 3.0,
            base: [
                0.02, 1.0, 0.9, 0.8, 0.95, 1.0, 0.98, 1.8, 0.9, 2.0, 2.5, 1.0, 0.2, 0.02, 5.0,
            ],
        },
    }
}

/// Sample a personality; `forced` picks the archetype explicitly.
pub fn sample_personality(rng: &mut Rng, forced: Option<Archetype>) -> Personality {
    let arch = match forced {
        Some(a) => a,
        None => {
            let entries: Vec<(Archetype, f64)> = ARCHETYPES
                .iter()
                .map(|&a| (a, template(a).weight))
                .collect();
            rng.weighted(&entries)
        }
    };
    let t = template(arch);
    let b = t.base;
    let mut j = |v: f64, spread: f64, lo: f64, hi: f64| clamp(v + rng.gauss(0.0, spread), lo, hi);
    Personality {
        archetype: arch,
        label: t.label,
        aggression: j(b[0], 0.08, 0.0, 1.0),
        patience: j(b[1], 0.08, 0.0, 1.0),
        lawfulness: j(b[2], 0.07, 0.0, 1.0),
        skill: j(b[3], 0.08, 0.1, 1.0),
        attentiveness: j(b[4], 0.06, 0.05, 1.0),
        politeness: j(b[5], 0.08, 0.0, 1.0),
        speed_factor: j(b[6], 0.04, 0.8, 1.55),
        headway: j(b[7], 0.12, 0.5, 2.6),
        accel_factor: j(b[8], 0.08, 0.5, 1.6),
        comfort_decel: j(b[9], 0.2, 1.4, 4.0),
        min_gap: j(b[10], 0.2, 1.2, 3.5),
        signal_discipline: j(b[11], 0.05, 0.0, 1.0),
        temper: j(b[12], 0.1, 0.1, 2.5),
        horn_happiness: j(b[13], 0.06, 0.0, 1.0),
        critical_gap: j(b[14], 0.3, 2.0, 7.0),
    }
}
