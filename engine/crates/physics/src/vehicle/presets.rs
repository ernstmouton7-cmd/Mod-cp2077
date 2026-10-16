//! Vehicle presets matching the traffic simulation's vehicle classes.

use super::powertrain::*;
use super::tyre::TyreDesc;
use super::*;

/// Compact parameter sheet from which a full `VehicleDesc` is derived.
#[derive(Clone, Debug)]
pub struct VehicleSpec {
    pub name: &'static str,
    pub length: f32,
    pub width: f32,
    pub height: f32,
    pub wheelbase: f32,
    /// Fraction of static weight on the front axle.
    pub front_weight: f32,
    pub cg_height: f32,
    pub mass: f32,
    pub wheel_radius: f32,
    pub idle_rpm: f32,
    pub redline: f32,
    pub torque_curve: &'static [(f32, f32)],
    pub engine_brake: f32,
    pub gears: &'static [f32],
    pub reverse: f32,
    pub final_drive: f32,
    pub shift_time: f32,
    pub layout: DriveLayout,
    pub brake_torque: f32,
    pub brake_bias: f32,
    pub grip: f32,
    pub drag_area: f32,
    pub roll_resist: f32,
    pub max_steer: f32,
    /// Suspension natural frequencies (Hz) front/rear.
    pub ride_hz: (f32, f32),
    pub travel: f32,
    pub ground_clearance: f32,
    /// Anti-roll stiffness as a fraction of the corner spring rate (front, rear).
    pub arb: (f32, f32),
    pub rear_diff: Differential,
    pub downforce: (f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VehicleClass {
    Compact,
    Sedan,
    Sports,
    Muscle,
    Suv,
    Van,
    Truck,
    Bus,
    Taxi,
    Police,
    Ambulance,
}

impl VehicleClass {
    pub const ALL: [VehicleClass; 11] = [
        VehicleClass::Compact,
        VehicleClass::Sedan,
        VehicleClass::Sports,
        VehicleClass::Muscle,
        VehicleClass::Suv,
        VehicleClass::Van,
        VehicleClass::Truck,
        VehicleClass::Bus,
        VehicleClass::Taxi,
        VehicleClass::Police,
        VehicleClass::Ambulance,
    ];

    pub fn from_name(s: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|c| c.name().eq_ignore_ascii_case(s))
    }

    pub fn name(self) -> &'static str {
        match self {
            VehicleClass::Compact => "compact",
            VehicleClass::Sedan => "sedan",
            VehicleClass::Sports => "sports",
            VehicleClass::Muscle => "muscle",
            VehicleClass::Suv => "suv",
            VehicleClass::Van => "van",
            VehicleClass::Truck => "truck",
            VehicleClass::Bus => "bus",
            VehicleClass::Taxi => "taxi",
            VehicleClass::Police => "police",
            VehicleClass::Ambulance => "ambulance",
        }
    }

    pub fn spec(self) -> VehicleSpec {
        use DriveLayout::*;
        let open = Differential::Open;
        let lsd = Differential::Limited {
            preload: 80.0,
            ramp: 0.35,
        };
        match self {
            VehicleClass::Compact => VehicleSpec {
                name: "Compact",
                length: 3.9,
                width: 1.72,
                height: 1.45,
                wheelbase: 2.5,
                front_weight: 0.6,
                cg_height: 0.5,
                mass: 1100.0,
                wheel_radius: 0.29,
                idle_rpm: 800.0,
                redline: 6500.0,
                torque_curve: &[
                    (800.0, 90.0),
                    (2000.0, 125.0),
                    (3500.0, 140.0),
                    (5000.0, 132.0),
                    (6500.0, 105.0),
                ],
                engine_brake: 35.0,
                gears: &[3.5, 2.0, 1.35, 1.0, 0.8],
                reverse: 3.3,
                final_drive: 4.1,
                shift_time: 0.35,
                layout: Fwd,
                brake_torque: 4200.0,
                brake_bias: 0.68,
                grip: 1.0,
                drag_area: 0.66,
                roll_resist: 0.013,
                max_steer: 0.62,
                ride_hz: (1.45, 1.6),
                travel: 0.2,
                ground_clearance: 0.15,
                arb: (0.35, 0.15),
                rear_diff: open,
                downforce: (0.0, 0.0),
            },
            VehicleClass::Sedan => VehicleSpec {
                name: "Sedan",
                length: 4.8,
                width: 1.85,
                height: 1.45,
                wheelbase: 2.85,
                front_weight: 0.57,
                cg_height: 0.52,
                mass: 1550.0,
                wheel_radius: 0.32,
                idle_rpm: 750.0,
                redline: 6300.0,
                torque_curve: &[
                    (750.0, 160.0),
                    (2000.0, 240.0),
                    (4000.0, 260.0),
                    (5500.0, 240.0),
                    (6300.0, 200.0),
                ],
                engine_brake: 45.0,
                gears: &[3.6, 2.2, 1.5, 1.15, 0.92, 0.75],
                reverse: 3.4,
                final_drive: 3.7,
                shift_time: 0.3,
                layout: Fwd,
                brake_torque: 6200.0,
                brake_bias: 0.66,
                grip: 1.0,
                drag_area: 0.62,
                roll_resist: 0.012,
                max_steer: 0.6,
                ride_hz: (1.4, 1.55),
                travel: 0.21,
                ground_clearance: 0.15,
                arb: (0.4, 0.2),
                rear_diff: open,
                downforce: (0.0, 0.0),
            },
            VehicleClass::Sports => VehicleSpec {
                name: "Sports Coupe",
                length: 4.5,
                width: 1.95,
                height: 1.22,
                wheelbase: 2.65,
                front_weight: 0.48,
                cg_height: 0.42,
                mass: 1380.0,
                wheel_radius: 0.34,
                idle_rpm: 900.0,
                redline: 8000.0,
                torque_curve: &[
                    (900.0, 250.0),
                    (3000.0, 420.0),
                    (5500.0, 470.0),
                    (7000.0, 440.0),
                    (8000.0, 380.0),
                ],
                engine_brake: 60.0,
                gears: &[3.1, 2.1, 1.6, 1.28, 1.05, 0.86],
                reverse: 3.0,
                final_drive: 3.6,
                shift_time: 0.12,
                layout: Rwd,
                brake_torque: 9000.0,
                brake_bias: 0.62,
                grip: 1.15,
                drag_area: 0.55,
                roll_resist: 0.011,
                max_steer: 0.56,
                ride_hz: (1.9, 2.1),
                travel: 0.15,
                ground_clearance: 0.11,
                arb: (0.6, 0.35),
                rear_diff: lsd,
                downforce: (0.25, 0.35),
            },
            VehicleClass::Muscle => VehicleSpec {
                name: "Muscle Car",
                length: 4.9,
                width: 1.95,
                height: 1.35,
                wheelbase: 2.9,
                front_weight: 0.55,
                cg_height: 0.5,
                mass: 1650.0,
                wheel_radius: 0.34,
                idle_rpm: 700.0,
                redline: 6200.0,
                torque_curve: &[
                    (700.0, 380.0),
                    (2500.0, 560.0),
                    (4000.0, 600.0),
                    (5500.0, 540.0),
                    (6200.0, 470.0),
                ],
                engine_brake: 70.0,
                gears: &[2.9, 1.9, 1.35, 1.0, 0.8],
                reverse: 2.9,
                final_drive: 3.3,
                shift_time: 0.25,
                layout: Rwd,
                brake_torque: 7600.0,
                brake_bias: 0.64,
                grip: 1.0,
                drag_area: 0.72,
                roll_resist: 0.013,
                max_steer: 0.58,
                ride_hz: (1.5, 1.6),
                travel: 0.19,
                ground_clearance: 0.13,
                arb: (0.45, 0.15),
                rear_diff: lsd,
                downforce: (0.0, 0.05),
            },
            VehicleClass::Suv => VehicleSpec {
                name: "SUV",
                length: 4.95,
                width: 1.98,
                height: 1.8,
                wheelbase: 2.95,
                front_weight: 0.55,
                cg_height: 0.72,
                mass: 2150.0,
                wheel_radius: 0.37,
                idle_rpm: 700.0,
                redline: 6000.0,
                torque_curve: &[
                    (700.0, 260.0),
                    (2000.0, 380.0),
                    (3500.0, 420.0),
                    (5000.0, 380.0),
                    (6000.0, 320.0),
                ],
                engine_brake: 60.0,
                gears: &[4.0, 2.4, 1.6, 1.2, 1.0, 0.8],
                reverse: 3.6,
                final_drive: 3.5,
                shift_time: 0.3,
                layout: Awd { front: 0.4 },
                brake_torque: 9500.0,
                brake_bias: 0.65,
                grip: 0.95,
                drag_area: 1.0,
                roll_resist: 0.015,
                max_steer: 0.6,
                ride_hz: (1.3, 1.45),
                travel: 0.26,
                ground_clearance: 0.21,
                arb: (0.5, 0.25),
                rear_diff: open,
                downforce: (0.0, 0.0),
            },
            VehicleClass::Van => VehicleSpec {
                name: "Van",
                length: 5.3,
                width: 2.0,
                height: 2.2,
                wheelbase: 3.3,
                front_weight: 0.55,
                cg_height: 0.85,
                mass: 2400.0,
                wheel_radius: 0.36,
                idle_rpm: 750.0,
                redline: 5000.0,
                torque_curve: &[
                    (750.0, 260.0),
                    (1800.0, 380.0),
                    (3000.0, 390.0),
                    (4200.0, 340.0),
                    (5000.0, 280.0),
                ],
                engine_brake: 60.0,
                gears: &[4.2, 2.5, 1.6, 1.2, 1.0, 0.82],
                reverse: 3.8,
                final_drive: 3.9,
                shift_time: 0.35,
                layout: Rwd,
                brake_torque: 9500.0,
                brake_bias: 0.65,
                grip: 0.9,
                drag_area: 1.4,
                roll_resist: 0.016,
                max_steer: 0.62,
                ride_hz: (1.35, 1.6),
                travel: 0.22,
                ground_clearance: 0.17,
                arb: (0.45, 0.2),
                rear_diff: open,
                downforce: (0.0, 0.0),
            },
            VehicleClass::Truck => VehicleSpec {
                name: "Box Truck",
                length: 7.8,
                width: 2.45,
                height: 3.4,
                wheelbase: 4.8,
                front_weight: 0.5,
                cg_height: 1.2,
                mass: 7500.0,
                wheel_radius: 0.5,
                idle_rpm: 600.0,
                redline: 2600.0,
                torque_curve: &[
                    (600.0, 900.0),
                    (1200.0, 1400.0),
                    (1800.0, 1450.0),
                    (2300.0, 1200.0),
                    (2600.0, 1000.0),
                ],
                engine_brake: 400.0,
                gears: &[6.5, 4.0, 2.6, 1.8, 1.3, 1.0],
                reverse: 6.0,
                final_drive: 4.1,
                shift_time: 0.6,
                layout: Rwd,
                brake_torque: 38000.0,
                brake_bias: 0.55,
                grip: 0.85,
                drag_area: 5.5,
                roll_resist: 0.008,
                max_steer: 0.6,
                ride_hz: (1.4, 1.5),
                travel: 0.22,
                ground_clearance: 0.25,
                arb: (0.6, 0.5),
                rear_diff: open,
                downforce: (0.0, 0.0),
            },
            VehicleClass::Bus => VehicleSpec {
                name: "City Bus",
                length: 12.0,
                width: 2.55,
                height: 3.2,
                wheelbase: 6.2,
                front_weight: 0.38,
                cg_height: 1.1,
                mass: 12000.0,
                wheel_radius: 0.5,
                idle_rpm: 600.0,
                redline: 2400.0,
                torque_curve: &[
                    (600.0, 1200.0),
                    (1100.0, 1700.0),
                    (1600.0, 1750.0),
                    (2100.0, 1500.0),
                    (2400.0, 1250.0),
                ],
                engine_brake: 500.0,
                gears: &[4.7, 2.6, 1.6, 1.15, 1.0, 0.8],
                reverse: 4.5,
                final_drive: 5.2,
                shift_time: 0.5,
                layout: Rwd,
                brake_torque: 60000.0,
                brake_bias: 0.5,
                grip: 0.85,
                drag_area: 7.0,
                roll_resist: 0.008,
                max_steer: 0.62,
                ride_hz: (1.2, 1.3),
                travel: 0.2,
                ground_clearance: 0.25,
                arb: (0.6, 0.5),
                rear_diff: open,
                downforce: (0.0, 0.0),
            },
            VehicleClass::Taxi => VehicleSpec {
                name: "Cab",
                length: 4.85,
                width: 1.9,
                height: 1.5,
                wheelbase: 2.85,
                front_weight: 0.56,
                cg_height: 0.52,
                mass: 1600.0,
                wheel_radius: 0.32,
                idle_rpm: 750.0,
                redline: 6300.0,
                torque_curve: &[
                    (750.0, 200.0),
                    (2000.0, 300.0),
                    (4000.0, 320.0),
                    (5500.0, 290.0),
                    (6300.0, 250.0),
                ],
                engine_brake: 45.0,
                gears: &[3.6, 2.2, 1.5, 1.15, 0.92, 0.75],
                reverse: 3.4,
                final_drive: 3.7,
                shift_time: 0.25,
                layout: Awd { front: 0.45 },
                brake_torque: 6600.0,
                brake_bias: 0.66,
                grip: 1.02,
                drag_area: 0.6,
                roll_resist: 0.012,
                max_steer: 0.6,
                ride_hz: (1.45, 1.6),
                travel: 0.2,
                ground_clearance: 0.15,
                arb: (0.4, 0.25),
                rear_diff: open,
                downforce: (0.0, 0.0),
            },
            VehicleClass::Police => VehicleSpec {
                name: "Patrol Interceptor",
                length: 5.0,
                width: 1.95,
                height: 1.5,
                wheelbase: 2.95,
                front_weight: 0.54,
                cg_height: 0.5,
                mass: 1800.0,
                wheel_radius: 0.34,
                idle_rpm: 750.0,
                redline: 6800.0,
                torque_curve: &[
                    (750.0, 300.0),
                    (2500.0, 470.0),
                    (4500.0, 500.0),
                    (6000.0, 460.0),
                    (6800.0, 400.0),
                ],
                engine_brake: 55.0,
                gears: &[3.2, 2.1, 1.5, 1.15, 0.92, 0.75],
                reverse: 3.2,
                final_drive: 3.5,
                shift_time: 0.18,
                layout: Awd { front: 0.4 },
                brake_torque: 9800.0,
                brake_bias: 0.63,
                grip: 1.1,
                drag_area: 0.66,
                roll_resist: 0.012,
                max_steer: 0.6,
                ride_hz: (1.7, 1.85),
                travel: 0.18,
                ground_clearance: 0.14,
                arb: (0.55, 0.3),
                rear_diff: lsd,
                downforce: (0.05, 0.1),
            },
            VehicleClass::Ambulance => VehicleSpec {
                name: "Ambulance",
                length: 6.2,
                width: 2.2,
                height: 2.7,
                wheelbase: 3.7,
                front_weight: 0.5,
                cg_height: 1.0,
                mass: 3800.0,
                wheel_radius: 0.4,
                idle_rpm: 700.0,
                redline: 5200.0,
                torque_curve: &[
                    (700.0, 420.0),
                    (1800.0, 600.0),
                    (3000.0, 640.0),
                    (4200.0, 560.0),
                    (5200.0, 460.0),
                ],
                engine_brake: 90.0,
                gears: &[4.2, 2.6, 1.7, 1.25, 1.0, 0.8],
                reverse: 3.8,
                final_drive: 3.9,
                shift_time: 0.35,
                layout: Rwd,
                brake_torque: 16000.0,
                brake_bias: 0.6,
                grip: 0.95,
                drag_area: 2.4,
                roll_resist: 0.013,
                max_steer: 0.62,
                ride_hz: (1.4, 1.6),
                travel: 0.22,
                ground_clearance: 0.2,
                arb: (0.5, 0.35),
                rear_diff: open,
                downforce: (0.0, 0.0),
            },
        }
    }

    pub fn desc(self) -> VehicleDesc {
        VehicleDesc::from_spec(&self.spec())
    }
}

impl VehicleDesc {
    /// Derive suspension, tyres, inertia and a chassis hull from a parameter sheet.
    pub fn from_spec(s: &VehicleSpec) -> Self {
        let g = 9.81;
        let lf = s.wheelbase * (1.0 - s.front_weight); // COM to front axle
        let half_wb = s.wheelbase * 0.5;
        let com = Vec3::new(0.0, s.cg_height, -half_wb + lf);
        let track = s.width - 0.25 - s.wheel_radius * 0.15;
        let corner_front = s.mass * s.front_weight * 0.5;
        let corner_rear = s.mass * (1.0 - s.front_weight) * 0.5;
        let mut wheels = Vec::new();
        let wheel_inertia = 0.55
            * (14.0 + 40.0 * s.wheel_radius)
            * s.wheel_radius
            * s.wheel_radius
            * (s.mass / 1500.0).clamp(0.8, 4.0);
        for (front, corner, hz) in [
            (true, corner_front, s.ride_hz.0),
            (false, corner_rear, s.ride_hz.1),
        ] {
            let k = corner * (2.0 * std::f32::consts::PI * hz).powi(2);
            let crit = 2.0 * (k * corner).sqrt();
            // Static compression from full droop; hardpoint sits so the origin rests on the ground.
            let static_comp = corner * g / k;
            let travel = s.travel.max(static_comp * 1.6);
            let hy = s.wheel_radius + (travel - static_comp);
            let z = if front { -half_wb } else { half_wb };
            let brake_share = if front {
                s.brake_bias * 0.5
            } else {
                (1.0 - s.brake_bias) * 0.5
            };
            let driven = match s.layout {
                DriveLayout::Fwd => front,
                DriveLayout::Rwd => !front,
                DriveLayout::Awd { .. } => true,
            };
            for side in [-1.0f32, 1.0] {
                wheels.push(WheelDesc {
                    hardpoint: Vec3::new(side * track * 0.5, hy, z),
                    radius: s.wheel_radius,
                    width: 0.18 + s.wheel_radius * 0.15,
                    inertia: wheel_inertia,
                    travel,
                    spring: k,
                    bump_damping: crit * 0.28,
                    rebound_damping: crit * 0.42,
                    steer: if front { 1.0 } else { 0.0 },
                    driven,
                    brake_share,
                    handbrake: !front,
                    tyre: TyreDesc {
                        mu: s.grip,
                        nominal_load: corner * g,
                        rolling_resistance: s.roll_resist,
                        peak_slip_angle: if s.mass > 4000.0 { 0.17 } else { 0.13 },
                        ..Default::default()
                    },
                    toe: 0.0,
                });
            }
        }
        let corner_k = |i: usize| wheels[i].spring;
        let anti_roll = vec![
            AntiRollBar {
                left: 0,
                right: 1,
                stiffness: corner_k(0) * s.arb.0,
            },
            AntiRollBar {
                left: 2,
                right: 3,
                stiffness: corner_k(2) * s.arb.1,
            },
        ];
        let peak = s.torque_curve.iter().fold(0.0f32, |m, p| m.max(p.1));
        let engine = EngineDesc {
            torque_curve: s.torque_curve.to_vec(),
            idle_rpm: s.idle_rpm,
            redline: s.redline,
            stall_rpm: s.idle_rpm * 0.45,
            inertia: (0.08 + peak / 2500.0).min(3.0),
            engine_brake: s.engine_brake,
        };
        let gearbox = GearboxDesc {
            ratios: s.gears.to_vec(),
            reverse: s.reverse,
            final_drive: s.final_drive,
            shift_time: s.shift_time,
            efficiency: 0.9,
            clutch_capacity: 1.6,
        };
        // Inertia of a hollow-ish box (mass concentrated in the floor and ends).
        let (l, w, h) = (s.length, s.width, s.height * 0.75);
        let inertia = Vec3::new(
            s.mass * (h * h + l * l) / 12.0 * 0.85,
            s.mass * (w * w + l * l) / 12.0 * 0.85,
            s.mass * (w * w + h * h) / 12.0 * 0.85,
        );
        VehicleDesc {
            name: s.name.to_string(),
            mass: s.mass,
            com,
            inertia,
            hull: car_hull(s),
            wheels,
            anti_roll,
            engine,
            gearbox,
            layout: s.layout,
            front_diff: Differential::Open,
            rear_diff: s.rear_diff,
            brake_torque: s.brake_torque,
            handbrake_torque: s.brake_torque * 0.35,
            max_steer: s.max_steer,
            steer_speed: 2.4,
            aero: Aero {
                drag_area: s.drag_area,
                lift_front: s.downforce.0,
                lift_rear: s.downforce.1,
            },
            wheelbase: s.wheelbase,
            track,
            size: Vec3::new(s.length, s.width, s.height),
        }
    }
}

/// A rounded lower body plus a narrower greenhouse, centred on the wheelbase.
fn car_hull(s: &VehicleSpec) -> Vec<Vec3> {
    let hw = s.width * 0.5;
    let hl = s.length * 0.5;
    let y0 = s.ground_clearance;
    let boxy = s.height > 2.0;
    let belt = if boxy {
        s.height * 0.92
    } else {
        s.height * 0.62
    };
    let top = s.height;
    let mut p = Vec::new();
    // Lower body with chamfered corners (front at -Z).
    for &y in &[y0, belt] {
        let inset = if y == y0 { 0.12 } else { 0.0 };
        for &(x, z) in &[
            (hw - 0.08, -hl),
            (hw, -hl + 0.35),
            (hw, hl - 0.3),
            (hw - 0.08, hl),
        ] {
            for sx in [-1.0f32, 1.0] {
                p.push(Vec3::new(sx * (x - inset), y, z * (1.0 - inset * 0.3)));
            }
        }
    }
    if !boxy {
        let rw = hw * 0.82;
        let (zf, zb) = (-hl * 0.25, hl * 0.55);
        for sx in [-1.0f32, 1.0] {
            p.push(Vec3::new(sx * rw, top, zf + 0.3));
            p.push(Vec3::new(sx * rw, top, zb - 0.25));
        }
    } else {
        for sx in [-1.0f32, 1.0] {
            p.push(Vec3::new(sx * (hw - 0.05), top, -hl + 0.6));
            p.push(Vec3::new(sx * (hw - 0.05), top, hl - 0.05));
        }
    }
    p
}
