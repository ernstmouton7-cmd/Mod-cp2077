//! Port of tests/peds.test.ts.

mod common;

use gridlock_traffic::{ControlType, Event, PedState, World, WorldOptions};
use std::collections::HashSet;

#[test]
fn pedestrians_cross_with_the_walk_signal_and_traffic_yields_to_them() {
    let mut world = World::new(WorldOptions { seed: 5, traffic: 110, no_player: true, start_hour: 12.0, ..Default::default() });
    world.env_mut().time_scale = 0.0;
    let mut hits = 0;
    let mut shouts = 0;
    let mut crossed_on_dont_walk = 0;
    let mut max_peds = 0;
    let mut crossings = 0;
    let mut started = HashSet::new();
    for _ in 0..60 * 300 {
        world.step(common::DT);
        for e in world.drain_events() {
            match e {
                Event::PedHit { .. } => hits += 1,
                Event::PedSpeech { .. } => shouts += 1,
                _ => {}
            }
        }
        max_peds = max_peds.max(world.peds().peds.len());
        for p in &world.peds().peds {
            if p.state == PedState::Crossing && started.insert(p.id) {
                crossings += 1;
                let cw = &world.peds().crosswalks[p.cw.idx()];
                if world.net().node(cw.node).control == ControlType::Signal && !world.walk_allowed(p.cw, true) && p.waited < 120.0 {
                    crossed_on_dont_walk += 1;
                }
            }
        }
    }
    println!("peds max={max_peds} crossings={crossings} startedOnDontWalk={crossed_on_dont_walk} hits={hits} shouts={shouts} collisions={}", world.stats().collisions);
    assert!(max_peds > 15, "pedestrians spawn");
    assert!(crossings > 100, "they cross");
    assert_eq!(crossed_on_dont_walk, 0, "nobody starts on a solid don't-walk");
    assert!(hits <= 1, "AI drivers hit {hits} pedestrians");
}
