import { Mood, OffenseKind } from "./emotion";
import { Rng } from "../math/rng";

const BY_OFFENSE: Partial<Record<OffenseKind, string[]>> = {
  cut_off: ["Use your signal!", "Whoa! Watch it!", "Are you kidding me?!", "Nice merge, genius.", "Hey! I was there!"],
  tailgating: ["Back off!", "Get off my bumper!", "Want to sit in my trunk?", "Personal space, choom!"],
  honked_at: ["Yeah, yeah, I hear you.", "Honk again, I dare you.", "Relax!", "What's your problem?"],
  long_honk: ["Okay, OKAY!", "Lay off the horn!", "You want a piece of me?!"],
  high_beams: ["Dim those lights!", "I'm blind here!", "Flash me again..."],
  blocked: ["Move it!", "Light's green, wake up!", "Any day now...", "GO!"],
  collision: ["You hit my car!", "Are you insane?!", "My paint job!", "You're paying for that!"],
  near_miss: ["Jesus! That was close!", "Watch where you're going!", "Almost killed me!"],
  rude_gesture: ["Oh, you're dead.", "Same to you, pal!", "Real mature!", "Come say that to my face!"],
  brake_check: ["What are you doing?!", "Psycho!", "Brake-checking me?!"],
  slow_driver: ["Gas pedal's on the right!", "Speed it up!", "Come ON..."],
  long_wait: ["This traffic...", "Unbelievable.", "I'm gonna be late."],
};

const BY_MOOD: Record<Mood, string[]> = {
  calm: ["...", "Easy does it."],
  irritated: ["Ugh.", "Seriously?", "Come on.", "Every single day..."],
  angry: ["Learn to drive!", "Get off the road!", "Idiot!", "Unreal!"],
  furious: ["THAT'S IT!", "You're gonna regret that!", "Pull over, coward!", "I'm coming for you!"],
};

export const APOLOGY_ACCEPTED = ["Alright, alright.", "No worries.", "Okay, fine.", "Just be careful.", "Apology accepted."];
export const APOLOGY_REJECTED = ["Sorry won't fix it!", "Yeah, you'd better be sorry!", "Too late for that!"];
export const POLICE_LINES = ["Pull over!", "Stop the vehicle!", "License and registration.", "Pull to the right!"];

export function lineFor(rng: Rng, mood: Mood, kind: OffenseKind | undefined): string {
  if (kind !== undefined && rng.chance(0.7)) {
    const l = BY_OFFENSE[kind];
    if (l !== undefined) return rng.pick(l);
  }
  return rng.pick(BY_MOOD[mood]);
}
