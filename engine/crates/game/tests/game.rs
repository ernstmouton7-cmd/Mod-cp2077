use gridlock_game::coords::*;
use gridlock_game::*;
use gridlock_input::ControlFrame;
use gridlock_physics::glam::Vec3;
use gridlock_physics::Pose;
use gridlock_traffic::{Event, MotionMode};

fn drive(g: &mut Game, seconds: f32, throttle: f32, steer: f32) {
    let frame = ControlFrame {
        throttle,
        steer,
        ..Default::default()
    };
    for _ in 0..(seconds * 60.0) as usize {
        g.step(&frame);
    }
}

#[test]
fn city_runs_with_physics_traffic() {
    let mut g = Game::new(GameConfig {
        traffic: 60,
        ..Default::default()
    });
    let start = g.scene().player_pose.unwrap().p;
    let t0 = std::time::Instant::now();
    drive(&mut g, 2.0, 0.0, 0.0);
    drive(&mut g, 40.0, 0.45, 0.0);
    let ms = t0.elapsed().as_secs_f32() * 1000.0 / (42.0 * 60.0);
    let scene = g.scene();
    eprintln!(
        "{} cars ({} physics), {} props, {} peds, {} signal heads, {:.2} ms/step",
        scene.cars.len(),
        scene.cars.iter().filter(|c| c.physics).count(),
        scene.props.len(),
        scene.peds.len(),
        scene.signals.len(),
        ms
    );
    assert!(scene.cars.len() >= 40);
    for c in &scene.cars {
        assert!(
            c.pose.p.is_finite() && c.pose.p.y > -1.0 && c.pose.p.y < 5.0,
            "car {} at {:?}",
            c.id,
            c.pose.p
        );
    }
    let moving = scene.cars.iter().filter(|c| c.speed.abs() > 1.0).count();
    assert!(moving > scene.cars.len() / 3, "only {moving} cars moving");
    let player = scene.player_pose.unwrap().p;
    assert!(
        (player - start).length() > 30.0,
        "player barely moved: {start:?} -> {player:?}"
    );
    // Puppets track their lanes.
    let mut worst: f32 = 0.0;
    for v in g.traffic.vehicles() {
        if v.mode == MotionMode::Lane {
            if let Some(c) = scene.cars.iter().find(|c| c.id == v.id.0) {
                worst = worst.max((c.pose.p - to_world(v.x, v.y, 0.0)).length());
            }
        }
    }
    assert!(
        worst < 0.5,
        "a lane car drifted {worst} m from its lane pose"
    );
}

#[test]
fn rear_ending_a_traffic_car_turns_it_into_a_physics_crash() {
    let mut g = Game::new(GameConfig {
        traffic: 50,
        props: false,
        ..Default::default()
    });
    drive(&mut g, 3.0, 0.0, 0.0);
    // Pick a moving lane car and put the player 14 m behind it, closing at +12 m/s.
    let target = g
        .traffic
        .vehicles()
        .iter()
        .find(|v| v.mode == MotionMode::Lane && v.speed > 5.0 && !v.is_player)
        .map(|v| (v.id, v.x, v.y, v.heading, v.speed))
        .expect("a moving car");
    let (id, x, y, h, speed) = target;
    let q = rotation_for_heading(h);
    let fwd = q * -Vec3::Z;
    let behind = to_world(x, y, 0.0) - fwd * 14.0;
    let pv = g.player_vehicle().unwrap();
    let chassis = g.physics.vehicle(pv).unwrap().chassis;
    g.physics
        .set_pose(chassis, Pose::new(behind + Vec3::Y * 0.02, q));
    g.physics
        .set_velocity(chassis, fwd * (speed as f32 + 12.0), Vec3::ZERO);
    for w in &mut g.physics.vehicle_mut(pv).unwrap().wheels {
        w.omega = (speed as f32 + 12.0) / 0.32;
    }
    let mut collided = false;
    for _ in 0..120 {
        g.step(&ControlFrame {
            throttle: 0.3,
            ..Default::default()
        });
        for e in &g.events {
            if let GameEvent::Traffic(Event::Collision { a, b, .. }) = e {
                if *a == id || *b == id {
                    collided = true;
                }
            }
        }
    }
    assert!(collided, "traffic should register the 3D collision");
    let v = g.traffic.vehicle(id).expect("still exists");
    assert_eq!(
        v.mode,
        MotionMode::Physics,
        "victim should be in physics mode"
    );
    assert!(matches!(g.links[&id].state, CarState::Physics { .. }));
    let dmg = g.damage_of(id).unwrap();
    let dent = dmg
        .body
        .positions
        .iter()
        .zip(&dmg.body.rest)
        .map(|(p, r)| (*p - *r).length())
        .fold(0.0, f32::max);
    eprintln!("victim rear dent {dent:.3} m");
}
