use gridlock_physics::cloth::{Cloth, ClothDesc};
use gridlock_physics::fracture::DestructionSystem;
use gridlock_physics::glam::{Vec2, Vec3};
use gridlock_physics::props::{PropEvent, PropKind, PropSystem};
use gridlock_physics::water::{PuddleField, WaterVolume};
use gridlock_physics::*;

const DT: f32 = 1.0 / 60.0;

fn flat_world() -> PhysicsWorld {
    let mut w = PhysicsWorld::default();
    w.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(0.0, -0.5, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(500.0, 0.5, 500.0))).friction(0.9),
    );
    w
}

fn drive_at(w: &mut PhysicsWorld, class: VehicleClass, speed: f32) -> VehicleHandle {
    let v = w.add_vehicle(
        class.desc(),
        Pose::from_translation(Vec3::new(0.0, 0.05, 0.0)),
    );
    for _ in 0..30 {
        w.step(DT);
    }
    let chassis = w.vehicle(v).unwrap().chassis;
    w.set_velocity(chassis, Vec3::new(0.0, 0.0, -speed), Vec3::ZERO);
    for wh in &mut w.vehicle_mut(v).unwrap().wheels {
        wh.omega = speed / 0.32;
    }
    v
}

#[test]
fn car_snaps_a_sign_but_a_tap_only_wobbles_a_lamp() {
    let mut w = flat_world();
    let mut props = PropSystem::new();
    let sign = props.spawn(
        &mut w,
        PropKind::TrafficSign,
        Pose::from_translation(Vec3::new(0.0, 0.0, -20.0)),
    );
    let lamp = props.spawn(
        &mut w,
        PropKind::StreetLamp,
        Pose::from_translation(Vec3::new(6.0, 0.0, -12.0)),
    );
    let v = drive_at(&mut w, VehicleClass::Sedan, 14.0);
    w.set_vehicle_controls(
        v,
        VehicleControls {
            throttle: 0.4,
            ..Default::default()
        },
    );
    let mut snapped = Vec::new();
    let mut speed_before = 0.0;
    for _ in 0..180 {
        let s = w.vehicle(v).unwrap().telemetry.speed;
        w.step(DT);
        for e in props.update(&w) {
            if let PropEvent::Snapped { prop, .. } = e {
                snapped.push(prop);
                speed_before = s;
            }
        }
    }
    assert!(snapped.contains(&sign), "sign should snap");
    let speed = w.vehicle(v).unwrap().telemetry.speed;
    assert!(
        speed > speed_before * 0.7,
        "a sign shouldn't stop a car dead: {speed_before} -> {speed}"
    );
    // Nudge the lamp at walking pace.
    let mut w2 = flat_world();
    let mut props2 = PropSystem::new();
    let lamp2 = props2.spawn(
        &mut w2,
        PropKind::StreetLamp,
        Pose::from_translation(Vec3::new(0.0, 0.0, -8.0)),
    );
    let v2 = drive_at(&mut w2, VehicleClass::Compact, 1.5);
    w2.set_vehicle_controls(
        v2,
        VehicleControls {
            throttle: 0.15,
            ..Default::default()
        },
    );
    let mut broke = false;
    for _ in 0..300 {
        w2.step(DT);
        broke |= props2
            .update(&w2)
            .iter()
            .any(|e| matches!(e, PropEvent::Snapped { .. }));
    }
    assert!(
        !broke && !props2.props[lamp2].broken,
        "slow nudge must not snap a lamp post"
    );
    let _ = lamp;
}

#[test]
fn car_smashes_through_a_destructible_wall() {
    let mut w = flat_world();
    let mut ds = DestructionSystem::new();
    // A low cinder-block garden wall (~850 kg).
    let wall = ConvexHull::cuboid(Vec3::new(3.0, 0.5, 0.1));
    ds.spawn(
        &mut w,
        wall,
        Pose::from_translation(Vec3::new(0.0, 0.5, -25.0)),
        true,
        1400.0,
        6000.0,
        24,
        Surface::Concrete,
    );
    let v = drive_at(&mut w, VehicleClass::Suv, 20.0);
    w.set_vehicle_controls(
        v,
        VehicleControls {
            throttle: 0.7,
            ..Default::default()
        },
    );
    let mut pieces = 0;
    for _ in 0..240 {
        w.step(DT);
        pieces += ds.update(&mut w).len();
    }
    eprintln!(
        "wall broke into {pieces} pieces; car speed {:.1}",
        w.vehicle(v).unwrap().telemetry.speed
    );
    assert!(pieces >= 10);
    let z = w.body(w.vehicle(v).unwrap().chassis).unwrap().pose.p.z;
    assert!(z < -25.0, "car should break through, at z={z}");
}

#[test]
fn flag_in_the_wind_stays_attached_and_sane() {
    let w = flat_world();
    let desc = ClothDesc {
        width: 1.5,
        height: 1.0,
        columns: 16,
        rows: 11,
        ..Default::default()
    };
    let mut flag = Cloth::sheet(desc, Vec3::new(0.0, 6.0, 0.0), Vec3::Z, -Vec3::Y);
    // Pin the left edge (column 0) like a flagpole.
    for r in 0..11 {
        let p = flag.particle(0, r);
        let pos = flag.positions[p as usize];
        flag.pin(p, gridlock_physics::cloth::Pin::World(pos));
    }
    for i in 0..600 {
        let gust = 1.0 + 0.4 * (i as f32 * 0.05).sin();
        flag.step(&w, DT, Vec3::new(0.0, 0.0, 9.0 * gust), 4);
    }
    assert!(flag.positions.iter().all(|p| p.is_finite()));
    let far = flag.positions[flag.particle(15, 5) as usize];
    // Blown out downwind, not hanging straight down.
    assert!(far.z > 0.8, "flag tip at {far:?}");
    let edge = flag.positions[flag.particle(15, 0) as usize]
        - flag.positions[flag.particle(0, 0) as usize];
    assert!(
        edge.length() < 1.5 * 1.12,
        "over-stretched: {}",
        edge.length()
    );
}

#[test]
fn light_crate_floats_half_submerged() {
    let mut w = PhysicsWorld::default();
    let water = WaterVolume::new(Vec3::new(-20.0, -10.0, -20.0), Vec3::new(20.0, 0.0, 20.0));
    let (b, _) = w.add_body_with(
        BodyDesc::dynamic(Pose::from_translation(Vec3::new(0.0, 2.0, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::splat(0.5))).density(500.0),
    );
    for _ in 0..900 {
        water.apply(&mut w);
        w.step(DT);
    }
    let y = w.body(b).unwrap().pose.p.y;
    assert!(
        y.abs() < 0.15,
        "crate should float with its centre near the surface, y={y}"
    );
}

#[test]
fn deep_water_at_speed_aquaplanes() {
    let mut w = flat_world();
    w.puddles = Some(PuddleField::new(
        Vec2::new(-10.0, -400.0),
        Vec2::new(20.0, 400.0),
        1.0,
        |_, _| 0.009,
    ));
    for d in &mut w.puddles.as_mut().unwrap().depth {
        *d = 0.009;
    }
    let v = drive_at(&mut w, VehicleClass::Sedan, 33.0);
    for _ in 0..10 {
        w.step(DT);
    }
    let grip: f32 = w
        .vehicle(v)
        .unwrap()
        .wheels
        .iter()
        .map(|w| w.aquaplane)
        .fold(1.0, f32::min);
    assert!(grip < 0.35, "should aquaplane, grip {grip}");
    assert!(w.vehicle(v).unwrap().wheels[0].water_depth > 0.008);
}
