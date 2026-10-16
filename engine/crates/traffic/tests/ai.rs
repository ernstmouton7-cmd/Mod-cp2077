//! Port of tests/ai.test.ts.

mod common;

use gridlock_traffic::{idm_accel, idm_free, sample_personality, Archetype, Emotion, Event, IdmParams, Indication, Mood, OffenseKind, PhaseState, Rng, SegRef, TurnType, VehicleId, World, WorldOptions};
use std::collections::{HashMap, HashSet};

const P: IdmParams = IdmParams { v0: 14.0, t: 1.4, a: 1.5, b: 2.0, s0: 2.0 };

#[test]
fn idm_reaches_desired_speed_and_stops_behind_an_obstacle_without_overlap() {
    let mut v: f64 = 0.0;
    for _ in 0..60 * 60 {
        v += idm_free(v, &P) / 60.0;
    }
    assert!((v - 14.0).abs() < 0.3, "free speed {v:.2}");
    let mut x = 0.0;
    v = 14.0;
    let wall = 200.0;
    let mut min_gap = f64::INFINITY;
    for _ in 0..60 * 60 {
        let gap = wall - x;
        v = (v + idm_accel(v, gap, v, &P) / 60.0).max(0.0);
        x += v / 60.0;
        min_gap = min_gap.min(wall - x);
    }
    assert!(min_gap > 1.2 && min_gap < 3.0, "standstill gap {min_gap:.2} m");
}

#[test]
fn emotions_escalate_decay_and_apologies_help() {
    let mut rng = Rng::new(3);
    let mut hot = Emotion::new(sample_personality(&mut rng, Some(Archetype::Aggressor)));
    let mut zen = Emotion::new(sample_personality(&mut rng, Some(Archetype::Zen)));
    let o = VehicleId(9);
    for e in [&mut hot, &mut zen] {
        e.offend(Some(o), OffenseKind::CutOff, 1.5, 0.0);
        e.offend(Some(o), OffenseKind::LongHonk, 1.0, 1.0);
        e.offend(Some(o), OffenseKind::RudeGesture, 1.0, 2.0);
        for i in 0..10 {
            e.update(0.1, 2.0 + i as f64 * 0.1, 0.0);
        }
    }
    assert!(hot.anger > zen.anger * 3.0, "hot-head {:.0} vs zen {:.0}", hot.anger, zen.anger);
    assert_eq!(hot.mood, Mood::Furious);
    assert_eq!(hot.target, Some(o));
    assert!(hot.can_rage());
    let before = hot.anger;
    hot.apologize(o, 3.5);
    assert!(hot.anger < before);
    let mut t = 0.0;
    while t < 120.0 {
        hot.update(0.1, 4.0 + t, 0.0);
        t += 0.1;
    }
    assert_eq!(hot.mood, Mood::Calm, "anger fades over two minutes");
}

#[derive(Default, Debug)]
struct Stats {
    entries: usize,
    red_entries: usize,
    max_stopped: f64,
    collisions: usize,
    lc_vehicles: usize,
}

fn city_run(seconds: usize, seed: i64) -> (World, Stats) {
    let mut world = World::new(WorldOptions { seed, traffic: 110, no_player: true, start_hour: 17.0, ..Default::default() });
    world.env_mut().time_scale = 0.0;
    let mut prev_seg: HashMap<VehicleId, Option<SegRef>> = HashMap::new();
    let mut stopped_for: HashMap<VehicleId, f64> = HashMap::new();
    let mut lc: HashSet<VehicleId> = HashSet::new();
    let mut st = Stats::default();
    for _ in 0..seconds * 60 {
        world.step(common::DT);
        st.collisions += world.drain_events().iter().filter(|e| matches!(e, Event::Collision { .. })).count();
        let net = world.net();
        for v in world.vehicles() {
            let Some(d) = v.driver() else { continue };
            if d.shadow_lane.is_some() {
                lc.insert(v.id);
            }
            let before = prev_seg.get(&v.id).copied().flatten();
            if let (Some(SegRef::Connector(c)), Some(SegRef::Lane(_))) = (v.seg, before) {
                if v.seg != before {
                    let conn = net.connector(c);
                    let node = net.node(conn.node);
                    if node.control == gridlock_traffic::ControlType::Signal {
                        st.entries += 1;
                        let sig = &world.signals()[node.signal.unwrap().idx()];
                        if sig.indication(conn) == Indication::Red && sig.phase_state() != PhaseState::AllRed && conn.turn != TurnType::Right {
                            st.red_entries += 1;
                        }
                    }
                }
            }
            prev_seg.insert(v.id, v.seg);
            let s = if v.abs_speed() < 0.2 { stopped_for.get(&v.id).copied().unwrap_or(0.0) + common::DT } else { 0.0 };
            stopped_for.insert(v.id, s);
            if s > st.max_stopped {
                st.max_stopped = s;
            }
        }
    }
    st.lc_vehicles = lc.len();
    (world, st)
}

#[test]
fn ten_minutes_of_city_traffic_lights_obeyed_no_gridlock_few_crashes() {
    let (world, st) = city_run(600, 5);
    let odo: f64 = world.vehicles().iter().map(|v| v.odometer).sum();
    let n = world.vehicles().len() as f64;
    let veh_hours = (n * 600.0) / 3600.0;
    println!(
        "entries={} redRuns={} ({:.2}%) maxStopped={:.0}s collisions={} ({:.2}/veh-h) laneChangers={} avgOdo={:.0}m",
        st.entries,
        st.red_entries,
        st.red_entries as f64 / st.entries as f64 * 100.0,
        st.max_stopped,
        st.collisions,
        st.collisions as f64 / veh_hours,
        st.lc_vehicles,
        odo / n
    );
    assert!(st.entries > 500, "lots of junction traffic");
    assert!((st.red_entries as f64) / (st.entries as f64) < 0.03, "fewer than 3% enter on red");
    assert!(st.max_stopped < 200.0, "longest standstill {:.0} s (gridlock?)", st.max_stopped);
    assert!((st.collisions as f64) / veh_hours < 3.0, "crash rate stays low");
    assert!(st.lc_vehicles > 20, "drivers change lanes");
}
