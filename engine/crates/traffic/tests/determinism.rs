//! Replaces tests/parity.test.ts (JS vs Lua): the same seed must give an
//! identical state after N steps, including the player scenario of the
//! original smoke test.

mod common;

use gridlock_traffic::{ActionId, World, WorldOptions};

fn smoke(seed: i64) -> (u64, usize, f64) {
    let mut world = World::new(WorldOptions { seed, traffic: 110, start_hour: 17.5, ..Default::default() });
    let mut events = Vec::new();
    let (x0, y0) = {
        let p = world.player().unwrap();
        (p.x, p.y)
    };
    world.input.set(ActionId::Throttle, 1.0);
    for _ in 0..360 {
        common::step(&mut world, &mut events);
    }
    let p = world.player().unwrap();
    let moved = ((p.x - x0).powi(2) + (p.y - y0).powi(2)).sqrt();
    world.input.set(ActionId::Throttle, 0.0);
    world.input.set(ActionId::Brake, 1.0);
    for _ in 0..300 {
        common::step(&mut world, &mut events);
    }
    world.input.set(ActionId::Brake, 0.0);
    world.input.press(ActionId::IndicatorLeft);
    common::step(&mut world, &mut events);
    world.input.press(ActionId::Hazards);
    common::step(&mut world, &mut events);
    world.input.press(ActionId::Headlights);
    common::step(&mut world, &mut events);
    for _ in 0..7200 {
        common::step(&mut world, &mut events);
    }
    println!("seed {seed}: moved={moved:.1} collisions={} events: {:?}", world.stats().collisions, common::count(&events));
    (world.state_hash(), events.len(), moved)
}

#[test]
fn same_seed_gives_identical_state() {
    let a = smoke(42);
    let b = smoke(42);
    assert_eq!(a, b, "same seed must reproduce the run exactly");
    assert!(a.2 > 20.0, "player moved {}", a.2);
    let c = smoke(43);
    assert_ne!(a.0, c.0, "a different seed gives a different run");
}
