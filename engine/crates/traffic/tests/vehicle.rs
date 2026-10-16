//! Port of tests/vehicle.test.ts.

use gridlock_traffic::vehicle::{HeadlightSwitch, Indicator, TransmissionMode};
use gridlock_traffic::{spec, MotionMode, Vehicle, VehicleClass, VehicleId, VehicleLights, VehicleRole};

const DT: f64 = 1.0 / 60.0;

fn car(cls: VehicleClass) -> Vehicle {
    let mut v = Vehicle::new(VehicleId(1), spec(cls), 0xffffff, VehicleRole::Player);
    v.mode = MotionMode::Physics;
    v.dynamics.reset(0.0, 0.0, 0.0, 0.0);
    v
}

fn step(v: &mut Vehicle, grip: f64) {
    let scale = if v.dynamics.tcs_active { 0.4 } else { 1.0 };
    let t = v.pt.update(DT, v.dynamics.driven_omega(), v.inputs.throttle, v.dynamics.u, scale);
    v.dynamics.surface_grip = grip;
    let (inputs, steer, assists) = (v.inputs, v.steer_target, v.assists);
    v.dynamics.step(DT, &inputs, t, steer, &assists);
    v.sync_from_dynamics();
}

fn zero_to_100(cls: VehicleClass) -> f64 {
    let mut v = car(cls);
    v.inputs.throttle = 1.0;
    let mut t = 0.0;
    while v.dynamics.u < 100.0 / 3.6 && t < 60.0 {
        step(&mut v, 1.0);
        t += DT;
    }
    t
}

fn braking_distance(cls: VehicleClass, abs: bool, grip: f64) -> (f64, bool) {
    let mut v = car(cls);
    v.dynamics.reset(0.0, 0.0, 0.0, 100.0 / 3.6);
    v.pt.gear = 5;
    v.assists.abs = abs;
    v.inputs.brake = 1.0;
    let mut locked_seen = false;
    let mut i = 0;
    while i < 60 * 20 && v.dynamics.u > 0.05 {
        step(&mut v, grip);
        if v.dynamics.front_locked {
            locked_seen = true;
        }
        i += 1;
    }
    (v.dynamics.x, locked_seen)
}

#[test]
fn zero_to_100_times_are_plausible_per_class() {
    let sports = zero_to_100(VehicleClass::Sports);
    let sedan = zero_to_100(VehicleClass::Sedan);
    let compact = zero_to_100(VehicleClass::Compact);
    let truck = zero_to_100(VehicleClass::Truck);
    println!("0-100: sports {sports:.1} s, sedan {sedan:.1} s, compact {compact:.1} s, truck {truck:.1} s");
    assert!(sports > 2.8 && sports < 6.0, "sports {sports}");
    assert!(sedan > 6.0 && sedan < 12.0, "sedan {sedan}");
    assert!(compact > sedan, "compact slower than sedan");
    assert!(truck > 15.0, "truck slow");
}

#[test]
fn braking_dry_wet_and_without_abs() {
    let dry = braking_distance(VehicleClass::Sedan, true, 1.0);
    let wet = braking_distance(VehicleClass::Sedan, true, 0.68);
    let no_abs = braking_distance(VehicleClass::Sedan, false, 1.0);
    println!("braking: dry ABS {:.1} m, wet ABS {:.1} m, dry no-ABS {:.1} m", dry.0, wet.0, no_abs.0);
    assert!(dry.0 > 32.0 && dry.0 < 52.0, "dry {}", dry.0);
    assert!(wet.0 > dry.0 * 1.25, "wet road is longer");
    assert!(!dry.1, "ABS prevents lock-up");
    assert!(no_abs.1, "without ABS the wheels lock");
    assert!(no_abs.0 > dry.0, "locked wheels stop longer");
}

#[test]
fn steady_cornering_stays_within_the_friction_limit() {
    let mut v = car(VehicleClass::Sports);
    v.dynamics.reset(0.0, 0.0, 0.0, 25.0);
    v.pt.gear = 3;
    let mut max_lat: f64 = 0.0;
    for _ in 0..60 * 5 {
        v.inputs.throttle = if v.dynamics.u < 25.0 { 0.5 } else { 0.2 };
        v.steer_target = 0.12;
        step(&mut v, 1.0);
        max_lat = max_lat.max(v.dynamics.ay.abs());
    }
    assert!(max_lat < spec(VehicleClass::Sports).grip * 9.81 * 1.15, "lateral accel {max_lat:.1} m/s²");
    assert!(v.dynamics.r.abs() > 0.2, "car turns");
}

#[test]
fn handbrake_at_speed_locks_the_rear_and_rotates_the_car() {
    let mut v = car(VehicleClass::Sports);
    v.dynamics.reset(0.0, 0.0, 0.0, 18.0);
    v.pt.gear = 2;
    v.assists.esc = false;
    let mut rear_locked = false;
    let mut max_skid: f64 = 0.0;
    for _ in 0..90 {
        v.inputs.handbrake = 1.0;
        v.steer_target = 0.35;
        step(&mut v, 1.0);
        rear_locked |= v.dynamics.rear_locked;
        max_skid = max_skid.max(v.dynamics.skid);
    }
    assert!(rear_locked, "rear axle locks");
    assert!(v.dynamics.yaw.abs() > 0.6, "rotated {:.2} rad", v.dynamics.yaw);
    assert!(max_skid > 0.5, "tyres skid");
}

#[test]
fn burnout_without_traction_control_controlled_launch_with_it() {
    let mut off = car(VehicleClass::Muscle);
    off.assists.tcs = false;
    off.inputs.throttle = 1.0;
    let mut spin = false;
    for _ in 0..60 {
        step(&mut off, 1.0);
        spin |= off.dynamics.wheelspin;
    }
    assert!(spin, "wheelspin with TCS off");
    let mut on = car(VehicleClass::Muscle);
    on.inputs.throttle = 1.0;
    let mut tcs = false;
    for _ in 0..60 {
        step(&mut on, 1.0);
        tcs |= on.dynamics.tcs_active;
        assert!(!on.dynamics.wheelspin);
    }
    assert!(tcs, "TCS intervenes");
}

#[test]
fn automatic_creeps_manual_stalls_when_the_clutch_is_dumped() {
    let mut a = car(VehicleClass::Sedan);
    for _ in 0..60 * 6 {
        step(&mut a, 1.0);
    }
    assert!(a.dynamics.u > 0.5 && a.dynamics.u < 4.0, "creep speed {:.2} m/s", a.dynamics.u);
    let mut m = car(VehicleClass::Sedan);
    m.pt.mode = TransmissionMode::Manual;
    m.pt.gear = 1;
    m.pt.clutch_pedal = 0.0;
    for _ in 0..60 {
        step(&mut m, 1.0);
    }
    assert!(m.pt.stalled && !m.pt.running, "engine stalled");
    m.pt.start();
    m.pt.clutch_pedal = 1.0;
    for _ in 0..60 {
        step(&mut m, 1.0);
    }
    assert!(m.pt.running, "restarts with the clutch in");
}

#[test]
fn automatic_shifts_up_through_the_gears_and_holds_below_redline() {
    let mut v = car(VehicleClass::Sedan);
    v.inputs.throttle = 1.0;
    let mut max_gear = 1;
    let mut max_rpm: f64 = 0.0;
    for _ in 0..60 * 25 {
        step(&mut v, 1.0);
        max_gear = max_gear.max(v.pt.gear);
        max_rpm = max_rpm.max(v.pt.rpm);
    }
    assert!(max_gear >= 5, "reached gear {max_gear}");
    assert!(max_rpm <= spec(VehicleClass::Sedan).redline + 200.0, "rpm {max_rpm:.0}");
}

#[test]
fn engine_damage_cuts_power_no_fuel_stops_the_engine() {
    let a = zero_to_100(VehicleClass::Sedan);
    let mut v = car(VehicleClass::Sedan);
    v.pt.health = 0.3;
    v.inputs.throttle = 1.0;
    let mut t = 0.0;
    while v.dynamics.u < 100.0 / 3.6 && t < 60.0 {
        step(&mut v, 1.0);
        t += DT;
    }
    assert!(t > a * 1.3, "damaged engine is slower");
    let mut f = car(VehicleClass::Sedan);
    f.pt.fuel = 0.0001;
    f.inputs.throttle = 1.0;
    for _ in 0..120 {
        step(&mut f, 1.0);
    }
    assert!(!f.pt.running, "out of fuel");
}

#[test]
fn indicators_relay_timing_toggling_hazards_and_auto_cancel() {
    let mut l = VehicleLights::new(0.0);
    l.toggle_indicator(Indicator::Left);
    let mut edges = 0;
    let mut prev = false;
    for _ in 0..60 * 7 {
        l.update(DT, 0.0);
        let s = l.left_lamp_on();
        if s != prev {
            edges += 1;
        }
        prev = s;
    }
    let blinks_per_min = edges as f64 / 2.0 / (7.0 / 60.0);
    assert!(blinks_per_min > 60.0 && blinks_per_min < 120.0, "{blinks_per_min:.0} blinks/min");
    assert!(!l.right_lamp_on());
    l.toggle_indicator(Indicator::Left);
    assert_eq!(l.indicator, Indicator::Off, "pressing again cancels");
    l.toggle_indicator(Indicator::Right);
    for _ in 0..30 {
        l.update(DT, -0.3); // steer right
    }
    assert_eq!(l.indicator, Indicator::Right);
    for _ in 0..30 {
        l.update(DT, 0.0);
    }
    assert_eq!(l.indicator, Indicator::Off, "auto-cancel after the wheel returns");
    l.hazard = true;
    let mut both = false;
    for _ in 0..60 {
        l.update(DT, 0.0);
        both |= l.left_lamp_on() && l.right_lamp_on();
    }
    assert!(both, "hazards flash both sides");
}

#[test]
fn headlights_switch_cycle_auto_high_beam_needs_low_beam_flash_always_works() {
    let mut l = VehicleLights::new(0.0);
    l.headlight_switch = HeadlightSwitch::Off;
    l.cycle_headlights();
    assert_eq!(l.headlight_switch, HeadlightSwitch::Auto);
    l.dark = false;
    assert!(!l.low_beam_on());
    l.dark = true;
    assert!(l.low_beam_on());
    l.cycle_headlights();
    assert_eq!(l.headlight_switch, HeadlightSwitch::Low);
    l.cycle_headlights();
    assert_eq!(l.headlight_switch, HeadlightSwitch::Off);
    l.high_beam = true;
    assert!(!l.high_beam_on(), "high beam needs the lights on");
    l.flash = true;
    assert!(l.high_beam_on(), "flash-to-pass works with lights off");
}
