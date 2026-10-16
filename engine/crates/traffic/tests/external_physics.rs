//! Host-physics bridge: the test plays the role of a 3D physics engine,
//! integrating physics-mode vehicles with a trivial kinematic model driven by
//! `ai_controls` / `player_controls`, while lane-mode vehicles keep their
//! kinematic lane positions.

mod common;

use gridlock_traffic::ai::DriverState;
use gridlock_traffic::{
    ActionId, Archetype, DriveControls, Event, ExternalPose, MotionMode, VehicleId, World,
    WorldOptions,
};

/// Kinematic bicycle: speed from pedals, yaw from the steering angle.
fn integrate(world: &mut World, id: VehicleId, c: DriveControls, dt: f64) {
    let v = world.vehicle(id).unwrap();
    let (mut x, mut y, mut h) = (v.x, v.y, v.heading);
    let mut u = v.dynamics.u;
    let wheelbase = v.spec.wheelbase;
    u += c.throttle as f64 * 3.0 * c.gear_dir as f64 * dt;
    let stop = (c.brake as f64 * 7.0 + c.handbrake as f64 * 5.0) * dt;
    u = if u.abs() <= stop {
        0.0
    } else {
        u - u.signum() * stop
    };
    let r = u * (c.steer_angle as f64).tan() / wheelbase;
    h += r * dt;
    x += u * h.cos() * dt;
    y += u * h.sin() * dt;
    let pose = ExternalPose {
        x: x as f32,
        y: y as f32,
        heading: h as f32,
        vx: (u * h.cos()) as f32,
        vy: (u * h.sin()) as f32,
        yaw_rate: r as f32,
    };
    assert!(world.set_external_pose(id, pose));
}

/// Integrate every external physics-mode vehicle, then step the World.
fn host_step(world: &mut World, events: &mut Vec<Event>) {
    let dt = common::DT;
    let ids: Vec<VehicleId> = world
        .vehicles()
        .iter()
        .filter(|v| v.external && v.mode == MotionMode::Physics)
        .map(|v| v.id)
        .collect();
    for id in ids {
        let c = if Some(id) == world.player_id() {
            world.player_controls()
        } else {
            world.ai_controls(id)
        };
        integrate(
            world,
            id,
            c.expect("physics-mode vehicles expose controls"),
            dt,
        );
    }
    common::step(world, events);
}

#[test]
fn host_simulated_city_with_reported_collisions() {
    let mut world = World::new(WorldOptions {
        seed: 21,
        traffic: 70,
        police: 2,
        start_hour: 12.0,
        external_physics: true,
        ..Default::default()
    });
    world.env_mut().time_scale = 0.0;
    world.breakdown_interval = 0.0;
    let mut events = Vec::new();
    assert!(
        world.vehicles().iter().all(|v| v.external),
        "every vehicle is host-simulated"
    );
    let pid = world.player_id().unwrap();
    let (x0, y0) = (world.player().unwrap().x, world.player().unwrap().y);
    // Lane-mode AI exposes no controls: its pose is the lane pose.
    let lane_ai = world
        .vehicles()
        .iter()
        .find(|v| v.is_ai() && v.mode == MotionMode::Lane)
        .unwrap()
        .id;
    assert!(world.ai_controls(lane_ai).is_none());

    // Drive the player for a few seconds through the bridge.
    world.input.set(ActionId::Throttle, 0.6);
    for _ in 0..240 {
        host_step(&mut world, &mut events);
    }
    world.input.set(ActionId::Throttle, 0.0);
    let p = world.player().unwrap();
    let moved = (p.x - x0).hypot(p.y - y0);
    assert!(moved > 10.0, "player moved {moved:.1} m under host physics");
    assert!(
        world.monitor.info.lane.is_some() || world.monitor.info.in_junction,
        "law monitor tracks the host pose"
    );
    assert_eq!(
        world.stats().collisions,
        0,
        "no internal 2D collision detection"
    );

    // Pick a calm, moving lane-mode driver; the host drives the player into its tail.
    let a = world
        .vehicles()
        .iter()
        .find(|v| {
            v.mode == MotionMode::Lane
                && v.speed > 3.0
                && v.driver().is_some_and(|d| {
                    matches!(
                        d.p.archetype,
                        Archetype::Commuter
                            | Archetype::Cautious
                            | Archetype::Zen
                            | Archetype::Professional
                    ) && d.rage.is_none()
                })
        })
        .map(|v| v.id)
        .expect("a calm moving driver");
    let va = world.vehicle(a).unwrap();
    let (h, sp) = (va.heading, va.speed + 8.0);
    let (bx, by) = (
        va.x - h.cos() * (va.half_len + 2.4),
        va.y - h.sin() * (va.half_len + 2.4),
    );
    let point = (
        (va.x - h.cos() * va.half_len) as f32,
        (va.y - h.sin() * va.half_len) as f32,
    );
    let impulse = (3.0 * va.dynamics.mass) as f32;
    world.set_external_pose(
        pid,
        ExternalPose {
            x: bx as f32,
            y: by as f32,
            heading: h as f32,
            vx: (sp * h.cos()) as f32,
            vy: (sp * h.sin()) as f32,
            yaw_rate: 0.0,
        },
    );
    // Two lane cars drifting apart are ignored, as by the internal detector.
    let other = world
        .vehicles()
        .iter()
        .find(|v| v.is_ai() && v.mode == MotionMode::Lane && v.id != a)
        .unwrap()
        .id;
    world.report_collision(other, Some(a), 1.0, point);
    assert_eq!(world.stats().collisions, 0);
    world.report_collision(pid, Some(a), impulse, point);
    events.extend(world.drain_events());
    assert_eq!(
        world.vehicle(a).unwrap().mode,
        MotionMode::Physics,
        "reported collision switches the lane car to physics"
    );
    assert!(events.iter().any(|e| matches!(e, Event::Collision { a: ea, b: eb, at_fault, .. } if *ea == pid && *eb == a && *at_fault == pid)), "collision event, player at fault");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Offense { victim, .. } if *victim == a)),
        "the victim is angry at the player"
    );
    assert_eq!(world.stats().collisions, 1);
    assert!(
        world.monitor.hit_run.is_some(),
        "law sees the player's collision"
    );
    assert_eq!(world.driver_of(a).unwrap().state, DriverState::Crashed);
    assert!(
        world.ai_controls(a).is_some(),
        "physics-mode AI exposes its controls"
    );
    assert!(
        world.vehicle(a).unwrap().damage.body() < 1.0,
        "damage applied"
    );
    // Static hit for the player (wall): damage and event.
    let p = world.player().unwrap();
    world.report_collision(
        pid,
        None,
        (2.5 * p.dynamics.mass) as f32,
        (p.x as f32 + 2.0, p.y as f32),
    );
    events.extend(world.drain_events());
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::CrashStatic { vehicle, .. } if *vehicle == pid)));

    // The host keeps integrating: crashed AI recovers and merges back into a lane.
    let mut recovered_after = None;
    for i in 0..60 * 60 {
        host_step(&mut world, &mut events);
        let v = world.vehicle(a).unwrap();
        if v.mode == MotionMode::Lane && v.driver().is_some_and(|d| d.state == DriverState::Drive) {
            recovered_after = Some(i as f64 * common::DT);
            break;
        }
    }
    println!("crashed AI rejoined its lane after {recovered_after:?} s of host-integrated driving");
    assert!(recovered_after.is_some(), "crashed AI rejoined traffic");
    assert_eq!(
        world.stats().collisions as usize,
        events
            .iter()
            .filter(|e| matches!(e, Event::Collision { .. }))
            .count(),
        "only reported collisions are counted"
    );
    for v in world.vehicles() {
        assert!(v.x.is_finite() && v.y.is_finite());
    }
}

#[test]
fn the_world_does_not_integrate_external_vehicles() {
    let mut world = World::new(WorldOptions {
        seed: 3,
        traffic: 0,
        police: 0,
        ..Default::default()
    });
    let pid = world.player_id().unwrap();
    world.set_external(pid, true);
    world.input.set(ActionId::Throttle, 1.0);
    let (x0, y0) = (world.player().unwrap().x, world.player().unwrap().y);
    for _ in 0..120 {
        world.step(common::DT);
    }
    let p = world.player().unwrap();
    assert_eq!(
        (p.x, p.y),
        (x0, y0),
        "pose only changes through set_external_pose"
    );
    let c = world.player_controls().unwrap();
    assert!(
        c.throttle > 0.5 && c.gear_dir == 1,
        "the player's controls are exposed: {c:?}"
    );
    world.set_external(pid, false);
    for _ in 0..120 {
        world.step(common::DT);
    }
    let p = world.player().unwrap();
    assert!((p.x - x0).hypot(p.y - y0) > 5.0, "internal physics resumes");
}
