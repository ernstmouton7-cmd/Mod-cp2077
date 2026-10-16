use gridlock_physics::glam::Vec3;
use gridlock_physics::*;
fn main() {
    let mut w = PhysicsWorld::default();
    w.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(0.0, -0.5, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(
            std::env::var("G")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000.0),
            0.5,
            std::env::var("G")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000.0),
        ))),
    );
    let v = w.add_vehicle(
        VehicleClass::Sedan.desc(),
        Pose::from_translation(Vec3::new(0.0, 0.05, 0.0)),
    );
    let d = w.vehicle(v).unwrap().desc.clone();
    println!(
        "hardpoints {:?} travel {} k {}",
        d.wheels.iter().map(|w| w.hardpoint).collect::<Vec<_>>(),
        d.wheels[0].travel,
        d.wheels[0].spring
    );
    let throttle: f32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    for i in 0..600 {
        if i == 120 {
            w.set_vehicle_controls(
                v,
                VehicleControls {
                    throttle,
                    ..Default::default()
                },
            );
        }
        w.step(1.0 / 60.0);
        if i % 30 == 0 {
            let veh = w.vehicle(v).unwrap();
            let b = w.body(veh.chassis).unwrap();
            println!(
                "{i:4} y {:.3} vz {:.2} sleep {} gear {} rpm {:.0} clutch {:.2} | {}",
                b.pose.p.y,
                veh.telemetry.speed,
                b.sleeping,
                veh.powertrain.gear,
                veh.powertrain.rpm(),
                veh.powertrain.clutch,
                veh.wheels
                    .iter()
                    .map(|w| format!(
                        "c{:.3} N{:.0} om{:.1} k{:.2} fx{:.0}",
                        w.compression, w.load, w.omega, w.slip_ratio, w.fx
                    ))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }
    }
}
