use gridlock_physics::glam::{Quat, Vec3};
use gridlock_physics::*;

const DT: f32 = 1.0 / 60.0;

fn ground(world: &mut PhysicsWorld) -> BodyHandle {
    let (g, _) = world.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(0.0, -0.5, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(50.0, 0.5, 50.0))).friction(0.8),
    );
    g
}

fn cube(world: &mut PhysicsWorld, p: Vec3, half: f32) -> BodyHandle {
    world
        .add_body_with(
            BodyDesc::dynamic(Pose::from_translation(p)),
            ColliderDesc::new(Shape::cuboid(Vec3::splat(half))).friction(0.6),
        )
        .0
}

fn run(world: &mut PhysicsWorld, seconds: f32) {
    for _ in 0..(seconds / DT).round() as usize {
        world.step(DT);
    }
}

#[test]
fn free_fall_matches_analytic() {
    let mut w = PhysicsWorld::default();
    let (b, _) = w.add_body_with(
        BodyDesc::dynamic(Pose::from_translation(Vec3::new(0.0, 100.0, 0.0))),
        ColliderDesc::new(Shape::sphere(0.5)),
    );
    run(&mut w, 2.0);
    let y = w.body(b).unwrap().pose.p.y;
    let expected = 100.0 - 0.5 * 9.81 * 4.0;
    assert!((y - expected).abs() < 0.1, "y {y} expected {expected}");
}

#[test]
fn box_comes_to_rest_and_sleeps() {
    let mut w = PhysicsWorld::default();
    ground(&mut w);
    let b = cube(&mut w, Vec3::new(0.0, 2.0, 0.0), 0.5);
    run(&mut w, 4.0);
    let body = w.body(b).unwrap();
    assert!((body.pose.p.y - 0.5).abs() < 0.02, "y {}", body.pose.p.y);
    assert!(body.sleeping, "should sleep, v={:?}", body.lin_vel);
}

#[test]
fn tall_stack_is_stable() {
    let mut w = PhysicsWorld::default();
    ground(&mut w);
    let mut boxes = Vec::new();
    for i in 0..12 {
        boxes.push(cube(&mut w, Vec3::new(0.0, 0.5 + i as f32 * 1.0, 0.0), 0.5));
    }
    run(&mut w, 10.0);
    let top = w.body(*boxes.last().unwrap()).unwrap();
    assert!(
        top.pose.p.x.abs() < 0.05 && top.pose.p.z.abs() < 0.05,
        "top drifted to {:?}",
        top.pose.p
    );
    assert!((top.pose.p.y - 11.5).abs() < 0.15, "top y {}", top.pose.p.y);
    assert!(
        boxes.iter().all(|b| w.body(*b).unwrap().sleeping),
        "stack should sleep"
    );
}

#[test]
fn pyramid_holds() {
    let mut w = PhysicsWorld::default();
    ground(&mut w);
    let mut boxes = Vec::new();
    let n = 8;
    for row in 0..n {
        for i in 0..(n - row) {
            let x = (i as f32 - (n - row - 1) as f32 * 0.5) * 1.02;
            boxes.push(cube(&mut w, Vec3::new(x, 0.5 + row as f32, 0.0), 0.5));
        }
    }
    run(&mut w, 8.0);
    let top = w.body(*boxes.last().unwrap()).unwrap();
    assert!(
        (top.pose.p.y - (n as f32 - 0.5)).abs() < 0.2,
        "top y {}",
        top.pose.p.y
    );
    assert!(top.pose.p.x.abs() < 0.1);
}

#[test]
fn restitution_bounce_height() {
    let mut w = PhysicsWorld::default();
    let (_, gc) = w.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(0.0, -0.5, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(10.0, 0.5, 10.0))),
    );
    w.collider_mut(gc).unwrap().restitution = 0.8;
    let (b, _) = w.add_body_with(
        BodyDesc::dynamic(Pose::from_translation(Vec3::new(0.0, 5.5, 0.0))),
        ColliderDesc::new(Shape::sphere(0.5)).restitution(0.8),
    );
    // Fall then record the apex after the first bounce.
    let mut bounced = false;
    let mut apex: f32 = 0.0;
    for _ in 0..240 {
        w.step(DT);
        let body = w.body(b).unwrap();
        if body.lin_vel.y > 0.5 {
            bounced = true;
        }
        if bounced {
            apex = apex.max(body.pose.p.y);
            if body.lin_vel.y < -0.5 {
                break;
            }
        }
    }
    let h = apex - 0.5;
    assert!(bounced);
    assert!((h - 5.0 * 0.64).abs() < 0.5, "bounce height {h}");
}

#[test]
fn friction_on_slope() {
    for (mu, should_slide) in [(0.6f32, false), (0.15f32, true)] {
        let mut w = PhysicsWorld::default();
        let angle = 20f32.to_radians();
        let q = Quat::from_rotation_z(angle);
        w.add_body_with(
            BodyDesc::fixed(Pose::new(Vec3::ZERO, q)),
            ColliderDesc::new(Shape::cuboid(Vec3::new(30.0, 0.5, 5.0))).friction(mu),
        );
        let start = q * Vec3::new(0.0, 1.0, 0.0);
        let (b, _) = w.add_body_with(
            BodyDesc::dynamic(Pose::new(start, q)),
            ColliderDesc::new(Shape::cuboid(Vec3::splat(0.5))).friction(mu),
        );
        run(&mut w, 0.5);
        let p0 = w.body(b).unwrap().pose.p;
        run(&mut w, 1.0);
        let p1 = w.body(b).unwrap().pose.p;
        let moved = (p1 - p0).length();
        if should_slide {
            // a = g (sin - mu cos); over 1 s starting from v0 ≈ a*0.5
            let a = 9.81 * (angle.sin() - mu * angle.cos());
            let expected = a * 0.5 + 0.5 * a;
            assert!(
                (moved - expected).abs() < 0.35 * expected,
                "mu {mu}: moved {moved}, expected {expected}"
            );
        } else {
            assert!(moved < 0.02, "mu {mu}: crept {moved}");
        }
    }
}

#[test]
fn pendulum_period() {
    let mut w = PhysicsWorld::default();
    let anchor = w.add_body(BodyDesc::fixed(Pose::from_translation(Vec3::new(
        0.0, 10.0, 0.0,
    ))));
    let len = 2.0;
    let small = 0.15f32;
    let start = Vec3::new(len * small.sin(), 10.0 - len * small.cos(), 0.0);
    let (bob, _) = w.add_body_with(
        BodyDesc::dynamic(Pose::from_translation(start)),
        ColliderDesc::new(Shape::sphere(0.05)).density(10000.0),
    );
    w.body_mut(bob).unwrap().angular_damping = 0.0;
    let fa = Pose::IDENTITY;
    let fb = Pose::from_translation(Vec3::new(0.0, 10.0, 0.0) - start);
    w.add_joint(JointDesc::new(anchor, bob, fa, fb, JointKind::Ball));
    // Measure time between successive x zero crossings (half periods).
    let mut last_x = start.x;
    let mut crossings = Vec::new();
    for i in 0..(10.0 / DT) as usize {
        w.step(DT);
        let x = w.body(bob).unwrap().pose.p.x;
        if last_x > 0.0 && x <= 0.0 || last_x < 0.0 && x >= 0.0 {
            crossings.push(i as f32 * DT);
        }
        last_x = x;
    }
    assert!(crossings.len() >= 4);
    let period =
        (crossings[crossings.len() - 1] - crossings[0]) / (crossings.len() - 1) as f32 * 2.0;
    let expected = 2.0 * std::f32::consts::PI * (len / 9.81).sqrt();
    assert!(
        (period - expected).abs() < 0.05 * expected,
        "period {period} expected {expected}"
    );
    // Rod length preserved.
    let d = (w.body(bob).unwrap().pose.p - Vec3::new(0.0, 10.0, 0.0)).length();
    assert!((d - len).abs() < 0.02, "length {d}");
}

#[test]
fn hinge_door_limits_and_motor() {
    let mut w = PhysicsWorld::default();
    let frame = w.add_body(BodyDesc::fixed(Pose::IDENTITY));
    // Door 1 m wide hinged on its left edge about +Y.
    let (door, _) = w.add_body_with(
        BodyDesc::dynamic(Pose::from_translation(Vec3::new(0.5, 1.0, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(0.5, 1.0, 0.03))).density(300.0),
    );
    let axis_y = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2); // frame X -> world Y
    let fa = Pose::new(Vec3::new(0.0, 1.0, 0.0), axis_y);
    let fb = Pose::new(Vec3::new(-0.5, 0.0, 0.0), axis_y);
    let j = w.add_joint(JointDesc::new(
        frame,
        door,
        fa,
        fb,
        JointKind::Hinge {
            limits: Some(HingeLimits {
                lower: -1.2,
                upper: 1.2,
            }),
            motor: Some(HingeMotor {
                speed: 2.0,
                max_torque: 200.0,
            }),
            spring: None,
        },
    ));
    let mut max_angle = f32::MIN;
    for _ in 0..240 {
        w.step(DT);
        let jj = w.joint(j).unwrap();
        let a = jj.hinge_angle(Quat::IDENTITY, w.body(door).unwrap().pose.q);
        max_angle = max_angle.max(a);
    }
    assert!(max_angle > 1.1 && max_angle < 1.28, "max angle {max_angle}");
    // Hinge point stays put.
    let hinge_world = w
        .body(door)
        .unwrap()
        .pose
        .transform_point(Vec3::new(-0.5, 0.0, 0.0));
    assert!(
        (hinge_world - Vec3::new(0.0, 1.0, 0.0)).length() < 0.02,
        "{hinge_world:?}"
    );
}

#[test]
fn breakable_weld_snaps_under_load() {
    let mut w = PhysicsWorld::default();
    let wall = w.add_body(BodyDesc::fixed(Pose::IDENTITY));
    let (beam, _) = w.add_body_with(
        BodyDesc::dynamic(Pose::from_translation(Vec3::new(1.0, 0.0, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(1.0, 0.1, 0.1))).density(500.0),
    );
    let j = w.add_joint(
        JointDesc::new(
            wall,
            beam,
            Pose::IDENTITY,
            Pose::from_translation(Vec3::new(-1.0, 0.0, 0.0)),
            JointKind::Weld {
                linear_hertz: 0.0,
                angular_hertz: 0.0,
                damping_ratio: 1.0,
            },
        )
        .breakable(5000.0, 2000.0),
    );
    run(&mut w, 1.0);
    assert!(
        w.joint(j).is_some(),
        "beam weight (~200 N) must not break it"
    );
    let y_held = w.body(beam).unwrap().pose.p.y;
    assert!(y_held.abs() < 0.05, "cantilever sag {y_held}");
    // Drop a heavy weight on the tip.
    w.add_body_with(
        BodyDesc::dynamic(Pose::from_translation(Vec3::new(1.8, 1.0, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::splat(0.2))).density(30000.0),
    );
    let mut broke = false;
    for _ in 0..120 {
        w.step(DT);
        if w.events()
            .iter()
            .any(|e| matches!(e, PhysicsEvent::JointBroken { .. }))
        {
            broke = true;
        }
    }
    assert!(broke && w.joint(j).is_none());
    assert!(
        w.body(beam).unwrap().pose.p.y < -0.5,
        "beam should fall after breaking"
    );
}

#[test]
fn fast_sphere_does_not_tunnel_thin_wall() {
    let mut w = PhysicsWorld::new(Vec3::ZERO);
    w.add_body_with(
        BodyDesc::fixed(Pose::from_translation(Vec3::new(5.0, 0.0, 0.0))),
        ColliderDesc::new(Shape::cuboid(Vec3::new(0.05, 3.0, 3.0))),
    );
    let (b, _) = w.add_body_with(
        BodyDesc {
            lin_vel: Vec3::new(120.0, 0.0, 0.0),
            ..BodyDesc::dynamic(Pose::IDENTITY)
        },
        ColliderDesc::new(Shape::sphere(0.2)),
    );
    run(&mut w, 0.5);
    assert!(
        w.body(b).unwrap().pose.p.x < 5.0,
        "tunnelled to {}",
        w.body(b).unwrap().pose.p.x
    );
}

#[test]
fn objects_rest_on_trimesh() {
    let mut w = PhysicsWorld::default();
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let n = 10;
    for z in 0..=n {
        for x in 0..=n {
            verts.push(Vec3::new(x as f32 * 2.0 - 10.0, 0.0, z as f32 * 2.0 - 10.0));
        }
    }
    for z in 0..n {
        for x in 0..n {
            let i = (z * (n + 1) + x) as u32;
            let r = n as u32 + 1;
            tris.push([i, i + r, i + 1]);
            tris.push([i + 1, i + r, i + r + 1]);
        }
    }
    w.add_body_with(
        BodyDesc::fixed(Pose::IDENTITY),
        ColliderDesc::new(Shape::trimesh(verts, tris)),
    );
    let b = cube(&mut w, Vec3::new(0.3, 1.0, 0.7), 0.5);
    let (s, _) = w.add_body_with(
        BodyDesc::dynamic(Pose::from_translation(Vec3::new(3.1, 1.0, 1.0))),
        ColliderDesc::new(Shape::sphere(0.4)),
    );
    let (c, _) = w.add_body_with(
        BodyDesc::dynamic(Pose::new(
            Vec3::new(-3.0, 1.0, -2.0),
            Quat::from_rotation_z(1.57),
        )),
        ColliderDesc::new(Shape::capsule(0.6, 0.3)),
    );
    run(&mut w, 3.0);
    assert!(
        (w.body(b).unwrap().pose.p.y - 0.5).abs() < 0.03,
        "box y {}",
        w.body(b).unwrap().pose.p.y
    );
    assert!((w.body(s).unwrap().pose.p.y - 0.4).abs() < 0.03);
    assert!(
        (w.body(c).unwrap().pose.p.y - 0.3).abs() < 0.05,
        "capsule y {}",
        w.body(c).unwrap().pose.p.y
    );
    // A box sliding across triangle seams should not hop.
    w.set_velocity(b, Vec3::new(6.0, 0.0, 0.0), Vec3::ZERO);
    let mut max_y: f32 = 0.0;
    for _ in 0..60 {
        w.step(DT);
        max_y = max_y.max(w.body(b).unwrap().pose.p.y);
    }
    assert!(max_y < 0.53, "hopped to {max_y}");
}

#[test]
fn ray_and_sphere_casts() {
    let mut w = PhysicsWorld::default();
    ground(&mut w);
    let b = cube(&mut w, Vec3::new(0.0, 3.0, 0.0), 0.5);
    w.step(DT);
    let hit = w
        .ray_cast(
            Vec3::new(0.0, 10.0, 0.0),
            -Vec3::Y,
            100.0,
            &QueryFilter::default(),
        )
        .unwrap();
    assert_eq!(hit.body, b);
    assert!((hit.point.y - w.body(b).unwrap().pose.p.y - 0.5).abs() < 1e-3);
    let hit = w
        .ray_cast(
            Vec3::new(0.0, 10.0, 0.0),
            -Vec3::Y,
            100.0,
            &QueryFilter::excluding(b),
        )
        .unwrap();
    assert!(hit.point.y.abs() < 1e-3);
    let sh = w
        .sphere_cast(
            Vec3::new(3.0, 5.0, 0.0),
            0.5,
            -Vec3::Y,
            10.0,
            &QueryFilter::default(),
        )
        .unwrap();
    assert!((sh.t - 4.5).abs() < 0.01, "{}", sh.t);
}

#[test]
fn random_pile_settles_without_explosion() {
    let mut w = PhysicsWorld::default();
    ground(&mut w);
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
        let shape = match i % 4 {
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
        bodies.push(
            w.add_body_with(BodyDesc::dynamic(Pose::new(p, q)), ColliderDesc::new(shape))
                .0,
        );
    }
    let start = std::time::Instant::now();
    let mut max_speed: f32 = 0.0;
    for i in 0..(12.0 / DT) as usize {
        w.step(DT);
        if i > 600 {
            for b in &bodies {
                max_speed = max_speed.max(w.body(*b).unwrap().lin_vel.length());
            }
        }
    }
    let ms = start.elapsed().as_secs_f32() * 1000.0 / (12.0 / DT);
    eprintln!("pile: {ms:.3} ms/step, max late speed {max_speed:.3}");
    for b in &bodies {
        let p = w.body(*b).unwrap().pose.p;
        assert!(
            p.y > -0.1 && p.y < 15.0 && p.is_finite(),
            "body escaped: {p:?}"
        );
    }
    assert!(max_speed < 3.0, "pile still jittering at {max_speed} m/s");
    let asleep = bodies
        .iter()
        .filter(|b| w.body(**b).unwrap().sleeping)
        .count();
    assert!(asleep > 150, "only {asleep} asleep");
}
