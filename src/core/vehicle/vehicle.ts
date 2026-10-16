import { PathSample } from "../math/path";
import { clamp } from "../math/scalar";
import { Occupant, Segment } from "../road/types";
import { DamageModel } from "./damage";
import { Assists, DriveInputs, VehicleDynamics } from "./dynamics";
import { VehicleLights } from "./lights";
import { Powertrain } from "./powertrain";
import { VehicleSpec } from "./specs";

export type VehicleRole = "civilian" | "player" | "police" | "ambulance" | "taxi";

/**
 * "lane":    kinematic, glued to a lane/connector (cheap, used by most AI)
 * "physics": full rigid-body dynamics (player, crashed cars, raging chasers)
 */
export type MotionMode = "lane" | "physics";

const tmp: PathSample = { x: 0, y: 0, heading: 0 };

export class Vehicle implements Occupant {
  readonly length: number;
  readonly width: number;
  readonly halfLen: number;
  readonly halfWid: number;
  mode: MotionMode = "lane";
  /** Pose (world). */
  x = 0;
  y = 0;
  heading = 0;
  /** Signed forward speed (m/s). */
  speed = 0;
  /** Longitudinal / lateral acceleration (m/s^2), for lights, audio, body sway. */
  accel = 0;
  latAccel = 0;
  yawRate = 0;
  /** Front wheel angle for rendering (rad, + = left). */
  wheelAngle = 0;
  /** Accumulated wheel rotation for rendering (rad). */
  wheelSpin = 0;

  // Lane-mode state.
  seg: Segment | undefined = undefined;
  /** Segment the vehicle came from (its tail may still be there). */
  prevSeg: Segment | undefined = undefined;
  s = 0;
  /** Lateral offset from the segment centreline (m, + = left). */
  lat = 0;
  latVel = 0;

  // Physics-mode state.
  readonly dyn: VehicleDynamics;
  readonly pt: Powertrain;
  readonly inputs: DriveInputs = { throttle: 0, brake: 0, steer: 0, handbrake: 0 };
  /** Desired front-wheel angle (rad, + = left) set by the controller. */
  steerTarget = 0;
  readonly assists: Assists = { abs: true, tcs: true, esc: true };

  readonly lights: VehicleLights;
  readonly damage = new DamageModel();

  readonly isPlayer: boolean;
  alive = true;
  /** Disabled by damage: no engine, hazards on. */
  wrecked = false;
  /** Seconds spent (nearly) stationary. */
  stoppedTime = 0;
  odometer = 0;
  /** Time of last collision (world time) and the other party. */
  lastHitTime = -100;
  lastHitBy = -1;
  /** Seconds since the vehicle entered physics mode (for recovery). */
  physicsTime = 0;
  /** Id of the connector this vehicle intends to take next (-1 unknown). Read by other drivers. */
  intentConnector = -1;
  /** Seconds to wait before the vehicle can be despawned. */
  despawnTimer = 0;
  /** Engine audible state for lane mode (rpm estimate). */
  rpmEstimate = 0;
  gearEstimate = 1;

  constructor(
    readonly id: number,
    readonly spec: VehicleSpec,
    readonly color: number,
    public role: VehicleRole,
  ) {
    this.length = spec.length;
    this.width = spec.width;
    this.halfLen = spec.length / 2;
    this.halfWid = spec.width / 2;
    this.dyn = new VehicleDynamics(spec);
    this.pt = new Powertrain(spec);
    this.lights = new VehicleLights(id);
    this.isPlayer = role === "player";
    if (spec.emergency) this.assists.esc = true;
  }


  isEmergency(): boolean {
    return this.role === "police" || this.role === "ambulance";
  }

  sirenOn(): boolean {
    return this.lights.siren !== "off";
  }

  absSpeed(): number {
    return this.mode === "physics" ? this.dyn.absSpeed() : Math.abs(this.speed);
  }

  /** World velocity. */
  vx(): number {
    return this.mode === "physics" ? this.dyn.vx() : Math.cos(this.heading) * this.speed;
  }

  vy(): number {
    return this.mode === "physics" ? this.dyn.vy() : Math.sin(this.heading) * this.speed;
  }

  /** Place on a segment in lane mode. */
  placeOnSegment(seg: Segment, s: number, speed: number): void {
    this.mode = "lane";
    this.seg = seg;
    this.s = s;
    this.lat = 0;
    this.latVel = 0;
    this.speed = speed;
    this.syncLanePose();
  }

  /** Update world pose from lane coordinates. */
  syncLanePose(): void {
    const seg = this.seg;
    if (seg === undefined) return;
    seg.path.sample(clamp(this.s, 0, seg.length), tmp);
    let x = tmp.x;
    let y = tmp.y;
    // Extrapolate beyond the end (rare, during transitions).
    if (this.s > seg.length) {
      x += Math.cos(tmp.heading) * (this.s - seg.length);
      y += Math.sin(tmp.heading) * (this.s - seg.length);
    }
    const nx = -Math.sin(tmp.heading);
    const ny = Math.cos(tmp.heading);
    this.x = x + nx * this.lat;
    this.y = y + ny * this.lat;
    const v = Math.max(Math.abs(this.speed), 1);
    const newHeading = tmp.heading + Math.atan2(this.latVel, v);
    let d = newHeading - this.heading;
    while (d > Math.PI) d -= Math.PI * 2;
    while (d < -Math.PI) d += Math.PI * 2;
    this.heading = this.heading + d;
  }

  /** Switch to rigid-body simulation, keeping the current motion. */
  enterPhysics(): void {
    if (this.mode === "physics") return;
    this.dyn.reset(this.x, this.y, this.heading, this.speed);
    // Carry the lateral (lane-change) velocity into the body.
    this.dyn.v = this.latVel;
    this.mode = "physics";
    this.physicsTime = 0;
    this.pt.mode = "automatic";
    if (this.pt.gear <= 0) this.pt.gear = 1;
    this.seg = undefined;
  }

  /** Return to lane mode at the given lane coordinates. */
  enterLane(seg: Segment, s: number, lat: number): void {
    const sp = this.dyn.u;
    this.mode = "lane";
    this.seg = seg;
    this.s = s;
    this.lat = lat;
    this.latVel = 0;
    this.speed = Math.max(0, sp);
    this.syncLanePose();
  }

  /** Copy pose from the dynamics body after a physics step. */
  syncFromDynamics(): void {
    const d = this.dyn;
    this.x = d.x;
    this.y = d.y;
    this.heading = d.yaw;
    this.speed = d.u;
    this.accel = d.ax;
    this.latAccel = d.ay;
    this.yawRate = d.r;
    this.wheelAngle = d.steerAngle;
  }

  /** Corners of the bounding box in world space (for collisions/rendering). */
  corners(out: number[]): number[] {
    const c = Math.cos(this.heading);
    const s = Math.sin(this.heading);
    const hl = this.halfLen;
    const hw = this.halfWid;
    let k = 0;
    for (const sx of [1, -1]) {
      for (const sy of [1, -1]) {
        const lx = sx * hl;
        const ly = sx * sy * hw;
        out[k] = this.x + lx * c - ly * s;
        out[k + 1] = this.y + lx * s + ly * c;
        k += 2;
      }
    }
    return out;
  }
}
