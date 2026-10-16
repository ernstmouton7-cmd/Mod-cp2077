/**
 * Gridlock core — engine-agnostic traffic, signals, vehicle dynamics and
 * road-rage simulation. Compiles to JavaScript (browser sandbox) and to a
 * single Lua file (Cyberpunk 2077 CET mod / any Lua engine).
 */
export { World } from "./world/world";
export type { WorldOptions, WorldStats } from "./world/world";
export type { SimEvent } from "./world/events";
export { Environment } from "./world/environment";
export type { Weather } from "./world/environment";
export { buildCity } from "./road/cityBuilder";
export type { City, Building, CityOptions } from "./road/cityBuilder";
export { Network, Lane, Connector, Node, Road, Approach } from "./road/network";
export { Router } from "./road/router";
export { SignalController } from "./signals/signalController";
export type { Indication, HeadState, Lamp, PedLamp, SignalMode } from "./signals/signalController";
export { Vehicle } from "./vehicle/vehicle";
export { VehicleDynamics } from "./vehicle/dynamics";
export { Powertrain } from "./vehicle/powertrain";
export { VehicleLights } from "./vehicle/lights";
export { SPECS, CIVILIAN_CLASSES, topSpeed, maxAccelAt } from "./vehicle/specs";
export type { VehicleSpec, VehicleClass } from "./vehicle/specs";
export { AIDriver } from "./ai/driver";
export { Emotion, OFFENSE_ANGER } from "./ai/emotion";
export type { Mood, OffenseKind } from "./ai/emotion";
export { samplePersonality, ARCHETYPES } from "./ai/personality";
export type { Personality, Archetype } from "./ai/personality";
export { idmAccel, idmFree } from "./ai/idm";
export { ACTIONS, InputMapper, actionDef } from "./player/actions";
export type { ActionId, ActionDef, ControlFrame } from "./player/actions";
export { PlayerDriver } from "./player/playerDriver";
export { DrivingRecord, VIOLATIONS } from "./law/violations";
export type { ViolationKind } from "./law/violations";
export { PlayerMonitor } from "./law/playerMonitor";
export { PoliceSystem } from "./law/police";
export { Rng } from "./math/rng";
export { EventBus } from "./util/events";

export const VERSION = "0.1.0";

import { World as WorldClass, WorldOptions as Opts } from "./world/world";

/** Factory for hosts that cannot use `new` (plain Lua: core.createWorld({ seed = 1 })). */
export function createWorld(opts?: Opts): WorldClass {
  return new WorldClass(opts ?? {});
}

// ---------------------------------------------------------------------------
// Factories for plain-Lua hosts (CET and other engines). TypeScriptToLua
// classes need `new`, which Lua callers cannot write directly.
// ---------------------------------------------------------------------------
import { VehicleLights as LightsClass } from "./vehicle/lights";
import { Emotion as EmotionClass } from "./ai/emotion";
import { Personality as PersonalityT, samplePersonality as sample, Archetype as ArchetypeT } from "./ai/personality";
import { DrivingRecord as RecordClass } from "./law/violations";
import { Rng as RngClass } from "./math/rng";
import { SignalController as SignalClass } from "./signals/signalController";
import { Network as NetworkClass } from "./road/network";

export function createLights(seed: number): LightsClass {
  return new LightsClass(seed);
}

export function createEmotion(p: PersonalityT): EmotionClass {
  return new EmotionClass(p);
}

export function createRecord(): RecordClass {
  return new RecordClass();
}

export function createRng(seed: number): RngClass {
  return new RngClass(seed);
}

/** Sample a personality (optionally of a given archetype). */
export function randomPersonality(rng: RngClass, archetype?: ArchetypeT): PersonalityT {
  return sample(rng, archetype);
}

export function createNetwork(): NetworkClass {
  return new NetworkClass();
}

/** Build a signal controller for a finalized network node (custom maps). */
export function createSignal(id: number, net: NetworkClass, nodeId: number, mode?: "fixed" | "actuated" | "flashing" | "off"): SignalClass {
  return new SignalClass(id, net.nodes[nodeId], { mode: mode ?? "actuated" });
}

export { lineFor, APOLOGY_ACCEPTED, APOLOGY_REJECTED } from "./ai/speech";
