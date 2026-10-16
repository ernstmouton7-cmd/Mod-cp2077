use gridlock_physics::fracture::DestructionSystem;
use gridlock_physics::glam::Vec3;
use gridlock_physics::*;
fn main() {
    let mut w = PhysicsWorld::default();
    w.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(0.0, -0.5, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(500.0, 0.5, 500.0))).friction(0.9),
    );
    let mut ds = DestructionSystem::new();
    ds.spawn(
        &mut w,
        ConvexHull::cuboid(Vec3::new(3.0, 0.5, 0.1)),
        Pose::from_translation(Vec3::new(0.0, 0.5, -25.0)),
        true,
        1400.0,
        6000.0,
        24,
        Surface::Concrete,
    );
    let v = w.add_vehicle(
        VehicleClass::Suv.desc(),
        Pose::from_translation(Vec3::new(0.0, 0.05, 0.0)),
    );
    for _ in 0..30 {
        w.step(1.0 / 60.0);
    }
    let ch = w.vehicle(v).unwrap().chassis;
    w.set_velocity(ch, Vec3::new(0.0, 0.0, -20.0), Vec3::ZERO);
    w.set_vehicle_controls(
        v,
        VehicleControls {
            throttle: 0.7,
            ..Default::default()
        },
    );
    for i in 0..240 {
        w.step(1.0 / 60.0);
        let n = ds.update(&mut w).len();
        if n > 0 {
            println!("{i} fractured into {n}");
        }
        if i % 10 == 0 || (60..90).contains(&i) {
            println!(
                "{i} z {:.2} v {:.2} destructibles {} y {:.2} pitch {:.2} wheels {:?} contacts {}",
                w.body(ch).unwrap().pose.p.z,
                w.vehicle(v).unwrap().telemetry.speed,
                ds.destructibles.len(),
                w.body(ch).unwrap().pose.p.y,
                (w.body(ch).unwrap().pose.q * -Vec3::Z).y,
                w.vehicle(v)
                    .unwrap()
                    .wheels
                    .iter()
                    .map(|w| (w.in_contact, w.contact_body.map(|b| b.0.index)))
                    .collect::<Vec<_>>(),
                w.contacts_of(ch).filter(|c| c.touching).count()
            );
        }
    }
}
