//! Runs the default city for 3 simulated minutes and prints ms/step.
//!
//! `cargo run -p gridlock-traffic --release --example bench`

use gridlock_traffic::{World, WorldOptions};
use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    let mut world = World::new(WorldOptions {
        seed: 42,
        start_hour: 17.5,
        ..Default::default()
    });
    let build_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let steps = 180 * 60;
    let t1 = Instant::now();
    let mut events = 0usize;
    for _ in 0..steps {
        world.step(1.0 / 60.0);
        events += world.drain_events().len();
    }
    let ms = t1.elapsed().as_secs_f64() * 1000.0;
    println!(
        "build {build_ms:.1} ms; {steps} steps in {ms:.0} ms = {:.3} ms/step; vehicles={} peds={} events={events} collisions={}",
        ms / steps as f64,
        world.vehicles().len(),
        world.peds().peds.len(),
        world.stats().collisions
    );
}
