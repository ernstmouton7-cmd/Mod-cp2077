import { OffenseKind, Mood } from "../ai/emotion";

/**
 * Every gameplay event the simulation emits. Renderers, audio and mod
 * adapters consume these (EventBus.drain() once per frame, or on()).
 */
export type SimEvent =
  | { type: "horn"; vehicle: number; long: boolean }
  | { type: "flash"; vehicle: number }
  | { type: "speech"; vehicle: number; text: string; mood: Mood }
  | { type: "offense"; offender: number; victim: number; kind: OffenseKind; anger: number }
  | { type: "mood"; vehicle: number; mood: Mood; target: number }
  | { type: "rage_start"; vehicle: number; target: number; behavior: "tailgate" | "brake_check" | "ram" | "chase" }
  | { type: "rage_end"; vehicle: number; reason: string }
  | { type: "collision"; a: number; b: number; impulse: number; severity: number; x: number; y: number; atFault: number }
  | { type: "crash_static"; vehicle: number; severity: number; x: number; y: number }
  | { type: "near_miss"; a: number; b: number }
  | { type: "signal"; node: number; phase: string; state: string }
  | { type: "violation"; vehicle: number; kind: string; label: string; fine: number; points: number; witnessed: string; x: number; y: number }
  | { type: "ticket"; vehicle: number; total: number; points: number; reason: string }
  | { type: "camera_flash"; x: number; y: number; kind: "red_light" | "speed" }
  | { type: "heat"; level: number; reason: string }
  | { type: "pullover"; police: number; stage: "request" | "stopped" | "issued" | "fled" | "escaped" }
  | { type: "dispatch"; unit: number; kind: "police" | "ambulance"; x: number; y: number; reason: string }
  | { type: "apology"; from: number; to: number; calmed: number }
  | { type: "gesture"; from: number; kind: "wave" | "rude" }
  | { type: "stall"; vehicle: number }
  | { type: "engine"; vehicle: number; on: boolean }
  | { type: "shift"; vehicle: number; gear: string }
  | { type: "spawn"; vehicle: number }
  | { type: "despawn"; vehicle: number }
  | { type: "info"; text: string };
