//! Anger, grudges and moods behind road rage.

use super::personality::Personality;
use crate::ids::VehicleId;
use crate::math::scalar::clamp;

/// Mood with hysteresis thresholds on anger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mood {
    Calm,
    Irritated,
    Angry,
    Furious,
}

/// Something another road user did to a driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OffenseKind {
    CutOff,
    Tailgating,
    HonkedAt,
    LongHonk,
    HighBeams,
    Blocked,
    Collision,
    NearMiss,
    RudeGesture,
    BrakeCheck,
    SlowDriver,
    LongWait,
}

impl OffenseKind {
    /// All kinds (documentation / tables).
    pub const ALL: [OffenseKind; 12] = [
        OffenseKind::CutOff,
        OffenseKind::Tailgating,
        OffenseKind::HonkedAt,
        OffenseKind::LongHonk,
        OffenseKind::HighBeams,
        OffenseKind::Blocked,
        OffenseKind::Collision,
        OffenseKind::NearMiss,
        OffenseKind::RudeGesture,
        OffenseKind::BrakeCheck,
        OffenseKind::SlowDriver,
        OffenseKind::LongWait,
    ];

    /// Base anger per offense (before temper/stress scaling).
    pub fn base_anger(self) -> f64 {
        match self {
            OffenseKind::CutOff => 18.0,
            OffenseKind::Tailgating => 4.0,
            OffenseKind::HonkedAt => 7.0,
            OffenseKind::LongHonk => 14.0,
            OffenseKind::HighBeams => 7.0,
            OffenseKind::Blocked => 2.5,
            OffenseKind::Collision => 38.0,
            OffenseKind::NearMiss => 14.0,
            OffenseKind::RudeGesture => 28.0,
            OffenseKind::BrakeCheck => 22.0,
            OffenseKind::SlowDriver => 1.5,
            OffenseKind::LongWait => 0.6,
        }
    }

    /// Continuous offenses do not produce `offense` events or reactions.
    pub fn is_continuous(self) -> bool {
        matches!(
            self,
            OffenseKind::Tailgating
                | OffenseKind::Blocked
                | OffenseKind::SlowDriver
                | OffenseKind::LongWait
        )
    }
}

/// Base anger per offense (same as [`OffenseKind::base_anger`]).
pub fn offense_anger(kind: OffenseKind) -> f64 {
    kind.base_anger()
}

#[derive(Clone, Copy, Debug)]
struct Grudge {
    id: VehicleId,
    amount: f64,
    last: f64,
}

/// Anger model behind road rage. Offenses add anger (scaled by temper and
/// stress) and a grudge against the offender; anger decays with patience.
/// Mood thresholds use hysteresis so drivers don't flicker.
#[derive(Clone, Debug)]
pub struct Emotion {
    pub p: Personality,
    pub anger: f64,
    /// 0..1 background stress (congestion, waiting).
    pub stress: f64,
    pub mood: Mood,
    grudges: Vec<Grudge>,
    /// Offender currently hated the most.
    pub target: Option<VehicleId>,
    pub last_offense: f64,
    pub last_offender: Option<VehicleId>,
    pub last_kind: Option<OffenseKind>,
    forgiven: Vec<Grudge>,
}

impl Emotion {
    pub fn new(p: Personality) -> Self {
        Emotion {
            p,
            anger: 0.0,
            stress: 0.0,
            mood: Mood::Calm,
            grudges: Vec::new(),
            target: None,
            last_offense: -100.0,
            last_offender: None,
            last_kind: None,
            forgiven: Vec::new(),
        }
    }

    /// Apply an offense (`offender` None = situational: no grudge). Returns the anger gained.
    pub fn offend(
        &mut self,
        offender: Option<VehicleId>,
        kind: OffenseKind,
        scale: f64,
        now: f64,
    ) -> f64 {
        let mut forgiveness = 1.0;
        if let Some(o) = offender {
            for f in &self.forgiven {
                if f.id == o && now - f.last < 20.0 {
                    forgiveness = 0.4;
                }
            }
        }
        let gain =
            kind.base_anger() * scale * self.p.temper * (1.0 + self.stress * 0.6) * forgiveness;
        self.anger = clamp(self.anger + gain, 0.0, 100.0);
        let Some(offender) = offender else {
            return gain;
        };
        let gi = match self.grudges.iter().rposition(|g| g.id == offender) {
            Some(i) => i,
            None => {
                self.grudges.push(Grudge {
                    id: offender,
                    amount: 0.0,
                    last: now,
                });
                self.grudges.len() - 1
            }
        };
        let g = &mut self.grudges[gi];
        g.amount = (g.amount + gain).min(100.0);
        g.last = now;
        self.last_offense = now;
        self.last_offender = Some(offender);
        self.last_kind = Some(kind);
        self.pick_target();
        gain
    }

    /// Offender apologised (wave / hazard "thank you"). Returns anger removed.
    pub fn apologize(&mut self, offender: VehicleId, now: f64) -> f64 {
        let Some(gi) = self.grudges.iter().rposition(|g| g.id == offender) else {
            return 0.0;
        };
        let recent = now - self.grudges[gi].last < 10.0;
        let base = if recent { 26.0 } else { 10.0 };
        let calmed = base * (1.2 - self.p.aggression * 0.7);
        let before = self.anger;
        self.anger = clamp(self.anger - calmed, 0.0, 100.0);
        let g = &mut self.grudges[gi];
        g.amount = (g.amount - calmed * 1.5).max(0.0);
        self.forgiven.push(Grudge {
            id: offender,
            amount: 0.0,
            last: now,
        });
        if self.forgiven.len() > 6 {
            self.forgiven.remove(0);
        }
        self.pick_target();
        before - self.anger
    }

    /// Grudge held against a vehicle (0 = none).
    pub fn grudge_against(&self, id: VehicleId) -> f64 {
        self.grudges
            .iter()
            .rev()
            .find(|g| g.id == id)
            .map_or(0.0, |g| g.amount)
    }

    fn pick_target(&mut self) {
        let mut best = 15.0;
        self.target = None;
        for g in &self.grudges {
            if g.amount > best {
                best = g.amount;
                self.target = Some(g.id);
            }
        }
    }

    pub fn update(&mut self, dt: f64, now: f64, congestion: f64) {
        self.stress += (clamp(congestion, 0.0, 1.0) - self.stress) * (dt * 0.05).min(1.0);
        let since_offense = now - self.last_offense;
        let decay = (0.6 + self.p.patience * 3.2)
            * (if since_offense > 6.0 { 1.6 } else { 0.6 })
            * (1.0 - self.stress * 0.4);
        self.anger = (self.anger - decay * dt).max(0.0);
        for g in self.grudges.iter_mut() {
            g.amount -= decay * 0.8 * dt;
        }
        self.grudges.retain(|g| g.amount > 0.0);
        if self.target.is_some() {
            self.pick_target();
        }
        let a = self.anger;
        self.mood = match self.mood {
            Mood::Calm if a > 22.0 => Mood::Irritated,
            Mood::Irritated if a > 48.0 => Mood::Angry,
            Mood::Irritated if a < 14.0 => Mood::Calm,
            Mood::Angry if a > 74.0 => Mood::Furious,
            Mood::Angry if a < 38.0 => Mood::Irritated,
            Mood::Furious if a < 60.0 => Mood::Angry,
            m => m,
        };
    }

    /// Is this driver willing to escalate into physical road rage?
    pub fn can_rage(&self) -> bool {
        self.mood == Mood::Furious && self.p.aggression > 0.55
    }
}
