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
