//! Port of tests/signals.test.ts.

use gridlock_traffic::road::ConflictKind;
use gridlock_traffic::{
    build_city, CityOptions, ConnectorId, ControlType, Indication, Network, NodeId, PedLamp,
    PhaseState, SignalController, SignalId, SignalMode, SignalOptions, TurnType,
};

fn city() -> Network {
    build_city(CityOptions {
        seed: Some(2077),
        ..Default::default()
    })
    .network
}

fn signal_nodes(net: &Network) -> Vec<NodeId> {
    net.nodes
        .iter()
        .filter(|n| n.control == ControlType::Signal)
        .map(|n| n.id)
        .collect()
}

fn controller_for(net: &mut Network, n: NodeId, mode: SignalMode) -> SignalController {
    SignalController::new(
        SignalId(0),
        net,
        n,
        SignalOptions {
            mode,
            ..Default::default()
        },
    )
}

#[test]
fn no_two_crossing_movements_are_ever_green_together() {
    let mut net = city();
    for mode in [SignalMode::Fixed, SignalMode::Actuated] {
        for n in signal_nodes(&net) {
            let mut sig = controller_for(&mut net, n, mode);
            let all = |_: ConnectorId| 1.0;
            let mut t = 0.0;
            while t < 400.0 {
                sig.update(0.1, Some(&all));
                for &c in &net.node(n).connectors {
                    let cc = net.connector(c);
                    if sig.indication(cc) != Indication::Green {
                        continue;
                    }
                    for z in &cc.conflicts {
                        if z.kind != ConflictKind::Cross {
                            continue;
                        }
                        let io = sig.indication(net.connector(z.other));
                        assert_ne!(
                            io,
                            Indication::Green,
                            "node {n:?}: {:?} {c:?} and {:?} both protected green ({mode:?})",
                            cc.turn,
                            z.other
                        );
                    }
                }
                t += 0.1;
            }
        }
    }
}

#[test]
fn phase_sequence_is_green_yellow_allred_with_speed_based_yellow() {
    let mut net = city();
    let n = signal_nodes(&net)[0];
    let mut sig = controller_for(&mut net, n, SignalMode::Fixed);
    let mut seen = Vec::new();
    let mut last = None;
    let mut yellow_start = -1.0;
    let mut yellow_durations = Vec::new();
    let mut t = 0.0;
    while t < 300.0 {
        sig.update(0.05, None);
        let st = sig.phase_state();
        if Some(st) != last {
            if st == PhaseState::Yellow {
                yellow_start = t;
            }
            if last == Some(PhaseState::Yellow) {
                yellow_durations.push(t - yellow_start);
            }
            seen.push(st);
            last = Some(st);
        }
        t += 0.05;
    }
    for w in seen.windows(2) {
        let ok = matches!(
            (w[0], w[1]),
            (PhaseState::Green, PhaseState::Yellow)
                | (PhaseState::Yellow, PhaseState::AllRed)
                | (PhaseState::AllRed, PhaseState::Green)
        );
        assert!(ok, "bad transition {:?} -> {:?}", w[0], w[1]);
    }
    assert!(!yellow_durations.is_empty());
    for d in yellow_durations {
        assert!((2.9..=5.6).contains(&d), "yellow {d:.2} s out of range");
    }
}

#[test]
fn protected_left_arrow_exists_where_a_dedicated_left_lane_exists() {
    let mut net = city();
    for n in signal_nodes(&net) {
        let sig = controller_for(&mut net, n, SignalMode::Fixed);
        let node = net.node(n);
        let has_dedicated = node.approaches.iter().any(|&a| {
            net.approach(a)
                .lanes
                .iter()
                .any(|&l| net.lane(l).turns == [TurnType::Left])
        });
        let has_arrow = sig.phases.iter().any(|p| p.is_left_arrow);
        if has_dedicated && node.approaches.len() >= 4 {
            assert!(has_arrow, "node {n:?} should have a left arrow phase");
        }
    }
}

#[test]
fn actuated_signal_rests_on_the_main_road_and_serves_side_road_demand() {
    let mut net = city();
    let n = *signal_nodes(&net)
        .iter()
        .find(|&&x| net.node(x).approaches.len() == 4)
        .unwrap();
    let mut sig = controller_for(&mut net, n, SignalMode::Actuated);
    let none = |_: ConnectorId| 0.0;
    let mut t = 0.0;
    while t < 60.0 {
        sig.update(0.1, Some(&none));
        t += 0.1;
    }
    assert!(
        sig.current_phase().recall,
        "with no demand it rests on the main-road phase"
    );
    assert_eq!(sig.phase_state(), PhaseState::Green);
    let side = sig
        .phases
        .iter()
        .position(|p| !p.recall && !p.is_left_arrow)
        .expect("side phase");
    let side_connectors: Vec<ConnectorId> = sig.phases[side].serves.iter().map(|s| s.0).collect();
    let mut served = -1.0;
    let mut t = 0.0;
    while t < 120.0 {
        let waiting = served < 0.0;
        let demand = |c: ConnectorId| {
            if side_connectors.contains(&c) && waiting {
                1.0
            } else {
                0.0
            }
        };
        sig.update(0.1, Some(&demand));
        if served < 0.0
            && sig.current_phase_index() == side
            && sig.phase_state() == PhaseState::Green
        {
            served = t;
        }
        t += 0.1;
    }
    assert!(
        (0.0..60.0).contains(&served),
        "side road served after {served:.1} s"
    );
}

#[test]
fn emergency_preemption_switches_to_the_requested_approach_quickly() {
    let mut net = city();
    let n = *signal_nodes(&net)
        .iter()
        .find(|&&x| net.node(x).approaches.len() == 4)
        .unwrap();
    let mut sig = controller_for(&mut net, n, SignalMode::Actuated);
    let all = |_: ConnectorId| 1.0;
    let mut t = 0.0;
    while t < 20.0 {
        sig.update(0.1, Some(&all));
        t += 0.1;
    }
    let target = *net
        .node(n)
        .approaches
        .iter()
        .find(|&&a| !sig.is_green_for(a))
        .unwrap();
    let mut t = 0.0;
    while !sig.is_green_for(target) && t < 30.0 {
        sig.request_preempt(target);
        sig.update(0.1, Some(&all));
        t += 0.1;
    }
    assert!(sig.is_green_for(target), "pre-empted approach gets green");
    assert!(t < 14.0, "pre-emption took {t:.1} s");
}

#[test]
fn flashing_and_dark_modes_give_the_right_indications() {
    let mut net = city();
    let n = signal_nodes(&net)[0];
    let mut sig = controller_for(&mut net, n, SignalMode::Fixed);
    sig.set_mode(SignalMode::Flashing);
    let inds: Vec<Indication> = net
        .node(n)
        .connectors
        .iter()
        .map(|&c| sig.indication(net.connector(c)))
        .collect();
    assert!(inds
        .iter()
        .all(|&i| i == Indication::FlashYellow || i == Indication::FlashRed));
    assert!(
        inds.contains(&Indication::FlashYellow) && inds.contains(&Indication::FlashRed),
        "main road flashes yellow, side roads red"
    );
    sig.set_mode(SignalMode::Off);
    assert!(net
        .node(n)
        .connectors
        .iter()
        .all(|&c| sig.indication(net.connector(c)) == Indication::Dark));
    sig.set_mode(SignalMode::Actuated);
    assert_eq!(
        sig.phase_state(),
        PhaseState::AllRed,
        "restarts with an all-red clearance"
    );
}

#[test]
fn pedestrian_head_walks_then_counts_down_during_its_through_phase() {
    let mut net = city();
    let n = signal_nodes(&net)[0];
    let mut sig = controller_for(&mut net, n, SignalMode::Fixed);
    let mut states = std::collections::HashSet::new();
    let mut countdown_seen = false;
    let mut t = 0.0;
    while t < 200.0 {
        sig.update(0.1, None);
        for &ap in &net.node(n).approaches {
            let h = sig.head(ap);
            states.insert(h.ped);
            if h.ped == PedLamp::Flash && h.ped_countdown > 0.0 {
                countdown_seen = true;
            }
        }
        t += 0.1;
    }
    assert!(
        states.contains(&PedLamp::Walk)
            && states.contains(&PedLamp::Flash)
            && states.contains(&PedLamp::Dont)
    );
    assert!(countdown_seen);
}
