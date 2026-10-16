import * as THREE from "three";
import { SPECS, VehicleClass, VehicleSpec } from "../core/vehicle/specs";
import { Vehicle } from "../core/vehicle/vehicle";
import { World } from "../core/world/world";
import { GeomBuilder } from "./geom";
import { beamTexture, glowTexture } from "./textures";

const CLASSES = Object.keys(SPECS) as VehicleClass[];
const CAP = 220;

interface Pose {
  x: number;
  y: number;
  h: number;
}

/** Interpolated pose snapshot used for smooth rendering between fixed steps. */
export class PoseHistory {
  readonly prev: Map<number, Pose> = new Map();

  capture(world: World): void {
    for (const v of world.vehicles) {
      let p = this.prev.get(v.id);
      if (p === undefined) {
        p = { x: v.x, y: v.y, h: v.heading };
        this.prev.set(v.id, p);
      } else {
        p.x = v.x;
        p.y = v.y;
        p.h = v.heading;
      }
    }
  }

  pose(v: Vehicle, alpha: number, out: Pose): Pose {
    const p = this.prev.get(v.id);
    if (p === undefined) {
      out.x = v.x;
      out.y = v.y;
      out.h = v.heading;
      return out;
    }
    // Large jumps (respawn/reset): don't interpolate.
    if (Math.abs(p.x - v.x) + Math.abs(p.y - v.y) > 8) {
      out.x = v.x;
      out.y = v.y;
      out.h = v.heading;
      return out;
    }
    out.x = p.x + (v.x - p.x) * alpha;
    out.y = p.y + (v.y - p.y) * alpha;
    let dh = v.heading - p.h;
    while (dh > Math.PI) dh -= Math.PI * 2;
    while (dh < -Math.PI) dh += Math.PI * 2;
    out.h = p.h + dh * alpha;
    return out;
  }

  forget(id: number): void {
    this.prev.delete(id);
  }
}

const PAINT = new THREE.Color(1, 1, 1);
const GLASS = new THREE.Color(0.06, 0.07, 0.09);
const TRIM = new THREE.Color(0.12, 0.12, 0.13);
const CHROME = new THREE.Color(0.55, 0.55, 0.6);

/** Low-poly body for a vehicle class, in local coords (+x forward, +z right, y up). */
function bodyGeometry(spec: VehicleSpec): THREE.BufferGeometry {
  const g = new GeomBuilder();
  const L = spec.length;
  const W = spec.width;
  const H = spec.height;
  const clr = spec.wheelRadius * 0.55;
  const id = spec.id;
  const box = (cx: number, y0: number, cz: number, l: number, h: number, w: number, c: THREE.Color) => g.box(cx, y0, cz, l, h, w, c);
  if (id === "truck") {
    const cabL = 2.3;
    box(L / 2 - cabL / 2, clr, 0, cabL, 1.2, W, PAINT);
    box(L / 2 - cabL / 2 - 0.15, clr + 1.2, 0, cabL - 0.3, 0.9, W * 0.96, GLASS);
    box(L / 2 - cabL / 2 - 0.25, clr + 2.1, 0, cabL - 0.5, 0.15, W * 0.96, PAINT);
    box(-cabL / 2 + 0.05, clr + 0.2, 0, L - cabL - 0.2, H - clr - 0.2, W, new THREE.Color(0.85, 0.85, 0.85));
    box(L / 2 + 0.02, clr - 0.15, 0, 0.18, 0.35, W * 1.01, TRIM);
  } else if (id === "bus") {
    box(0, clr, 0, L, 1.1, W, PAINT);
    box(0, clr + 1.1, 0, L - 0.2, 1.05, W * 1.005, GLASS);
    box(0, clr + 2.15, 0, L, H - clr - 2.15, W, PAINT);
    box(L / 2 - 0.05, clr + 0.9, 0, 0.12, 1.6, W * 0.9, GLASS);
  } else if (id === "van" || id === "ambulance") {
    const tall = H - clr;
    box(0, clr, 0, L, tall * 0.55, W, PAINT);
    box(L / 2 - 1.3, clr + tall * 0.55, 0, 1.1, tall * 0.35, W * 0.96, GLASS);
    box(-0.55, clr + tall * 0.55, 0, L - 2.4, tall * 0.45, W, PAINT);
    box(L / 2 - 1.75, clr + tall * 0.9, 0, 0.3, tall * 0.1, W * 0.96, PAINT);
    if (id === "ambulance") box(-0.6, clr + tall * 0.42, 0, L - 2.6, 0.25, W * 1.01, new THREE.Color(0.9, 0.12, 0.1));
    box(L / 2 - 0.4, clr, 0, 0.8, tall * 0.42, W * 0.98, PAINT);
  } else {
    // Cars: lower body, greenhouse (glass) with painted roof, bumpers.
    const bodyH = (id === "suv" ? 0.85 : id === "sports" ? 0.5 : 0.62) * (H / 1.45);
    const cabinH = H - clr - bodyH;
    const cabinL = L * (id === "sports" ? 0.42 : id === "suv" ? 0.62 : 0.5);
    const cabinX = id === "sports" ? -L * 0.08 : id === "suv" ? -L * 0.06 : -L * 0.04;
    box(0, clr, 0, L * 0.98, bodyH, W, PAINT);
    // Wheel-arch flares.
    box(spec.wheelbase / 2, clr + bodyH * 0.25, 0, spec.wheelRadius * 2.3, bodyH * 0.5, W * 1.02, PAINT);
    box(-spec.wheelbase / 2, clr + bodyH * 0.25, 0, spec.wheelRadius * 2.3, bodyH * 0.5, W * 1.02, PAINT);
    box(cabinX, clr + bodyH, 0, cabinL, cabinH * 0.86, W * 0.86, GLASS);
    box(cabinX - 0.05, clr + bodyH + cabinH * 0.86, 0, cabinL * 0.86, cabinH * 0.14, W * 0.8, PAINT);
    box(L / 2 - 0.02, clr - 0.05, 0, 0.14, 0.3, W * 0.98, TRIM);
    box(-L / 2 + 0.02, clr - 0.05, 0, 0.14, 0.32, W * 0.98, TRIM);
    // Grille.
    box(L / 2 - 0.03, clr + bodyH * 0.3, 0, 0.06, bodyH * 0.35, W * 0.42, id === "muscle" ? CHROME : TRIM);
    if (id === "taxi") box(cabinX, clr + bodyH + cabinH, 0, 0.5, 0.18, 0.9, new THREE.Color(1, 0.85, 0.2));
    if (id === "police") {
      box(cabinX, clr + bodyH + cabinH, 0, 0.3, 0.08, W * 0.75, TRIM);
      // White doors.
      box(cabinX + 0.1, clr + 0.15, 0, cabinL * 0.9, bodyH * 0.6, W * 1.005, new THREE.Color(0.95, 0.95, 0.95));
    }
    if (id === "sports") box(-L / 2 + 0.35, clr + bodyH, 0, 0.3, 0.08, W * 0.9, TRIM);
  }
  return g.build();
}

interface LampSpec {
  kind: "head" | "tail" | "indL" | "indR" | "rindL" | "rindR" | "rev" | "barL" | "barR" | "roof";
  x: number;
  y: number;
  z: number;
  sx: number;
  sy: number;
  sz: number;
}

function lampLayout(spec: VehicleSpec): LampSpec[] {
  const L = spec.length;
  const W = spec.width;
  const H = spec.height;
  const clr = spec.wheelRadius * 0.55;
  const big = spec.id === "truck" || spec.id === "bus" || spec.id === "van" || spec.id === "ambulance";
  const fy = clr + (big ? 0.55 : 0.32);
  const ry = clr + (big ? 0.7 : 0.42);
  const out: LampSpec[] = [];
  for (const side of [-1, 1]) {
    out.push({ kind: "head", x: L / 2 + 0.01, y: fy, z: side * (W / 2 - 0.3), sx: 0.04, sy: 0.14, sz: 0.36 });
    out.push({ kind: "tail", x: -L / 2 - 0.01, y: ry, z: side * (W / 2 - 0.22), sx: 0.04, sy: 0.14, sz: 0.3 });
    out.push({ kind: side < 0 ? "indL" : "indR", x: L / 2 - 0.02, y: fy, z: side * (W / 2 - 0.04), sx: 0.12, sy: 0.1, sz: 0.1 });
    out.push({ kind: side < 0 ? "rindL" : "rindR", x: -L / 2 - 0.015, y: ry + 0.13, z: side * (W / 2 - 0.2), sx: 0.04, sy: 0.08, sz: 0.24 });
    out.push({ kind: "rev", x: -L / 2 - 0.015, y: ry - 0.13, z: side * 0.35, sx: 0.04, sy: 0.08, sz: 0.16 });
  }
  if (spec.emergency) {
    const y = spec.id === "ambulance" ? H + 0.05 : H + 0.08;
    const x = spec.id === "ambulance" ? L / 2 - 1.6 : -spec.length * 0.04;
    out.push({ kind: "barL", x, y, z: -0.42, sx: 0.28, sy: 0.12, sz: 0.5 });
    out.push({ kind: "barR", x, y, z: 0.42, sx: 0.28, sy: 0.12, sz: 0.5 });
  }
  if (spec.id === "taxi") out.push({ kind: "roof", x: -spec.length * 0.04, y: H + 0.12, z: 0, sx: 0.46, sy: 0.12, sz: 0.86 });
  return out;
}

const C = {
  headOff: new THREE.Color(0.35, 0.35, 0.33),
  headLow: new THREE.Color(3.2, 3.1, 2.7),
  headHigh: new THREE.Color(7, 7, 6.4),
  broken: new THREE.Color(0.05, 0.05, 0.05),
  tailOff: new THREE.Color(0.25, 0.02, 0.02),
  tailOn: new THREE.Color(1.6, 0.08, 0.05),
  brake: new THREE.Color(6, 0.3, 0.2),
  indOff: new THREE.Color(0.3, 0.15, 0.02),
  indOn: new THREE.Color(6, 2.6, 0.2),
  revOff: new THREE.Color(0.25, 0.25, 0.25),
  revOn: new THREE.Color(4, 4, 4),
  red: new THREE.Color(8, 0.3, 0.3),
  blue: new THREE.Color(0.3, 0.8, 8),
  dimRed: new THREE.Color(0.3, 0.02, 0.02),
  dimBlue: new THREE.Color(0.02, 0.04, 0.3),
  roofOn: new THREE.Color(3, 2.5, 0.6),
  roofOff: new THREE.Color(0.6, 0.5, 0.1),
};

/** All vehicles, rendered with a handful of instanced draw calls. */
export class VehicleView {
  readonly group = new THREE.Group();
  private bodies: Map<VehicleClass, THREE.InstancedMesh> = new Map();
  private layouts: Map<VehicleClass, LampSpec[]> = new Map();
  private wheels: THREE.InstancedMesh;
  private lamps: THREE.InstancedMesh;
  private beams: THREE.InstancedMesh;
  private glow: THREE.Points;
  private glowPos: Float32Array;
  private glowCol: Float32Array;
  readonly headlight: THREE.SpotLight;
  readonly tailGlow: THREE.PointLight;
  private m = new THREE.Matrix4();
  private q = new THREE.Quaternion();
  private e = new THREE.Euler(0, 0, 0, "YZX");
  private v3 = new THREE.Vector3();
  private s1 = new THREE.Vector3(1, 1, 1);
  private tmpColor = new THREE.Color();
  private pose = { x: 0, y: 0, h: 0 };
  // Scratch matrices (no per-frame allocation).
  private bm = new THREE.Matrix4();
  private wm = new THREE.Matrix4();
  private tm = new THREE.Matrix4();
  private rotM = new THREE.Matrix4();
  private spinM = new THREE.Matrix4();
  private scM = new THREE.Matrix4();
  private beamQ = new THREE.Quaternion();
  private up = new THREE.Vector3(0, 1, 0);
  private dark = new THREE.Color(0.05, 0.05, 0.05);
  private beamPos = new THREE.Vector3();
  /** Smoothed body pitch/roll per vehicle. */
  private sway: Map<number, { p: number; r: number }> = new Map();
  maxDistance = 330;
  readonly bodyMat: THREE.MeshStandardMaterial;

  constructor() {
    const bodyMat = new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.32, metalness: 0.25, envMapIntensity: 0.6 });
    this.bodyMat = bodyMat;
    for (const cls of CLASSES) {
      const im = new THREE.InstancedMesh(bodyGeometry(SPECS[cls]), bodyMat, CAP);
      im.instanceColor = new THREE.InstancedBufferAttribute(new Float32Array(CAP * 3), 3);
      im.count = 0;
      im.frustumCulled = false;
      this.bodies.set(cls, im);
      this.layouts.set(cls, lampLayout(SPECS[cls]));
      this.group.add(im);
    }
    const wheelGeo = new THREE.CylinderGeometry(1, 1, 1, 14);
    wheelGeo.rotateX(Math.PI / 2);
    // Hub detail via vertex colors.
    const colors: number[] = [];
    const pos = wheelGeo.getAttribute("position");
    for (let i = 0; i < pos.count; i++) {
      const r = Math.hypot(pos.getX(i), pos.getY(i));
      const cap = Math.abs(pos.getZ(i)) > 0.49 && r < 0.6;
      colors.push(cap ? 0.6 : 0.05, cap ? 0.6 : 0.05, cap ? 0.62 : 0.055);
    }
    wheelGeo.setAttribute("color", new THREE.Float32BufferAttribute(colors, 3));
    this.wheels = new THREE.InstancedMesh(wheelGeo, new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.8 }), CAP * 4);
    this.wheels.count = 0;
    this.wheels.frustumCulled = false;
    this.lamps = new THREE.InstancedMesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial({ toneMapped: false }), CAP * 14);
    this.lamps.instanceColor = new THREE.InstancedBufferAttribute(new Float32Array(CAP * 14 * 3), 3);
    this.lamps.count = 0;
    this.lamps.frustumCulled = false;
    const beamGeo = new THREE.PlaneGeometry(1, 1);
    beamGeo.rotateX(-Math.PI / 2);
    beamGeo.rotateY(-Math.PI / 2);
    this.beams = new THREE.InstancedMesh(beamGeo, new THREE.MeshBasicMaterial({ map: beamTexture(), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, toneMapped: false }), CAP);
    this.beams.instanceColor = new THREE.InstancedBufferAttribute(new Float32Array(CAP * 3), 3);
    this.beams.count = 0;
    this.beams.frustumCulled = false;
    this.beams.renderOrder = 2;
    const n = CAP * 8;
    this.glowPos = new Float32Array(n * 3);
    this.glowCol = new Float32Array(n * 3);
    const gg = new THREE.BufferGeometry();
    gg.setAttribute("position", new THREE.BufferAttribute(this.glowPos, 3));
    gg.setAttribute("color", new THREE.BufferAttribute(this.glowCol, 3));
    this.glow = new THREE.Points(gg, new THREE.PointsMaterial({ size: 1.6, map: glowTexture(), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, vertexColors: true, sizeAttenuation: true, toneMapped: false }));
    this.glow.frustumCulled = false;
    this.glow.renderOrder = 3;
    this.group.add(this.wheels, this.lamps, this.beams, this.glow);
    this.headlight = new THREE.SpotLight(0xfff2dd, 0, 90, 0.5, 0.55, 1.2);
    this.headlight.target = new THREE.Object3D();
    this.tailGlow = new THREE.PointLight(0xff2010, 0, 9, 2);
    this.group.add(this.headlight, this.headlight.target, this.tailGlow);
  }

  private counts = new Map<VehicleClass, number>();
  /** Hide the player's own lamp glows (hood/cockpit views). */
  hidePlayerGlow = false;

  update(world: World, hist: PoseHistory, alpha: number, cam: THREE.Vector3, night: number): void {
    const counts = this.counts;
    for (const cls of CLASSES) counts.set(cls, 0);
    let wheelN = 0;
    let lampN = 0;
    let beamN = 0;
    let glowN = 0;
    const maxD2 = this.maxDistance * this.maxDistance;
    const p = world.player;
    for (const v of world.vehicles) {
      const pose = hist.pose(v, v === p ? alpha : alpha, this.pose);
      const dx = pose.x - cam.x;
      const dz = -pose.y - cam.z;
      if (dx * dx + dz * dz > maxD2) continue;
      const cls = v.spec.id;
      const idx = counts.get(cls) as number;
      if (idx >= CAP) continue;
      counts.set(cls, idx + 1);
      // Body sway from accelerations.
      let sw = this.sway.get(v.id);
      if (sw === undefined) {
        sw = { p: 0, r: 0 };
        this.sway.set(v.id, sw);
      }
      const targetP = THREE.MathUtils.clamp(v.accel * 0.007, -0.06, 0.05);
      const targetR = THREE.MathUtils.clamp(v.latAccel * 0.009, -0.07, 0.07) * (v.spec.cgHeight / 0.5);
      sw.p += (targetP - sw.p) * 0.15;
      sw.r += (targetR - sw.r) * 0.15;
      this.e.set(sw.r, pose.h, sw.p, "YZX");
      this.q.setFromEuler(this.e);
      this.v3.set(pose.x, 0.02, -pose.y);
      this.m.compose(this.v3, this.q, this.s1);
      const body = this.bodies.get(cls) as THREE.InstancedMesh;
      body.setMatrixAt(idx, this.m);
      const dmg = v.damage.body();
      this.tmpColor.setHex(v.color).convertSRGBToLinear();
      if (dmg < 1) this.tmpColor.lerp(this.dark, (1 - dmg) * 0.6);
      body.setColorAt(idx, this.tmpColor);
      const bodyMatrix = this.bm.copy(this.m);
      // Wheels.
      const spec = v.spec;
      const wr = spec.wheelRadius;
      const wx = spec.wheelbase / 2;
      const wz = spec.width / 2 - 0.18;
      for (let k = 0; k < 4; k++) {
        const front = k < 2;
        const side = k % 2 === 0 ? -1 : 1;
        const steer = front ? v.wheelAngle : 0;
        const wm = this.wm.makeTranslation(front ? wx : -wx, wr, side * wz);
        wm.multiply(this.rotM.makeRotationY(steer)).multiply(this.spinM.makeRotationZ(-v.wheelSpin)).multiply(this.scM.makeScale(wr, wr, 0.26));
        this.wheels.setMatrixAt(wheelN++, this.tm.multiplyMatrices(bodyMatrix, wm));
      }
      // Lamps.
      const lights = v.lights;
      const lowOn = lights.lowBeamOn();
      const highOn = lights.highBeamOn();
      const leftOn = lights.leftLampOn();
      const rightOn = lights.rightLampOn();
      const brake = lights.brake > 0.5;
      const bar = lights.lightbar;
      const barPhase = lights.lightbarPhase();
      const layout = this.layouts.get(cls) as LampSpec[];
      for (const L of layout) {
        let c: THREE.Color;
        switch (L.kind) {
          case "head": {
            const broken = (L.z < 0 && lights.leftHeadlightBroken) || (L.z > 0 && lights.rightHeadlightBroken);
            c = broken ? C.broken : highOn ? C.headHigh : lowOn ? C.headLow : C.headOff;
            break;
          }
          case "tail":
            c = brake ? C.brake : lowOn ? C.tailOn : C.tailOff;
            break;
          case "indL":
          case "rindL":
            c = leftOn ? C.indOn : C.indOff;
            break;
          case "indR":
          case "rindR":
            c = rightOn ? C.indOn : C.indOff;
            break;
          case "rev":
            c = lights.reverse ? C.revOn : C.revOff;
            break;
          case "barL":
            c = bar ? (barPhase === 0 ? C.red : C.dimRed) : C.dimRed;
            break;
          case "barR":
            c = bar ? (barPhase === 1 ? C.blue : C.dimBlue) : C.dimBlue;
            break;
          default:
            c = lowOn ? C.roofOn : C.roofOff;
        }
        const lm = this.wm.makeTranslation(L.x, L.y, L.z).multiply(this.scM.makeScale(L.sx, L.sy, L.sz));
        this.lamps.setMatrixAt(lampN, this.tm.multiplyMatrices(bodyMatrix, lm));
        this.lamps.setColorAt(lampN, c);
        lampN++;
        // Glow sprites for lit lamps.
        const lit = c === C.headLow || c === C.headHigh || c === C.tailOn || c === C.brake || c === C.indOn || c === C.revOn || c === C.red || c === C.blue;
        if (lit && glowN < CAP * 8 && !(v === p && this.hidePlayerGlow)) {
          this.v3.set(L.x + (L.x > 0 ? 0.15 : -0.15), L.y, L.z).applyMatrix4(bodyMatrix);
          const gi = glowN * 3;
          this.glowPos[gi] = this.v3.x;
          this.glowPos[gi + 1] = this.v3.y;
          this.glowPos[gi + 2] = this.v3.z;
          const k = (c === C.headHigh ? 0.6 : c === C.headLow ? 0.32 : c === C.tailOn ? 0.18 : 0.35) * (0.35 + night * 0.65);
          this.glowCol[gi] = c.r * k;
          this.glowCol[gi + 1] = c.g * k;
          this.glowCol[gi + 2] = c.b * k;
          glowN++;
        }
      }
      // Headlight pool on the road at night.
      if ((lowOn || highOn) && night > 0.15 && !(lights.leftHeadlightBroken && lights.rightHeadlightBroken)) {
        const len = highOn ? 26 : 15;
        const local = this.wm.makeTranslation(spec.length / 2 + len / 2 - 0.5, 0, 0).multiply(this.scM.makeScale(len, 1, spec.width * (highOn ? 2.6 : 2.2)));
        this.beamPos.set(pose.x, 0.06, -pose.y);
        this.beamQ.setFromAxisAngle(this.up, pose.h);
        this.tm.compose(this.beamPos, this.beamQ, this.s1);
        this.beams.setMatrixAt(beamN, this.tm.multiply(local));
        const k = (highOn ? 0.55 : 0.4) * night;
        this.tmpColor.setRGB(k, k * 0.96, k * 0.88);
        this.beams.setColorAt(beamN, this.tmpColor);
        beamN++;
      }
      // Real spot light for the player's headlights.
      if (v === p) {
        const hl = this.headlight;
        const on = lowOn || highOn;
        hl.intensity = on ? (highOn ? 260 : 150) * (0.25 + night * 0.75) : 0;
        hl.distance = highOn ? 110 : 70;
        hl.angle = highOn ? 0.42 : 0.55;
        this.v3.set(spec.length / 2, 0.9, 0).applyMatrix4(bodyMatrix);
        hl.position.copy(this.v3);
        this.v3.set(spec.length / 2 + 20, -1.5, 0).applyMatrix4(bodyMatrix);
        hl.target.position.copy(this.v3);
        this.v3.set(-spec.length / 2 - 0.6, 0.7, 0).applyMatrix4(bodyMatrix);
        this.tailGlow.position.copy(this.v3);
        this.tailGlow.intensity = brake ? 6 : lowOn ? 1.5 : 0;
      }
    }
    for (const cls of CLASSES) {
      const im = this.bodies.get(cls) as THREE.InstancedMesh;
      im.count = counts.get(cls) as number;
      im.instanceMatrix.needsUpdate = true;
      if (im.instanceColor !== null) im.instanceColor.needsUpdate = true;
    }
    this.wheels.count = wheelN;
    this.wheels.instanceMatrix.needsUpdate = true;
    this.lamps.count = lampN;
    this.lamps.instanceMatrix.needsUpdate = true;
    if (this.lamps.instanceColor !== null) this.lamps.instanceColor.needsUpdate = true;
    this.beams.count = beamN;
    this.beams.instanceMatrix.needsUpdate = true;
    if (this.beams.instanceColor !== null) this.beams.instanceColor.needsUpdate = true;
    const g = this.glow.geometry;
    g.setDrawRange(0, glowN);
    (g.getAttribute("position") as THREE.BufferAttribute).needsUpdate = true;
    (g.getAttribute("color") as THREE.BufferAttribute).needsUpdate = true;
    // Drop sway entries of vehicles that left.
    if (this.sway.size > world.vehicles.length * 2) {
      const alive = new Set(world.vehicles.map((v) => v.id));
      for (const id of [...this.sway.keys()]) if (!alive.has(id)) this.sway.delete(id);
    }
  }
}
