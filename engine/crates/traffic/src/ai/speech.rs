//! What drivers shout.

use super::emotion::{Mood, OffenseKind};
use crate::math::Rng;

fn by_offense(kind: OffenseKind) -> &'static [&'static str] {
    match kind {
        OffenseKind::CutOff => &["Use your signal!", "Whoa! Watch it!", "Are you kidding me?!", "Nice merge, genius.", "Hey! I was there!"],
        OffenseKind::Tailgating => &["Back off!", "Get off my bumper!", "Want to sit in my trunk?", "Personal space, choom!"],
        OffenseKind::HonkedAt => &["Yeah, yeah, I hear you.", "Honk again, I dare you.", "Relax!", "What's your problem?"],
        OffenseKind::LongHonk => &["Okay, OKAY!", "Lay off the horn!", "You want a piece of me?!"],
        OffenseKind::HighBeams => &["Dim those lights!", "I'm blind here!", "Flash me again..."],
        OffenseKind::Blocked => &["Move it!", "Light's green, wake up!", "Any day now...", "GO!"],
        OffenseKind::Collision => &["You hit my car!", "Are you insane?!", "My paint job!", "You're paying for that!"],
        OffenseKind::NearMiss => &["Jesus! That was close!", "Watch where you're going!", "Almost killed me!"],
        OffenseKind::RudeGesture => &["Oh, you're dead.", "Same to you, pal!", "Real mature!", "Come say that to my face!"],
        OffenseKind::BrakeCheck => &["What are you doing?!", "Psycho!", "Brake-checking me?!"],
        OffenseKind::SlowDriver => &["Gas pedal's on the right!", "Speed it up!", "Come ON..."],
        OffenseKind::LongWait => &["This traffic...", "Unbelievable.", "I'm gonna be late."],
    }
}

fn by_mood(m: Mood) -> &'static [&'static str] {
    match m {
        Mood::Calm => &["...", "Easy does it."],
        Mood::Irritated => &["Ugh.", "Seriously?", "Come on.", "Every single day..."],
        Mood::Angry => &["Learn to drive!", "Get off the road!", "Idiot!", "Unreal!"],
        Mood::Furious => &["THAT'S IT!", "You're gonna regret that!", "Pull over, coward!", "I'm coming for you!"],
    }
}

pub const APOLOGY_ACCEPTED: [&str; 5] = ["Alright, alright.", "No worries.", "Okay, fine.", "Just be careful.", "Apology accepted."];
pub const APOLOGY_REJECTED: [&str; 3] = ["Sorry won't fix it!", "Yeah, you'd better be sorry!", "Too late for that!"];
pub const POLICE_LINES: [&str; 4] = ["Pull over!", "Stop the vehicle!", "License and registration.", "Pull to the right!"];

/// A line for a driver in `mood` reacting to `kind`.
pub fn line_for(rng: &mut Rng, mood: Mood, kind: Option<OffenseKind>) -> &'static str {
    if let Some(k) = kind {
        if rng.chance(0.7) {
            return rng.pick(by_offense(k));
        }
    }
    rng.pick(by_mood(mood))
}
