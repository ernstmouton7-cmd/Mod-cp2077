use gridlock_physics::glam::{Quat, Vec3};
use gridlock_physics::*;
fn main() {
    let only: Option<usize> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let mut w = PhysicsWorld::default();
    w.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(0.0, -0.5, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(50.0, 0.5, 50.0))).friction(0.8),
    );
    let mut seed = 12345u32;
    let mut rnd = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / 16777216.0
    };
    let mut bodies = Vec::new();
    for i in 0..200 {
        let p = Vec3::new(
            (i % 5) as f32 * 1.3 - 2.6 + rnd() * 0.2,
            1.0 + (i / 25) as f32 * 1.3,
            ((i / 5) % 5) as f32 * 1.3 - 2.6 + rnd() * 0.2,
        );
        let q = Quat::from_euler(
            gridlock_physics::glam::EulerRot::XYZ,
            rnd() * 3.0,
            rnd() * 3.0,
            rnd() * 3.0,
        );
        let kind = i % 4;
        let shape = match kind {
            0 => Shape::cuboid(Vec3::new(
                0.3 + rnd() * 0.3,
                0.2 + rnd() * 0.3,
                0.3 + rnd() * 0.2,
            )),
            1 => Shape::sphere(0.2 + rnd() * 0.3),
            2 => Shape::capsule(0.2 + rnd() * 0.3, 0.15 + rnd() * 0.1),
            _ => Shape::convex(
                &(0..12)
                    .map(|_| Vec3::new(rnd() - 0.5, rnd() - 0.5, rnd() - 0.5) * 0.8)
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        };
        if only.is_some_and(|k| k != kind) {
            continue;
        }
        bodies.push((
            kind,
            w.add_body_with(BodyDesc::dynamic(Pose::new(p, q)), ColliderDesc::new(shape))
                .0,
        ));
    }
    for i in 0..720 {
        w.step(1.0 / 60.0);
        if i > 300 {
            for (k, b) in &bodies {
                let body = w.body(*b).unwrap();
                if body.lin_vel.length() > 3.0 || body.ang_vel.length() > 20.0 {
                    println!(
                        "step {i} kind {k} body {:?} v {:.2} w {:.2} p {:?}",
                        b.0.index,
                        body.lin_vel.length(),
                        body.ang_vel.length(),
                        body.pose.p
                    );
                }
            }
        }
    }
    let asleep = bodies
        .iter()
        .filter(|(_, b)| w.body(*b).unwrap().sleeping)
        .count();
    println!("asleep {asleep}/{}", bodies.len());
}
