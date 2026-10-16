//! Port of tests/incidents.test.ts.

mod common;

use gridlock_traffic::ai::HoldKind;
use gridlock_traffic::{Archetype, RoadKind, SegRef, SignalMode, VehicleClass, VehicleId, VehicleRole, World, WorldOptions};
use std::collections::HashMap;

#[test]
fn traffic_overtakes_a_broken_down_car_on_a_single_lane_street() {
    let mut world = World::new(WorldOptions { seed: 8, traffic: 0, police: 0, no_player: true, start_hour: 12.0, ..Default::default() });
    world.env_mut().time_scale = 0.0;
    world.breakdown_interval = 0.0;
    world.outage_interval = 0.0;
    world.pedestrian_density = 0.0;
    world.spawner.enabled = false;
    let net = world.net();
    let mut lanes: Vec<_> = net
        .lanes
        .iter()
        .filter(|l| {
            let r = net.road(l.road);
            r.kind == RoadKind::Street && r.opts.lanes_fwd == 1 && r.opts.lanes_back == 1 && l.length > 80.0
        })
        .collect();
    lanes.sort_by(|a, b| b.length.partial_cmp(&a.length).unwrap());
    let lane = lanes.first().expect("found a single-lane two-way street").id;
    let len = net.lane(lane).length;
    let broken = world.spawn_vehicle(VehicleClass::Sedan, VehicleRole::Civilian, lane, len * 0.55, 0.0, None);
    world.driver_of_mut(broken).unwrap().hold(HoldKind::Breakdown, 120.0);
    let followers = [
        world.spawn_vehicle(VehicleClass::Compact, VehicleRole::Civilian, lane, 8.0, 8.0, Some(Archetype::Commuter)),
        world.spawn_vehicle(VehicleClass::Suv, VehicleRole::Civilian, lane, 18.0, 8.0, Some(Archetype::Speeder)),
    ];
    let start_odo: Vec<f64> = followers.iter().map(|&f| world.vehicle(f).unwrap().odometer).collect();
    let mut passing_seen = false;
    for _ in 0..60 * 60 {
        world.step(common::DT);
        world.drain_events();
        for &f in &followers {
            if world.driver_of(f).is_some_and(|d| d.passing.is_some()) {
                passing_seen = true;
            }
        }
    }
    let overtook = followers.iter().zip(&start_odo).filter(|(&f, &o)| world.vehicle(f).unwrap().odometer - o > len).count();
    let b = world.vehicle(broken).unwrap();
    assert!(b.lights.hazard, "broken-down car shows hazards");
    assert!(b.speed < 0.3, "broken-down car stays put");
    assert!(passing_seen, "followers used the oncoming lane");
    assert_eq!(overtook, 2, "both followers got past");
    assert_eq!(world.stats().collisions, 0);
}

#[test]
fn a_power_cut_turns_a_junction_into_an_all_way_stop_without_deadlock() {
    let mut world = World::new(WorldOptions { seed: 5, traffic: 110, no_player: true, start_hour: 17.0, ..Default::default() });
    world.env_mut().time_scale = 0.0;
    world.outage_interval = 0.0;
    // Busiest signal: most approaches (first in id order on ties, like a stable sort).
    let net = world.net();
    let si = (0..world.signals().len()).max_by(|&a, &b| {
        let na = net.node(world.signals()[a].node).approaches.len();
        let nb = net.node(world.signals()[b].node).approaches.len();
        na.cmp(&nb).then(b.cmp(&a))
    });
    let si = si.unwrap();
    let node = world.signals()[si].node;
    for _ in 0..60 * 30 {
        world.step(common::DT);
        world.drain_events();
    }
    world.signals_mut()[si].set_mode(SignalMode::Off);
    let mut through = 0;
    let mut prev: HashMap<VehicleId, Option<SegRef>> = HashMap::new();
    let mut stopped: HashMap<VehicleId, f64> = HashMap::new();
    let mut max_stopped: f64 = 0.0;
    for _ in 0..60 * 180 {
        world.step(common::DT);
        world.drain_events();
        for v in world.vehicles() {
            if let Some(SegRef::Connector(c)) = v.seg {
                if prev.get(&v.id).copied().flatten() != v.seg && world.net().connector(c).node == node {
                    through += 1;
                }
            }
            prev.insert(v.id, v.seg);
            let s = if v.abs_speed() < 0.2 { stopped.get(&v.id).copied().unwrap_or(0.0) + common::DT } else { 0.0 };
            stopped.insert(v.id, s);
            max_stopped = max_stopped.max(s);
        }
    }
    println!("dark junction: {through} vehicles through in 3 min, longest wait {max_stopped:.0} s, collisions {}", world.stats().collisions);
    assert!(through > 40, "traffic keeps flowing through the dark junction");
    assert!(max_stopped < 150.0, "no deadlock");
}
