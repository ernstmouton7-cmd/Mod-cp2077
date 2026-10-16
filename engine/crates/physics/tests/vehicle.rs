use gridlock_physics::glam::{Quat, Vec3};
use gridlock_physics::vehicle::powertrain::Selector;
use gridlock_physics::*;

const DT: f32 = 1.0 / 60.0;

fn flat_world() -> PhysicsWorld {
    let mut w = PhysicsWorld::default();
    w.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(0.0, -0.5, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(3000.0, 0.5, 3000.0))).friction(0.9),
    );
    w
}

fn spawn(w: &mut PhysicsWorld, class: VehicleClass) -> VehicleHandle {
    w.add_vehicle(
        class.desc(),
        Pose::from_translation(Vec3::new(0.0, 0.05, 0.0)),
    )
}

fn controls(w: &mut PhysicsWorld, v: VehicleHandle, throttle: f32, brake: f32, steer: f32) {
    w.set_vehicle_controls(
        v,
        VehicleControls {
            throttle,
            brake,
            steer,
            ..Default::default()
        },
    );
}

fn chassis(w: &PhysicsWorld, v: VehicleHandle) -> &RigidBody {
    w.body(w.vehicle(v).unwrap().chassis).unwrap()
}

fn speed(w: &PhysicsWorld, v: VehicleHandle) -> f32 {
    w.vehicle(v).unwrap().telemetry.speed
}

#[test]
fn settles_at_ride_height() {
    for class in VehicleClass::ALL {
        let mut w = flat_world();
        let v = spawn(&mut w, class);
        w.vehicle_mut(v).unwrap().set_selector(Selector::Park);
        for _ in 0..240 {
            w.step(DT);
        }
        let b = chassis(&w, v);
        assert!(
            b.pose.p.y.abs() < 0.04,
            "{class:?} rests at y={}",
            b.pose.p.y
        );
        assert!(
            b.lin_vel.length() < 0.05,
            "{class:?} still moving {:?}",
            b.lin_vel
        );
        let up = b.pose.q * Vec3::Y;
        assert!(up.y > 0.999, "{class:?} tilted {up:?}");
        assert_eq!(w.vehicle(v).unwrap().telemetry.wheels_on_ground, 4);
        let pos = b.pose.p;
        for _ in 0..600 {
            w.step(DT);
        }
        let drift = (chassis(&w, v).pose.p - pos).length();
        assert!(drift < 0.01, "{class:?} creeps {drift} m while parked");
    }
}

fn zero_to_100(class: VehicleClass) -> f32 {
    let mut w = flat_world();
    let v = spawn(&mut w, class);
    for _ in 0..60 {
        w.step(DT);
    }
    controls(&mut w, v, 1.0, 0.0, 0.0);
    let mut t = 0.0;
    while speed(&w, v) < 100.0 / 3.6 && t < 60.0 {
        w.step(DT);
        t += DT;
    }
    t
}

#[test]
fn acceleration_times_are_plausible() {
    let cases = [
        (VehicleClass::Sports, 3.8, 6.5),
        (VehicleClass::Sedan, 7.0, 11.0),
        (VehicleClass::Compact, 8.5, 14.0),
        (VehicleClass::Suv, 6.5, 11.0),
        (VehicleClass::Truck, 14.0, 40.0),
    ];
    for (class, lo, hi) in cases {
        let t = zero_to_100(class);
        eprintln!("{class:?}: 0-100 in {t:.2} s");
        assert!(
            t >= lo && t <= hi,
            "{class:?} 0-100 {t:.2}s outside {lo}-{hi}"
        );
    }
}

#[test]
fn automatic_shifts_up_through_the_gears() {
    let mut w = flat_world();
    let v = spawn(&mut w, VehicleClass::Sedan);
    controls(&mut w, v, 1.0, 0.0, 0.0);
    let mut max_gear = 0;
    let mut max_rpm: f32 = 0.0;
    for _ in 0..(20.0 / DT) as usize {
        w.step(DT);
        let veh = w.vehicle(v).unwrap();
        max_gear = max_gear.max(veh.powertrain.gear);
        max_rpm = max_rpm.max(veh.powertrain.rpm());
    }
    assert!(max_gear >= 4, "only reached gear {max_gear}");
    assert!(max_rpm < 6300.0 * 1.04, "over-revved {max_rpm}");
}

#[test]
fn braking_distance_from_100() {
    for (abs, lo, hi) in [(true, 36.0, 48.0), (false, 38.0, 60.0)] {
        let mut w = flat_world();
        let v = spawn(&mut w, VehicleClass::Sedan);
        w.vehicle_mut(v).unwrap().assists.abs = abs;
        controls(&mut w, v, 1.0, 0.0, 0.0);
        let mut guard = 0;
        while speed(&w, v) < 100.0 / 3.6 && guard < 3000 {
            w.step(DT);
            guard += 1;
        }
        controls(&mut w, v, 0.0, 1.0, 0.0);
        let start = chassis(&w, v).pose.p;
        while speed(&w, v) > 0.3 && guard < 6000 {
            w.step(DT);
            guard += 1;
        }
        let d = (chassis(&w, v).pose.p - start).length();
        eprintln!("abs={abs}: 100-0 in {d:.1} m");
        assert!(d > lo && d < hi, "abs={abs}: stopping distance {d:.1}");
        let yaw = (chassis(&w, v).pose.q * -Vec3::Z).x.abs();
        assert!(yaw < 0.1, "car turned while braking straight: {yaw}");
    }
}

fn slope_world(grade: f32) -> (PhysicsWorld, Quat) {
    let mut w = PhysicsWorld::default();
    // Slope rising toward -Z (the car's forward).
    let q = Quat::from_rotation_x(grade.atan());
    w.add_body_with(
        BodyDesc::fixed(Pose::new(Vec3::new(0.0, -0.5, 0.0), q)),
        ColliderDesc::new(Shape::cuboid(Vec3::new(50.0, 0.5, 200.0))).friction(0.9),
    );
    (w, q)
}

#[test]
fn holds_on_a_hill_in_park_and_rolls_in_neutral() {
    let (mut w, q) = slope_world(0.15);
    let v = w.add_vehicle(
        VehicleClass::Sedan.desc(),
        Pose::new(q * Vec3::new(0.0, 0.1, 0.0), q),
    );
    w.vehicle_mut(v).unwrap().set_selector(Selector::Park);
    for _ in 0..180 {
        w.step(DT);
    }
    let p0 = chassis(&w, v).pose.p;
    for _ in 0..300 {
        w.step(DT);
    }
    let crept = (chassis(&w, v).pose.p - p0).length();
    assert!(crept < 0.02, "parked car slid {crept} m");
    w.vehicle_mut(v).unwrap().set_selector(Selector::Neutral);
    w.wake(w.vehicle(v).unwrap().chassis);
    for _ in 0..180 {
        w.step(DT);
    }
    let rolled = (chassis(&w, v).pose.p - p0).length();
    assert!(
        rolled > 1.0,
        "car in neutral should roll back, moved {rolled}"
    );
    assert!(
        speed(&w, v) < -0.5,
        "should roll backwards, speed {}",
        speed(&w, v)
    );
}

#[test]
fn foot_brake_holds_on_hill() {
    let (mut w, q) = slope_world(0.2);
    let v = w.add_vehicle(
        VehicleClass::Compact.desc(),
        Pose::new(q * Vec3::new(0.0, 0.1, 0.0), q),
    );
    w.vehicle_mut(v).unwrap().set_selector(Selector::Neutral);
    controls(&mut w, v, 0.0, 0.6, 0.0);
    for _ in 0..120 {
        w.step(DT);
    }
    let p0 = chassis(&w, v).pose.p;
    for _ in 0..240 {
        w.step(DT);
    }
    assert!((chassis(&w, v).pose.p - p0).length() < 0.02);
}

#[test]
fn steady_cornering_turns_left_and_stays_composed() {
    let mut w = flat_world();
    let v = spawn(&mut w, VehicleClass::Sedan);
    // Reach ~50 km/h, then hold a left turn with gentle throttle.
    controls(&mut w, v, 0.6, 0.0, 0.0);
    while speed(&w, v) < 14.0 {
        w.step(DT);
    }
    let mut max_lat: f32 = 0.0;
    let heading0 = chassis(&w, v).pose.q * -Vec3::Z;
    for _ in 0..(6.0 / DT) as usize {
        controls(&mut w, v, 0.3, 0.0, 0.6);
        w.step(DT);
        max_lat = max_lat.max(w.vehicle(v).unwrap().telemetry.lateral_g.abs());
    }
    let heading1 = chassis(&w, v).pose.q * -Vec3::Z;
    let turned = heading0.cross(heading1).y;
    let tel = w.vehicle(v).unwrap().telemetry;
    eprintln!(
        "cornering: lat g {max_lat:.2}, drift {:.3} rad, yaw {:.2}",
        tel.drift_angle, tel.yaw_rate
    );
    assert!(
        tel.yaw_rate > 0.2,
        "should be turning left (positive yaw), yaw {}",
        tel.yaw_rate
    );
    assert!(turned.abs() > 0.1 || heading0.dot(heading1) < 0.9);
    assert!(max_lat < 1.2, "impossible lateral g {max_lat}");
    assert!(
        tel.drift_angle.abs() < 0.2,
        "spun out: drift {}",
        tel.drift_angle
    );
    let up = chassis(&w, v).pose.q * Vec3::Y;
    assert!(up.y > 0.95, "rolled over");
}

#[test]
fn handbrake_turn_rotates_the_car() {
    let mut w = flat_world();
    let v = spawn(&mut w, VehicleClass::Sports);
    w.vehicle_mut(v).unwrap().assists.esc = false;
    controls(&mut w, v, 0.7, 0.0, 0.0);
    while speed(&w, v) < 16.0 {
        w.step(DT);
    }
    let mut max_drift: f32 = 0.0;
    for _ in 0..90 {
        w.set_vehicle_controls(
            v,
            VehicleControls {
                throttle: 0.0,
                steer: 1.0,
                handbrake: 1.0,
                ..Default::default()
            },
        );
        w.step(DT);
        max_drift = max_drift.max(w.vehicle(v).unwrap().telemetry.drift_angle.abs());
    }
    assert!(
        max_drift > 0.3,
        "handbrake should kick the tail out, drift {max_drift}"
    );
    let rear_slide = w.vehicle(v).unwrap().wheels[2].slide_speed;
    eprintln!("handbrake drift {max_drift:.2} rad, rear slide {rear_slide:.1} m/s");
}

#[test]
fn crash_into_wall_reports_impact() {
    let mut w = flat_world();
    w.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(0.0, 1.0, -40.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(10.0, 1.0, 0.5))),
    );
    let v = spawn(&mut w, VehicleClass::Sedan);
    controls(&mut w, v, 1.0, 0.0, 0.0);
    let mut impact: Option<(f32, f32)> = None;
    for _ in 0..600 {
        w.step(DT);
        for e in w.events() {
            if let PhysicsEvent::Impact { impulse, speed, .. } = e {
                if *speed > 3.0 && impact.is_none() {
                    impact = Some((*impulse, *speed));
                }
            }
        }
    }
    let (impulse, sp) = impact.expect("no impact event");
    eprintln!("impact {impulse:.0} N·s at {sp:.1} m/s");
    assert!(impulse > 1550.0 * sp * 0.4, "impulse too small");
    assert!(chassis(&w, v).pose.p.z > -40.0, "drove through the wall");
}

#[test]
fn crash_deforms_body_and_breaks_parts() {
    use gridlock_physics::deform::*;
    let mut w = flat_world();
    w.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(0.0, 1.0, -60.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(10.0, 1.0, 0.5))),
    );
    let v = spawn(&mut w, VehicleClass::Sedan);
    let size = w.vehicle(v).unwrap().desc.size;
    let body = DeformMesh::grid_box(
        Vec3::new(0.0, 0.75, 0.0),
        Vec3::new(size.y * 0.5, 0.55, size.x * 0.5),
        12,
    );
    let hl = size.x * 0.5;
    let parts = vec![
        PartDesc {
            name: "front bumper".into(),
            kind: PartKind::FrontBumper,
            mount: PartMount::Fixed,
            mesh: DeformMesh::grid_box(
                Vec3::new(0.0, 0.45, -hl - 0.05),
                Vec3::new(0.9, 0.15, 0.08),
                4,
            ),
            mass: 8.0,
            strength: 9000.0,
        },
        PartDesc {
            name: "hood".into(),
            kind: PartKind::Hood,
            mount: PartMount::Hinge {
                pivot: Vec3::new(0.0, 1.0, -0.9),
                axis: Vec3::X,
                lower: -1.2,
                upper: 0.0,
            },
            mesh: DeformMesh::grid_box(Vec3::new(0.0, 1.0, -1.7), Vec3::new(0.85, 0.03, 0.75), 4),
            mass: 15.0,
            strength: 14000.0,
        },
    ];
    let mut dmg = VehicleDamage::new(v, body, parts, 4);
    controls(&mut w, v, 1.0, 0.0, 0.0);
    let mut events = Vec::new();
    for _ in 0..900 {
        w.step(DT);
        events.extend(dmg.process(&mut w));
        events.extend(dmg.update(&w));
    }
    let max_dent = dmg
        .body
        .positions
        .iter()
        .zip(&dmg.body.rest)
        .map(|(p, r)| (*p - *r).length())
        .fold(0.0, f32::max);
    let front_dent = dmg
        .body
        .positions
        .iter()
        .zip(&dmg.body.rest)
        .filter(|(_, r)| r.z < -hl * 0.8)
        .map(|(p, r)| p.z - r.z)
        .fold(0.0, f32::max);
    let rear_dent = dmg
        .body
        .positions
        .iter()
        .zip(&dmg.body.rest)
        .filter(|(_, r)| r.z > hl * 0.8)
        .map(|(p, r)| (*p - *r).length())
        .fold(0.0, f32::max);
    let engine = w.vehicle(v).unwrap().damage.engine;
    eprintln!("crash: max dent {max_dent:.3} m, front pushed back {front_dent:.3} m, rear {rear_dent:.3}, engine {engine:.2}, events {}", events.len());
    assert!(front_dent > 0.05, "front should crumple backwards");
    assert!(rear_dent < 0.02, "rear untouched");
    assert!(max_dent <= 0.45 + 1e-3);
    assert!(engine < 1.0);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DamageEvent::PartDetached { part: 0, .. })),
        "bumper should come off"
    );
    assert!(
        dmg.parts[1].state != PartState::Attached,
        "hood should be unlatched"
    );
}
