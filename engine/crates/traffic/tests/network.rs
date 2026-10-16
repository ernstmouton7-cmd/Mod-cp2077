//! Port of tests/network.test.ts.

use gridlock_traffic::{build_city, CityOptions, ControlType, RoadKind, Router, TurnType};

#[test]
fn city_builds_a_connected_network_with_sane_geometry() {
    let city = build_city(CityOptions {
        seed: Some(7),
        ..Default::default()
    });
    let net = &city.network;
    assert!(net.nodes.len() >= 30);
    assert!(net.lanes.len() > 100);
    assert!(net.connectors.len() > 100);
    for l in &net.lanes {
        assert!(l.length > 5.0, "lane {:?} too short: {}", l.id, l.length);
        assert!(
            !l.outgoing.is_empty(),
            "lane {:?} on {} has no exits",
            l.id,
            net.road(l.road).name
        );
        assert!(!l.turns.is_empty());
    }
    for c in &net.connectors {
        assert!(
            c.length > 0.5 && c.length < 80.0,
            "connector length {}",
            c.length
        );
        assert!(c.speed_limit > 3.0);
    }
    // Crossing movements at a 4-way must have conflicts.
    let four = net.nodes.iter().find(|n| n.roads.len() == 4).unwrap();
    assert!(four
        .connectors
        .iter()
        .any(|&c| net.connector(c).turn == TurnType::Straight
            && !net.connector(c).conflicts.is_empty()));
    println!(
        "nodes={} roads={} lanes={} connectors={} buildings={}",
        net.nodes.len(),
        net.roads.len(),
        net.lanes.len(),
        net.connectors.len(),
        city.buildings.len()
    );
    // Same generator and RNG as the TypeScript build: identical counts.
    assert_eq!(
        (
            net.nodes.len(),
            net.roads.len(),
            net.lanes.len(),
            net.connectors.len(),
            city.buildings.len()
        ),
        (42, 71, 240, 430, 110)
    );
}

#[test]
fn lane_use_and_approaches_are_assigned() {
    let city = build_city(CityOptions {
        seed: Some(3),
        ..Default::default()
    });
    let net = &city.network;
    for n in &net.nodes {
        for &ap in &n.approaches {
            let a = net.approach(ap);
            assert!(!a.lanes.is_empty());
            if n.control == ControlType::Signal {
                assert_eq!(a.sign, gridlock_traffic::ApproachSign::Signal);
            }
        }
    }
    let avenue_lane = net
        .lanes
        .iter()
        .find(|l| {
            net.road(l.road).kind == RoadKind::Avenue
                && l.index == 0
                && net.node(l.to).roads.len() == 4
        })
        .unwrap();
    assert_eq!(avenue_lane.turns, vec![TurnType::Left]);
}

#[test]
fn router_finds_routes_between_arbitrary_roads() {
    let city = build_city(CityOptions {
        seed: Some(11),
        ..Default::default()
    });
    let net = &city.network;
    let router = Router::new(net);
    let mut found = 0;
    for i in 0..40 {
        let start = &net.lanes[(i * 37) % net.lanes.len()];
        let goal = &net.lanes[(i * 91 + 13) % net.lanes.len()];
        let r = router.route(net, start.id, goal.road, goal.forward);
        if goal.road == start.road && goal.forward == start.forward {
            continue;
        }
        let r = r
            .unwrap_or_else(|| panic!("no route from lane {:?} to road {:?}", start.id, goal.road));
        // Consecutive steps must be connected.
        let mut node = start.to;
        let mut via = start.road;
        for step in &r {
            assert!(
                router.can_turn(net, node, Some(via), step.road, step.forward),
                "route contains impossible turn"
            );
            node = step.to;
            via = step.road;
        }
        found += 1;
    }
    assert!(found > 30);
}
