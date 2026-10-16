//! Port of tests/sim.test.ts.

mod common;

use gridlock_traffic::{MotionMode, World, WorldOptions};
use std::collections::BTreeMap;
use std::time::Instant;

#[test]
fn city_traffic_flows_for_3_simulated_minutes_without_blowing_up() {
    let mut world = World::new(WorldOptions { seed: 42, traffic: 110, start_hour: 17.5, ..Default::default() });
    let mut events = Vec::new();
    let t0 = Instant::now();
    for _ in 0..180 * 60 {
        common::step(&mut world, &mut events);
    }
    let ms = t0.elapsed().as_millis();
    let mut moving = 0;
    let mut physics = 0;
    let mut total_odo = 0.0;
    let mut states: BTreeMap<String, usize> = BTreeMap::new();
    for v in world.vehicles() {
        assert!(v.x.is_finite() && v.y.is_finite(), "vehicle {:?} has NaN position", v.id);
        if v.odometer > 100.0 {
            moving += 1;
        }
        if v.mode == MotionMode::Physics && !v.is_player {
            physics += 1;
        }
        total_odo += v.odometer;
        if let Some(d) = v.driver() {
            *states.entry(d.status(v.speed)).or_insert(0) += 1;
        }
    }
    let n = world.vehicles().len();
    let avg_speed = total_odo / n as f64 / 180.0;
    println!("sim 180s in {ms}ms; vehicles={n} moving={moving} physicsAI={physics} avgSpeed={:.1}km/h collisions={}", avg_speed * 3.6, world.stats().collisions);
    println!("events {:?}", common::count(&events));
    println!("states {states:?}");
    assert!(moving as f64 > n as f64 * 0.6, "only {moving} vehicles moved");
    assert!(common::count(&events).get("signal").copied().unwrap_or(0) > 20, "signals should cycle");
}
