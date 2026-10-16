//! Shared helpers for the integration tests.
#![allow(dead_code)]

use gridlock_traffic::{Event, World};
use std::collections::BTreeMap;

pub const DT: f64 = 1.0 / 60.0;

/// Step once and collect the emitted events.
pub fn step(world: &mut World, events: &mut Vec<Event>) {
    world.step(DT);
    events.extend(world.drain_events());
}

/// Count events by type name.
pub fn count(events: &[Event]) -> BTreeMap<&'static str, usize> {
    let mut m = BTreeMap::new();
    for e in events {
        *m.entry(e.type_name()).or_insert(0) += 1;
    }
    m
}
