use gridlock_physics::glam::{Quat, Vec3};
use gridlock_physics::vehicle::powertrain::Selector;
use gridlock_physics::*;
fn main() {
    let mut w = PhysicsWorld::default();
    let q = Quat::from_rotation_x(0.15f32.atan());
    w.add_body_with(
        BodyDesc::fixed(Pose::new(Vec3::new(0.0, -0.5, 0.0), q)),
        ColliderDesc::new(Shape::cuboid(Vec3::new(50.0, 0.5, 200.0))).friction(0.9),
    );
    let v = w.add_vehicle(
        VehicleClass::Sedan.desc(),
        Pose::new(q * Vec3::new(0.0, 0.1, 0.0), q),
    );
    w.vehicle_mut(v).unwrap().set_selector(Selector::Park);
    for i in 0..480 {
        w.step(1.0 / 60.0);
        if i % 20 == 0 {
            let veh = w.vehicle(v).unwrap();
            let b = w.body(veh.chassis).unwrap();
            println!(
                "{i} p {:?} v {:.3} sleep {} gear {} sel {:?} | {}",
                b.pose.p,
                veh.telemetry.speed,
                b.sleeping,
                veh.powertrain.gear,
                veh.powertrain.selector,
                veh.wheels
                    .iter()
                    .map(|w| format!(
                        "N{:.0} om{:.2} bt{:.0} fx{:.0} c{}",
                        w.load, w.omega, w.brake_torque, w.fx, w.in_contact
                    ))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }
    }
}
