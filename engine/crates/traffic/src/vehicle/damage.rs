//! Per-zone body damage.

use crate::math::scalar::clamp;

/// Body zone hit by an impact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageZone {
    Front,
    Rear,
    Left,
    Right,
}

/// Per-zone body health (1 = intact, 0 = destroyed).
#[derive(Clone, Debug)]
pub struct DamageModel {
    pub front: f64,
    pub rear: f64,
    pub left: f64,
    pub right: f64,
    /// Total severity absorbed (stats).
    pub impacts: f64,
}

impl Default for DamageModel {
    fn default() -> Self {
        DamageModel { front: 1.0, rear: 1.0, left: 1.0, right: 1.0, impacts: 0.0 }
    }
}

impl DamageModel {
    /// Overall body health 0..1.
    pub fn body(&self) -> f64 {
        (self.front + self.rear + self.left + self.right) / 4.0
    }

    pub fn totaled(&self) -> bool {
        self.body() < 0.2 || self.front < 0.05
    }

    /// Apply an impact at body-frame point (x forward) with `severity` = Δv
    /// (m/s). Returns the zone hit and the health lost.
    pub fn hit(&mut self, local_x: f64, local_y: f64, half_len: f64, half_wid: f64, severity: f64) -> (DamageZone, f64) {
        let nx = local_x / half_len;
        let ny = local_y / half_wid;
        let zone = if nx.abs() >= ny.abs() {
            if nx >= 0.0 {
                DamageZone::Front
            } else {
                DamageZone::Rear
            }
        } else if ny >= 0.0 {
            DamageZone::Left
        } else {
            DamageZone::Right
        };
        // Small bumps (< 1.5 m/s) do nothing; 15 m/s wrecks a zone.
        let lost = clamp((severity - 1.5) / 14.0, 0.0, 1.0);
        if lost > 0.0 {
            let z = match zone {
                DamageZone::Front => &mut self.front,
                DamageZone::Rear => &mut self.rear,
                DamageZone::Left => &mut self.left,
                DamageZone::Right => &mut self.right,
            };
            *z = (*z - lost).max(0.0);
            self.impacts += severity;
        }
        (zone, lost)
    }

    pub fn repair(&mut self) {
        self.front = 1.0;
        self.rear = 1.0;
        self.left = 1.0;
        self.right = 1.0;
    }
}
