//! Traffic violations, fines, points and the player's driving record.

use std::collections::HashMap;

/// A chargeable traffic violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ViolationKind {
    RedLight,
    RightOnRedNoStop,
    StopSign,
    SpeedingMinor,
    SpeedingMajor,
    SpeedingExtreme,
    WrongWay,
    Sidewalk,
    Collision,
    HitAndRun,
    NoSignalLane,
    NoSignalTurn,
    IllegalTurn,
    NoHeadlights,
    Seatbelt,
    FailYieldEmergency,
    Tailgating,
    Reckless,
    BlockingBox,
    HornAbuse,
    Evading,
    FailYieldPedestrian,
    HitPedestrian,
}

/// Fine, points, heat and cooldown of a violation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViolationDef {
    pub label: &'static str,
    /// Fine in eurodollars.
    pub fine: f64,
    /// License points.
    pub points: u32,
    /// Heat added when police see it.
    pub heat: f64,
    /// Seconds before the same violation can be charged again.
    pub cooldown: f64,
}

impl ViolationKind {
    pub const ALL: [ViolationKind; 23] = [
        ViolationKind::RedLight,
        ViolationKind::RightOnRedNoStop,
        ViolationKind::StopSign,
        ViolationKind::SpeedingMinor,
        ViolationKind::SpeedingMajor,
        ViolationKind::SpeedingExtreme,
        ViolationKind::WrongWay,
        ViolationKind::Sidewalk,
        ViolationKind::Collision,
        ViolationKind::HitAndRun,
        ViolationKind::NoSignalLane,
        ViolationKind::NoSignalTurn,
        ViolationKind::IllegalTurn,
        ViolationKind::NoHeadlights,
        ViolationKind::Seatbelt,
        ViolationKind::FailYieldEmergency,
        ViolationKind::Tailgating,
        ViolationKind::Reckless,
        ViolationKind::BlockingBox,
        ViolationKind::HornAbuse,
        ViolationKind::Evading,
        ViolationKind::FailYieldPedestrian,
        ViolationKind::HitPedestrian,
    ];

    /// The violation table (same values as `law/violations.ts`).
    pub fn def(self) -> ViolationDef {
        let d = |label, fine, points, heat, cooldown| ViolationDef { label, fine, points, heat, cooldown };
        match self {
            ViolationKind::RedLight => d("Ran a red light", 400.0, 3, 1.0, 4.0),
            ViolationKind::RightOnRedNoStop => d("Right on red without stopping", 150.0, 1, 1.0, 4.0),
            ViolationKind::StopSign => d("Failed to stop at stop sign", 200.0, 2, 1.0, 4.0),
            ViolationKind::SpeedingMinor => d("Speeding (10–25% over)", 120.0, 1, 1.0, 25.0),
            ViolationKind::SpeedingMajor => d("Speeding (25–50% over)", 350.0, 3, 1.0, 25.0),
            ViolationKind::SpeedingExtreme => d("Excessive speed (50%+ over)", 900.0, 6, 2.0, 25.0),
            ViolationKind::WrongWay => d("Driving the wrong way", 500.0, 4, 2.0, 20.0),
            ViolationKind::Sidewalk => d("Driving on the sidewalk", 250.0, 2, 1.0, 15.0),
            ViolationKind::Collision => d("Careless driving (collision)", 300.0, 3, 1.0, 3.0),
            ViolationKind::HitAndRun => d("Leaving the scene of a collision", 1500.0, 6, 3.0, 30.0),
            ViolationKind::NoSignalLane => d("Lane change without signalling", 60.0, 0, 1.0, 15.0),
            ViolationKind::NoSignalTurn => d("Turn without signalling", 80.0, 1, 1.0, 10.0),
            ViolationKind::IllegalTurn => d("Illegal turn", 200.0, 2, 1.0, 8.0),
            ViolationKind::NoHeadlights => d("No headlights after dark", 90.0, 1, 1.0, 60.0),
            ViolationKind::Seatbelt => d("Not wearing a seatbelt", 70.0, 0, 1.0, 60.0),
            ViolationKind::FailYieldEmergency => d("Failure to yield to emergency vehicle", 450.0, 3, 2.0, 30.0),
            ViolationKind::Tailgating => d("Following too closely", 120.0, 2, 1.0, 20.0),
            ViolationKind::Reckless => d("Reckless driving", 800.0, 5, 2.0, 20.0),
            ViolationKind::BlockingBox => d("Blocking the intersection", 150.0, 1, 1.0, 20.0),
            ViolationKind::HornAbuse => d("Unnecessary use of horn", 50.0, 0, 1.0, 30.0),
            ViolationKind::Evading => d("Evading police", 2500.0, 8, 3.0, 60.0),
            ViolationKind::FailYieldPedestrian => d("Failure to yield to a pedestrian", 350.0, 3, 1.0, 8.0),
            ViolationKind::HitPedestrian => d("Hit a pedestrian", 5000.0, 8, 3.0, 5.0),
        }
    }
}

/// Who saw the violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Witness {
    Police,
    Camera,
    None,
}

const SPEED_TIERS: [ViolationKind; 3] = [ViolationKind::SpeedingMinor, ViolationKind::SpeedingMajor, ViolationKind::SpeedingExtreme];

/// One logged violation.
#[derive(Clone, Debug, PartialEq)]
pub struct ViolationRecord {
    pub kind: ViolationKind,
    pub label: &'static str,
    pub time: f64,
    pub fine: f64,
    pub points: u32,
    pub witness: Witness,
    /// Fines become due when the ticket is written (police) or immediately (camera).
    pub paid: bool,
    pub x: f64,
    pub y: f64,
}

/// Summary of a written ticket.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TicketSummary {
    pub total: f64,
    pub points: u32,
    pub count: usize,
}

/// The player's driving record and wallet: history, pending police-witnessed
/// violations, licence points (12 suspends), heat and clean-driving credit.
#[derive(Clone, Debug)]
pub struct DrivingRecord {
    pub money: f64,
    pub points: u32,
    pub max_points: u32,
    pub history: Vec<ViolationRecord>,
    /// Police-witnessed violations awaiting a ticket (copies of history entries).
    pub pending: Vec<ViolationRecord>,
    pub heat: f64,
    /// Distance (m) driven without any violation.
    pub clean_distance: f64,
    pub tickets_issued: u32,
    pub total_fines: f64,
    last_charged: HashMap<ViolationKind, f64>,
}

impl Default for DrivingRecord {
    fn default() -> Self {
        DrivingRecord {
            money: 5000.0,
            points: 0,
            max_points: 12,
            history: Vec::new(),
            pending: Vec::new(),
            heat: 0.0,
            clean_distance: 0.0,
            tickets_issued: 0,
            total_fines: 0.0,
            last_charged: HashMap::new(),
        }
    }
}

impl DrivingRecord {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn suspended(&self) -> bool {
        self.points >= self.max_points
    }

    /// Is this violation off cooldown? Speeding tiers share one cooldown; a
    /// higher tier may upgrade after 8 s.
    pub fn can_charge(&self, kind: ViolationKind, now: f64) -> bool {
        if let Some(tier) = SPEED_TIERS.iter().position(|&k| k == kind) {
            for (i, k) in SPEED_TIERS.iter().enumerate() {
                let Some(&t) = self.last_charged.get(k) else { continue };
                let age = now - t;
                if i >= tier && age < kind.def().cooldown {
                    return false;
                }
                if i < tier && age < 8.0 {
                    return false;
                }
            }
            return true;
        }
        match self.last_charged.get(&kind) {
            None => true,
            Some(&last) => now - last >= kind.def().cooldown,
        }
    }

    /// Log a violation. Camera violations are charged at once, police ones
    /// wait for a ticket. Returns a copy of the record.
    pub fn add(&mut self, kind: ViolationKind, now: f64, witness: Witness, x: f64, y: f64) -> ViolationRecord {
        let d = kind.def();
        self.last_charged.insert(kind, now);
        let mut rec = ViolationRecord { kind, label: d.label, time: now, fine: d.fine, points: d.points, witness, paid: false, x, y };
        self.clean_distance = 0.0;
        match witness {
            Witness::Camera => {
                let total = rec.fine;
                rec.paid = true;
                self.points += rec.points;
                self.money -= total;
                self.total_fines += total;
            }
            Witness::Police => self.pending.push(rec.clone()),
            Witness::None => {}
        }
        self.history.push(rec.clone());
        if self.history.len() > 200 {
            self.history.remove(0);
        }
        rec
    }

    /// Charge records (pay fines, add points). Returns the total fine.
    pub fn charge(&mut self, recs: &mut [ViolationRecord]) -> f64 {
        let mut total = 0.0;
        for r in recs.iter_mut() {
            if r.paid {
                continue;
            }
            r.paid = true;
            total += r.fine;
            self.points += r.points;
        }
        self.money -= total;
        self.total_fines += total;
        total
    }

    /// Ticket for all pending police-witnessed violations.
    pub fn issue_ticket(&mut self) -> TicketSummary {
        let mut recs = std::mem::take(&mut self.pending);
        let pts = recs.iter().map(|r| r.points).sum();
        let total = self.charge(&mut recs);
        // Mark the matching history entries as paid.
        for r in &recs {
            if let Some(h) = self.history.iter_mut().rev().find(|h| !h.paid && h.kind == r.kind && h.time == r.time) {
                h.paid = true;
            }
        }
        self.tickets_issued += 1;
        TicketSummary { total, points: pts, count: recs.len() }
    }

    /// Points expire slowly with clean driving (1 point per 2 km).
    pub fn drive_clean(&mut self, meters: f64) {
        self.clean_distance += meters;
        if self.clean_distance > 2000.0 && self.points > 0 {
            self.points -= 1;
            self.clean_distance = 0.0;
        }
    }
}
