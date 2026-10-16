//! Port of tests/law.test.ts.

mod common;

use common::DT;
use gridlock_traffic::law::PoliceStage;
use gridlock_traffic::world::events::PulloverStage;
use gridlock_traffic::{
    ActionId, Archetype, ConnectorId, ControlType, Event, Indication, LaneId, PhaseState, RoadKind,
    TurnType, VehicleClass, VehicleRole, ViolationKind, Witness, World, WorldOptions,
};

fn empty_world(seed: i64, police: usize) -> World {
    let mut world = World::new(WorldOptions {
        seed,
        traffic: 0,
        police,
        start_hour: 14.0,
        ..Default::default()
    });
    world.env_mut().time_scale = 0.0;
    world.env_mut().auto_weather = false;
    world
}

fn place_player(world: &mut World, lane: LaneId, s: f64, speed: f64) {
    let pt = world.net().lane(lane).path.sample(s);
    let p = world.player_mut().unwrap();
    p.dynamics.reset(pt.x, pt.y, pt.heading, speed);
    p.sync_from_dynamics();
    p.pt.shift_to(3);
    world.player_driver.as_mut().unwrap().parked = false;
    world.input.release_all();
}

fn run(world: &mut World, events: &mut Vec<Event>, seconds: f64) {
    for _ in 0..(seconds * 60.0) as usize {
        common::step(world, events);
    }
}

/// A long straight lane into a signalised 4-way junction, with its straight connector.
fn signal_approach(world: &mut World, camera: bool) -> (LaneId, ConnectorId) {
    let net = world.net_mut();
    for ni in 0..net.nodes.len() {
        let n = &net.nodes[ni];
        if n.control != ControlType::Signal || n.approaches.len() < 4 {
            continue;
        }
        net.nodes[ni].red_light_camera = camera;
        let n = &net.nodes[ni];
        for &ap in &n.approaches {
            for &l in &net.approach(ap).lanes {
                let lane = net.lane(l);
                if lane.length < 70.0 {
                    continue;
                }
                if let Some(&c) = lane
                    .outgoing
                    .iter()
                    .find(|&&k| net.connector(k).turn == TurnType::Straight)
                {
                    return (l, c);
                }
            }
        }
    }
    panic!("no approach found");
}

fn longest_avenue_lane(world: &World, curb_only: bool) -> LaneId {
    let net = world.net();
    let mut lanes: Vec<_> = net
        .lanes
        .iter()
        .filter(|l| {
            net.road(l.road).kind == RoadKind::Avenue
                && l.length > 80.0
                && (!curb_only || l.right.is_none())
        })
        .collect();
    lanes.sort_by(|a, b| b.length.partial_cmp(&a.length).unwrap());
    lanes[0].id
}

#[test]
fn running_a_red_at_a_camera_junction_is_photographed_and_fined() {
    let mut world = empty_world(9, 0);
    let mut events = Vec::new();
    let (lane, c) = signal_approach(&mut world, true);
    let sid = world
        .net()
        .node(world.net().connector(c).node)
        .signal
        .unwrap();
    let ind = |w: &World| w.signals()[sid.idx()].indication(w.net().connector(c));
    let mut guard = 0;
    while !(ind(&world) == Indication::Red
        && world.signals()[sid.idx()].phase_state() == PhaseState::Green)
        && guard < 60 * 300
    {
        common::step(&mut world, &mut events);
        guard += 1;
    }
    assert_eq!(ind(&world), Indication::Red);
    let len = world.net().lane(lane).length;
    place_player(&mut world, lane, len - 45.0, 14.0);
    world.input.set(ActionId::Throttle, 0.35);
    run(&mut world, &mut events, 7.0);
    let v = events.iter().find(|e| {
        matches!(
            e,
            Event::Violation {
                kind: ViolationKind::RedLight,
                ..
            }
        )
    });
    assert!(v.is_some(), "red light violation detected");
    assert!(
        matches!(
            v,
            Some(Event::Violation {
                witnessed: Witness::Camera,
                ..
            })
        ),
        "witnessed by the camera"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::CameraFlash { .. })),
        "camera flashed"
    );
    assert!(
        world.record.points >= 3 && world.record.money < 5000.0,
        "fine and points applied"
    );
}

#[test]
fn police_see_you_speeding_pull_you_over_and_write_a_ticket_when_you_stop() {
    let mut world = empty_world(4, 0);
    let mut events = Vec::new();
    let lane = longest_avenue_lane(&world, false);
    let cop = world.spawn_vehicle(
        VehicleClass::Police,
        VehicleRole::Police,
        lane,
        6.0,
        12.0,
        None,
    );
    place_player(&mut world, lane, 40.0, 30.0);
    world.input.set(ActionId::Throttle, 0.6);
    run(&mut world, &mut events, 6.0);
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::Violation {
                witnessed: Witness::Police,
                ..
            }
        )),
        "an officer witnessed a violation"
    );
    let mut i = 0;
    while i < 60 * 20 && world.police.stage != PoliceStage::Pullover {
        common::step(&mut world, &mut events);
        i += 1;
    }
    assert_eq!(
        world.police.stage,
        PoliceStage::Pullover,
        "police stage {:?}",
        world.police.stage
    );
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Pullover {
            stage: PulloverStage::Request,
            ..
        }
    )));
    assert!(world.vehicle(cop).unwrap().siren_on(), "siren on");
    world.input.set(ActionId::Throttle, 0.0);
    world.input.set(ActionId::Brake, 1.0);
    let money = world.record.money;
    run(&mut world, &mut events, 1.5);
    world.input.set(ActionId::Brake, 0.0);
    world.player_driver.as_mut().unwrap().parked = true;
    let mut i = 0;
    while i < 60 * 25 && !events.iter().any(|e| matches!(e, Event::Ticket { .. })) {
        common::step(&mut world, &mut events);
        i += 1;
    }
    assert!(
        events.iter().any(|e| matches!(e, Event::Ticket { .. })),
        "ticket issued"
    );
    assert!(world.record.money < money, "fine paid");
    assert_eq!(world.police.stage, PoliceStage::None, "officer leaves");
    assert_eq!(world.record.heat, 0.0);
}

#[test]
fn fleeing_a_pullover_starts_a_pursuit_with_heat_and_an_evading_charge() {
    let mut world = empty_world(4, 0);
    let mut events = Vec::new();
    let lane = longest_avenue_lane(&world, false);
    world.spawn_vehicle(
        VehicleClass::Police,
        VehicleRole::Police,
        lane,
        6.0,
        12.0,
        None,
    );
    place_player(&mut world, lane, 40.0, 30.0);
    world.input.set(ActionId::Throttle, 0.6);
    let mut i = 0;
    while i < 60 * 30 && world.police.stage != PoliceStage::Pullover {
        common::step(&mut world, &mut events);
        i += 1;
    }
    assert_eq!(world.police.stage, PoliceStage::Pullover);
    // Escape: teleport far away down another road and keep moving.
    let (px, py) = (world.player().unwrap().x, world.player().unwrap().y);
    let net = world.net();
    let far = net
        .lanes
        .iter()
        .filter(|l| l.length > 70.0)
        .max_by(|a, b| {
            let da = (a.path.xs[0] - px).hypot(a.path.ys[0] - py);
            let db = (b.path.xs[0] - px).hypot(b.path.ys[0] - py);
            da.partial_cmp(&db).unwrap().then(b.id.cmp(&a.id))
        })
        .unwrap()
        .id;
    let mut i = 0;
    while i < 60 * 20 && world.police.stage != PoliceStage::Pursuit {
        if i % 60 == 0 {
            place_player(&mut world, far, 10.0, 16.0);
        }
        world.input.set(ActionId::Throttle, 0.4);
        common::step(&mut world, &mut events);
        i += 1;
    }
    assert_eq!(world.police.stage, PoliceStage::Pursuit);
    assert!(world.record.heat >= 2.0, "heat {}", world.record.heat);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Violation {
            kind: ViolationKind::Evading,
            ..
        }
    )));
}

#[test]
fn tailgating_honking_and_gestures_push_a_hot_head_into_road_rage() {
    let mut world = empty_world(12, 0);
    let mut events = Vec::new();
    let lane = longest_avenue_lane(&world, true);
    let npc = world.spawn_vehicle(
        VehicleClass::Sedan,
        VehicleRole::Civilian,
        lane,
        50.0,
        12.0,
        Some(Archetype::Aggressor),
    );
    place_player(&mut world, lane, 38.0, 12.0);
    let pid = world.player_id().unwrap();
    let mut t = 0.0;
    let mut rage = false;
    let mut i = 0;
    while i < 60 * 40 && !rage {
        let p = world.player().unwrap();
        let n = world.vehicle(npc).unwrap();
        let gap = (n.x - p.x).hypot(n.y - p.y) - (n.length + p.length) / 2.0;
        world
            .input
            .set(ActionId::Throttle, if gap > 4.0 { 0.5 } else { 0.0 });
        world
            .input
            .set(ActionId::Brake, if gap < 2.5 { 0.6 } else { 0.0 });
        if i % 90 == 0 {
            world.input.press(ActionId::Flash);
        }
        if i % 120 == 0 {
            world.input.press(ActionId::RudeGesture);
        }
        world
            .input
            .set(ActionId::Horn, if i % 100 < 70 { 1.0 } else { 0.0 });
        common::step(&mut world, &mut events);
        t += DT;
        rage = events
            .iter()
            .any(|e| matches!(e, Event::RageStart { target, .. } if *target == pid));
        i += 1;
    }
    let d = world.driver_of(npc).unwrap();
    println!(
        "rage after {t:.1} s, anger {:.0}, mood {:?}",
        d.emo.anger, d.emo.mood
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Offense { victim, .. } if *victim == npc)),
        "the driver was offended"
    );
    assert!(rage, "road rage started against the player");
    // A sincere apology calms them down.
    world.input.set(ActionId::Horn, 0.0);
    let before = world.driver_of(npc).unwrap().emo.anger;
    world.input.press(ActionId::Apologize);
    common::step(&mut world, &mut events);
    assert!(
        world.driver_of(npc).unwrap().emo.anger < before,
        "wave lowered anger"
    );
}
