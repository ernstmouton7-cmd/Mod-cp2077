import * as THREE from "three";
import { Approach } from "../core/road/network";
import { Lamp, PedLamp } from "../core/signals/signalController";
import { World } from "../core/world/world";
import { toV3 } from "./coords";
import { GeomBuilder } from "./geom";
import { lampMask } from "./textures";

type MaskKind = "ball" | "arrow" | "walk" | "hand";

interface HeadRef {
  ap: Approach;
  /** Instance indices of [red, yellow, green] in the ball or arrow mesh. */
  ball: number[] | undefined;
  arrow: number[] | undefined;
  walk: number;
  hand: number;
}

const ON: Record<string, THREE.Color> = {
  red: new THREE.Color(4.0, 0.25, 0.15),
  yellow: new THREE.Color(4.0, 2.2, 0.2),
  green: new THREE.Color(0.2, 4.0, 1.8),
  walk: new THREE.Color(3.2, 3.2, 3.0),
  hand: new THREE.Color(4.0, 1.4, 0.2),
};
const OFF: Record<string, THREE.Color> = {
  red: new THREE.Color(0.12, 0.02, 0.02),
  yellow: new THREE.Color(0.12, 0.08, 0.01),
  green: new THREE.Color(0.01, 0.1, 0.06),
  walk: new THREE.Color(0.06, 0.06, 0.06),
  hand: new THREE.Color(0.1, 0.04, 0.01),
};

/** Traffic signal hardware with live lamp states. */
export class SignalView {
  readonly group = new THREE.Group();
  private meshes: Record<MaskKind, THREE.InstancedMesh>;
  private counts: Record<MaskKind, number> = { ball: 0, arrow: 0, walk: 0, hand: 0 };
  private placements: Record<MaskKind, THREE.Matrix4[]> = { ball: [], arrow: [], walk: [], hand: [] };
  private heads: HeadRef[] = [];
  private flash = new THREE.PointLight(0xffffff, 0, 40);
  private flashTime = 0;

  constructor(readonly world: World) {
    const net = world.net;
    const hw = new GeomBuilder();
    const black = new THREE.Color(0x15161a);
    const yellowHousing = new THREE.Color(0x3a3420);
    const metal = new THREE.Color(0x5b5e66);
    for (const n of net.nodes) {
      if (n.control !== "signal") continue;
      for (const ap of n.approaches) {
        const h = ap.heading;
        const dx = Math.cos(h);
        const dy = Math.sin(h);
        const rx = Math.sin(h);
        const ry = -Math.cos(h);
        const far = n.radius + 0.8;
        const road = ap.road;
        const poleOff = road.width / 2 + 1.4;
        const px = n.x + dx * far + rx * poleOff;
        const py = n.y + dy * far + ry * poleOff;
        const pole = toV3(px, py, 0);
        hw.box(pole.x, 0, pole.z, 0.28, 6.4, 0.28, metal);
        // Mast arm reaching over the approach lanes.
        const reach = poleOff + 0.5;
        const steps = Math.ceil(reach / 1.0);
        for (let k = 0; k < steps; k++) {
          const t0 = k / steps;
          const ax = px - rx * reach * (t0 + 0.5 / steps);
          const ay = py - ry * reach * (t0 + 0.5 / steps);
          const p = toV3(ax, ay, 0);
          hw.box(p.x, 6.0, p.z, 0.2, 0.2, 0.2, metal);
        }
        const ref: HeadRef = { ap, ball: undefined, arrow: undefined, walk: -1, hand: -1 };
        // One head per lane, centred over the lane on the far side.
        const median = road.lanesAB.length > 0 && road.lanesBA.length > 0 ? road.medianWidth / 2 : 0;
        for (const lane of ap.lanes) {
          const lat = median + (lane.index + 0.5) * lane.width;
          const oneWayShift = road.lanesAB.length === 0 || road.lanesBA.length === 0 ? -((ap.lanes.length * lane.width) / 2) : 0;
          const hx = n.x + dx * far + rx * (lat + oneWayShift);
          const hy = n.y + dy * far + ry * (lat + oneWayShift);
          const hp = toV3(hx, hy, 0);
          const arrow = lane.turns.length === 1 && lane.turns[0] === "left";
          hw.box(hp.x, 4.6, hp.z, 0.42, 1.3, 0.42, arrow ? black : yellowHousing);
          hw.box(hp.x, 5.9, hp.z, 0.06, 0.2, 0.06, metal);
          // Backplate.
          const bx = hp.x + dx * 0.24;
          const bz = hp.z - dy * 0.24;
          hw.box(bx, 4.45, bz, Math.abs(dy) * 0.9 + 0.04, 1.6, Math.abs(dx) * 0.9 + 0.04, black);
          const kind: MaskKind = arrow ? "arrow" : "ball";
          const idx: number[] = [];
          for (let k = 0; k < 3; k++) {
            const y = 5.03 - k * 0.42;
            const lp = new THREE.Vector3(hp.x - dx * 0.22, y, hp.z + dy * 0.22);
            idx.push(this.place(kind, lp, h, 0.3));
          }
          if (arrow) ref.arrow = idx;
          else if (ref.ball === undefined) ref.ball = idx;
          else {
            // Additional ball heads share the first head's state: keep their indices too.
            (ref as HeadRef & { extra?: number[][] }).extra = [...((ref as HeadRef & { extra?: number[][] }).extra ?? []), idx];
          }
        }
        // Pedestrian head on the pole, facing approaching traffic.
        hw.box(pole.x - dx * 0.25, 2.6, pole.z + dy * 0.25, 0.45, 0.9, 0.45, black);
        const pp = new THREE.Vector3(pole.x - dx * 0.49, 2.8, pole.z + dy * 0.49);
        ref.hand = this.place("hand", pp, h, 0.34);
        ref.walk = this.place("walk", pp.clone().setY(2.4), h, 0.34);
        if (n.redLightCamera) hw.box(pole.x - dx * 0.2, 6.6, pole.z + dy * 0.2, 0.5, 0.35, 0.35, new THREE.Color(0xd8d8d8));
        this.heads.push(ref);
      }
    }
    this.group.add(new THREE.Mesh(hw.build(), new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.6, metalness: 0.3 })));
    const mk = (kind: MaskKind): THREE.InstancedMesh => {
      const mat = new THREE.MeshBasicMaterial({ map: lampMask(kind), transparent: true, alphaTest: 0.3, toneMapped: false, side: THREE.DoubleSide });
      const geo = new THREE.PlaneGeometry(1, 1);
      geo.rotateY(-Math.PI / 2);
      const list = this.placements[kind];
      const im = new THREE.InstancedMesh(geo, mat, Math.max(1, list.length));
      list.forEach((m, i) => {
        im.setMatrixAt(i, m);
        im.setColorAt(i, OFF.red);
      });
      im.count = list.length;
      this.group.add(im);
      return im;
    };
    this.meshes = { ball: mk("ball"), arrow: mk("arrow"), walk: mk("walk"), hand: mk("hand") };
    this.group.add(this.flash);
  }

  private place(kind: MaskKind, p: THREE.Vector3, heading: number, size: number): number {
    const m = new THREE.Matrix4();
    const q = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), heading);
    m.compose(p, q, new THREE.Vector3(1, size, size));
    this.placements[kind].push(m);
    return this.counts[kind]++;
  }

  private setTriple(mesh: THREE.InstancedMesh, idx: number[], lamp: Lamp): void {
    mesh.setColorAt(idx[0], lamp === "red" ? ON.red : OFF.red);
    mesh.setColorAt(idx[1], lamp === "yellow" ? ON.yellow : OFF.yellow);
    mesh.setColorAt(idx[2], lamp === "green" ? ON.green : OFF.green);
  }

  private setPed(walk: number, hand: number, lamp: PedLamp, blink: boolean): void {
    this.meshes.walk.setColorAt(walk, lamp === "walk" ? ON.walk : OFF.walk);
    const handOn = lamp === "dont" || (lamp === "flash" && blink);
    this.meshes.hand.setColorAt(hand, handOn ? ON.hand : OFF.hand);
  }

  /** Camera flash at a junction. */
  flashAt(x: number, y: number): void {
    this.flash.position.copy(toV3(x, y, 6));
    this.flashTime = 0.15;
  }

  update(dt: number, time: number): void {
    const blink = time % 1 < 0.5;
    for (const ref of this.heads) {
      const sig = this.world.signals[ref.ap.node.signalId];
      const st = sig.head(ref.ap);
      if (ref.ball !== undefined) this.setTriple(this.meshes.ball, ref.ball, st.ball);
      const extra = (ref as HeadRef & { extra?: number[][] }).extra;
      if (extra !== undefined) for (const e of extra) this.setTriple(this.meshes.ball, e, st.ball);
      if (ref.arrow !== undefined) {
        // An arrow head shows red arrow when not served, dark when the ball governs.
        // Permissive left during the through green: flashing yellow arrow.
        const a = st.arrow ?? "off";
        let lamp: Lamp = a;
        if (a === "off") {
          if (st.ballFlashing) lamp = st.ball;
          else if (st.ball === "green") lamp = blink ? "yellow" : "off";
          else lamp = st.ball;
        }
        this.setTriple(this.meshes.arrow, ref.arrow, lamp);
      }
      this.setPed(ref.walk, ref.hand, st.ped, blink);
    }
    for (const k of ["ball", "arrow", "walk", "hand"] as MaskKind[]) {
      const im = this.meshes[k];
      if (im.instanceColor !== null) im.instanceColor.needsUpdate = true;
    }
    if (this.flashTime > 0) {
      this.flashTime -= dt;
      this.flash.intensity = this.flashTime > 0 ? 400 : 0;
    }
  }
}
