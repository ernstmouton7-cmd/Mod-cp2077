//! Port of tests/controls.test.ts.

mod common;

use gridlock_traffic::player::GamepadBinding;
use gridlock_traffic::vehicle::TransmissionMode;
use gridlock_traffic::{ActionId, Event, InputMapper, World, WorldOptions, ACTIONS};
use std::collections::{HashMap, HashSet};

#[test]
fn every_action_has_a_unique_id_a_label_and_help_text() {
    let mut ids = HashSet::new();
    for a in ACTIONS.iter() {
        assert!(ids.insert(a.id), "duplicate {:?}", a.id);
        assert!(!a.label.is_empty() && !a.help.is_empty());
    }
    // No keyboard key drives two actions by default.
    let mut keys: HashMap<&str, ActionId> = HashMap::new();
    for a in ACTIONS.iter() {
        for &k in a.keys {
            assert!(!keys.contains_key(k), "{k} bound to {:?} and {:?}", keys[k], a.id);
            keys.insert(k, a.id);
        }
    }
    let _ = GamepadBinding::Button(0);
}

#[test]
fn keyboard_taps_are_never_lost_holds_are_tracked_pedals_ramp() {
    let mut m = InputMapper::new();
    m.key("KeyQ", true);
    m.key("KeyQ", false); // tap between frames
    let mut f = m.frame(1.0 / 60.0, 0.0);
    assert!(f.pressed.contains(&ActionId::IndicatorLeft));
    m.key("KeyW", true);
    f = m.frame(1.0 / 60.0, 0.0);
    assert!(f.throttle > 0.0 && f.throttle < 0.2, "throttle ramps up");
    for _ in 0..30 {
        f = m.frame(1.0 / 60.0, 0.0);
    }
    assert_eq!(f.throttle, 1.0);
    assert!(f.held.contains(&ActionId::Throttle));
    m.key("KeyW", false);
    f = m.frame(1.0 / 60.0, 0.0);
    assert!(f.released.iter().any(|&(id, held)| id == ActionId::Throttle && held > 0.4));
}

#[test]
fn rebinding_moves_a_key_and_steering_is_softer_at_speed() {
    let mut m = InputMapper::new();
    m.bind_keys(ActionId::Horn, &["KeyQ"]);
    assert_eq!(m.bindings(ActionId::Horn).keys, vec!["KeyQ".to_string()]);
    assert!(!m.bindings(ActionId::IndicatorLeft).keys.contains(&"KeyQ".to_string()), "key removed from its old action");
    let mut slow = InputMapper::new();
    let mut fast = InputMapper::new();
    slow.key("KeyD", true);
    fast.key("KeyD", true);
    let mut a = 0.0;
    let mut b = 0.0;
    for _ in 0..120 {
        a = slow.frame(1.0 / 60.0, 2.0).steer;
        b = fast.frame(1.0 / 60.0, 40.0).steer;
    }
    assert!(a > 0.9 && b < 0.6 && b > 0.0, "steer at 2 m/s {a:.2}, at 40 m/s {b:.2}");
}

#[test]
fn gamepad_analog_triggers_and_stick_map_to_pedals_and_steering() {
    let mut m = InputMapper::new();
    let mut buttons = vec![0.0; 17];
    buttons[7] = 0.5; // RT
    m.gamepad(&buttons, &[-0.8, 0.0, 0.0, 0.0]);
    let f = m.frame(1.0 / 60.0, 0.0);
    assert!((f.throttle - 0.5).abs() < 1e-6, "analog throttle passes through");
    assert!(f.steer < -0.1, "left stick steers left");
    buttons[0] = 1.0; // A = handbrake
    m.gamepad(&buttons, &[0.0, 0.0, 0.0, 0.0]);
    assert_eq!(m.frame(1.0 / 60.0, 0.0).handbrake, 1.0);
}

#[test]
fn manual_gearbox_without_clutch_assist_grinds_unless_the_clutch_is_pressed() {
    let mut w = World::new(WorldOptions { seed: 3, traffic: 0, police: 0, ..Default::default() });
    w.player_driver.as_mut().unwrap().settings.clutch_assist = false;
    w.input.press(ActionId::TransmissionMode);
    w.step(common::DT);
    assert_eq!(w.player().unwrap().pt.mode, TransmissionMode::Manual);
    {
        let p = w.player_mut().unwrap();
        p.pt.shift_to(0);
        p.pt.running = true;
        p.pt.stalled = false;
    }
    for _ in 0..30 {
        w.step(common::DT);
    }
    assert!(w.player().unwrap().pt.running);
    w.drain_events();
    let mut grinds = 0;
    let count = |w: &mut World| w.drain_events().iter().filter(|e| matches!(e, Event::Grind { .. })).count();
    let gear = w.player().unwrap().pt.gear;
    w.input.press(ActionId::ShiftUp);
    w.step(common::DT);
    grinds += count(&mut w);
    assert_eq!(grinds, 1, "grinds without clutch");
    assert_eq!(w.player().unwrap().pt.gear, gear, "gear did not change");
    w.input.set(ActionId::Clutch, 1.0);
    for _ in 0..30 {
        w.step(common::DT);
        grinds += count(&mut w);
    }
    w.input.press(ActionId::ShiftUp);
    for _ in 0..40 {
        w.step(common::DT);
        grinds += count(&mut w);
    }
    assert_eq!(grinds, 1);
    assert_eq!(w.player().unwrap().pt.gear, gear + 1, "shifts with the clutch in");
}
