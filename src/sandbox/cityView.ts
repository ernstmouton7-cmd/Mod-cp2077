import * as THREE from "three";
import { City } from "../core/road/cityBuilder";
import { Lane, Network, Node, Road } from "../core/road/network";
import { World } from "../core/world/world";
import { toV3 } from "./coords";
import { GeomBuilder } from "./geom";
import { asphaltTexture, beamTexture, concreteTexture, facadeTextures, glowTexture, laneArrowTexture, neonTexture, signTexture } from "./textures";

const WHITE = new THREE.Color(0xe8e8e2);
const YELLOW = new THREE.Color(0xf0b429);
const SIDEWALK = new THREE.Color(0x8b8c94);
const CURB = new THREE.Color(0xa6a7ad);
const MEDIAN = new THREE.Color(0x6f7077);

const NEON_WORDS = ["RAMEN", "MOTEL", "24/7", "CLINIC", "ARCADE", "BAR", "PAWN", "SUSHI", "GARAGE", "HOTEL", "NOODLES", "KARAOKE", "PHARMACY", "TATTOO", "DINER", "CYBERWARE"];
const NEON_COLORS = ["#ff2a6d", "#05d9e8", "#f9c80e", "#ff6c11", "#b967ff", "#01ff89"];

let rs = 97;
function rnd(): number {
  rs = (rs * 16807) % 2147483647;
  return (rs - 1) / 2147483646;
}

export interface CityViewUpdate {
  /** 0 day .. 1 night */
  night: number;
  wetness: number;
}

/** Static scenery: ground, sidewalks, markings, buildings, lamps, signs. */
export class CityView {
  readonly group = new THREE.Group();
  private buildingMat: THREE.MeshStandardMaterial;
  private roadMat: THREE.MeshStandardMaterial;
  private lampGlowMat: THREE.PointsMaterial;
  private poolMat: THREE.MeshBasicMaterial;
  private neonMats: THREE.MeshBasicMaterial[] = [];
  private beaconMat: THREE.PointsMaterial;
  private beaconClock = 0;
  private colWidth: Map<number, number> = new Map();
  private rowWidth: Map<number, number> = new Map();

  constructor(readonly world: World) {
    const city = world.city;
    const net = city.network;
    this.measureGrid(net);
    // Ground (asphalt everywhere; blocks sit on top).
    const pad = 400;
    const gw = net.maxX - net.minX + pad * 2;
    const gh = net.maxY - net.minY + pad * 2;
    this.roadMat = new THREE.MeshStandardMaterial({ map: asphaltTexture(), roughness: 0.92, metalness: 0.0, color: 0xffffff });
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(gw, gh), this.roadMat);
    ground.rotation.x = -Math.PI / 2;
    ground.position.set((net.minX + net.maxX) / 2, 0, -(net.minY + net.maxY) / 2);
    ground.receiveShadow = false;
    this.group.add(ground);

    this.buildBlocks(city);
    this.buildMedians(net);
    this.buildMarkings(net);
    this.buildArrows(net);
    const fac = facadeTextures();
    this.buildingMat = new THREE.MeshStandardMaterial({ map: fac.map, emissiveMap: fac.emissive, emissive: 0xffffff, emissiveIntensity: 1, vertexColors: true, roughness: 0.75, metalness: 0.2 });
    this.buildBuildings(city);
    this.buildSkyline(net, fac.map, fac.emissive);
    this.buildWalls();
    const glow = glowTexture();
    this.lampGlowMat = new THREE.PointsMaterial({ size: 3.2, map: glow, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, vertexColors: true, sizeAttenuation: true, toneMapped: false });
    this.poolMat = new THREE.MeshBasicMaterial({ map: glow, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, color: 0xffb35c, toneMapped: false, opacity: 0.0 });
    this.beaconMat = new THREE.PointsMaterial({ size: 4, map: glow, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, color: 0xff2020, sizeAttenuation: true, toneMapped: false });
    this.buildLamps(net);
    this.buildSigns(net);
    void beamTexture;
  }

  private measureGrid(net: Network): void {
    for (const r of net.roads) {
      if (Math.abs(r.a.x - r.b.x) < 0.5) this.colWidth.set(r.a.x, Math.max(this.colWidth.get(r.a.x) ?? 0, r.width));
      if (Math.abs(r.a.y - r.b.y) < 0.5) this.rowWidth.set(r.a.y, Math.max(this.rowWidth.get(r.a.y) ?? 0, r.width));
    }
  }

  private buildBlocks(city: City): void {
    const xs = city.xs;
    const ys = city.ys;
    const side = new GeomBuilder();
    const parks = new GeomBuilder();
    const curb = 0.16;
    const ext = 30; // verge outside the ring road
    const xEdges: number[][] = [];
    for (let i = -1; i < xs.length; i++) {
      const xa = i < 0 ? xs[0] - ext - (this.colWidth.get(xs[0]) ?? 0) : xs[i] + (this.colWidth.get(xs[i]) ?? 0) / 2;
      const xb = i >= xs.length - 1 ? xs[xs.length - 1] + ext + (this.colWidth.get(xs[xs.length - 1]) ?? 0) : xs[i + 1] - (this.colWidth.get(xs[i + 1]) ?? 0) / 2;
      xEdges.push([xa, xb]);
    }
    const yEdges: number[][] = [];
    for (let j = -1; j < ys.length; j++) {
      const ya = j < 0 ? ys[0] - ext - (this.rowWidth.get(ys[0]) ?? 0) : ys[j] + (this.rowWidth.get(ys[j]) ?? 0) / 2;
      const yb = j >= ys.length - 1 ? ys[ys.length - 1] + ext + (this.rowWidth.get(ys[ys.length - 1]) ?? 0) : ys[j + 1] - (this.rowWidth.get(ys[j + 1]) ?? 0) / 2;
      yEdges.push([ya, yb]);
    }
    for (const [x0, x1] of xEdges) {
      for (const [y0, y1] of yEdges) {
        if (x1 - x0 < 1 || y1 - y0 < 1) continue;
        const cx = (x0 + x1) / 2;
        const cy = (y0 + y1) / 2;
        // Curb (slightly lighter, thin ring) + sidewalk slab.
        side.box(cx, 0, -cy, x1 - x0, curb, y1 - y0, CURB);
        side.groundRect(cx, -cy, x1 - x0 - 0.5, y1 - y0 - 0.5, 0, curb + 0.005, SIDEWALK, [0, 0, (x1 - x0) / 4, 0, (x1 - x0) / 4, (y1 - y0) / 4, 0, (y1 - y0) / 4]);
      }
    }
    for (const b of city.buildings) {
      if (b.kind !== "park" && b.kind !== "plaza") continue;
      const col = b.kind === "park" ? new THREE.Color(0x1f3a26) : new THREE.Color(0x5a5560);
      parks.groundRect(b.x, -b.y, b.w, b.h, 0, curb + 0.02, col);
    }
    const sideMat = new THREE.MeshStandardMaterial({ map: concreteTexture(), vertexColors: true, roughness: 0.95 });
    this.group.add(new THREE.Mesh(side.build(), sideMat));
    if (!parks.empty) this.group.add(new THREE.Mesh(parks.build(), new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 1 })));
    // Trees in parks.
    const trees: THREE.Vector3[] = [];
    for (const b of city.buildings) {
      if (b.kind !== "park") continue;
      const n = Math.floor((b.w * b.h) / 140);
      for (let i = 0; i < n; i++) trees.push(new THREE.Vector3(b.x + (rnd() - 0.5) * (b.w - 6), 0, -(b.y + (rnd() - 0.5) * (b.h - 6))));
    }
    this.addTrees(trees);
  }

  private addTrees(at: THREE.Vector3[]): void {
    if (at.length === 0) return;
    const trunk = new THREE.InstancedMesh(new THREE.CylinderGeometry(0.15, 0.2, 2.4, 5), new THREE.MeshStandardMaterial({ color: 0x3b2a20 }), at.length);
    const crown = new THREE.InstancedMesh(new THREE.IcosahedronGeometry(1.7, 0), new THREE.MeshStandardMaterial({ color: 0x1e4d2b, flatShading: true }), at.length);
    const m = new THREE.Matrix4();
    at.forEach((p, i) => {
      const s = 0.8 + rnd() * 0.6;
      m.makeScale(1, 1, 1).setPosition(p.x, 1.2, p.z);
      trunk.setMatrixAt(i, m);
      m.makeScale(s, s * 1.2, s).setPosition(p.x, 2.9 * s + 0.6, p.z);
      crown.setMatrixAt(i, m);
    });
    this.group.add(trunk, crown);
  }

  private buildMedians(net: Network): void {
    const g = new GeomBuilder();
    const trees: THREE.Vector3[] = [];
    for (const r of net.roads) {
      if (r.medianWidth < 1.4 || r.lanesBA.length === 0 || r.lanesAB.length === 0) continue;
      const len = r.centerPath.length;
      const t0 = r.trimA + 1.5;
      const t1 = len - r.trimB - 1.5;
      if (t1 - t0 < 4) continue;
      const pts = r.centerPath.slice(t0, t1).map((p) => toV3(p.x, p.y, 0));
      for (let i = 0; i < pts.length - 1; i++) {
        const a = pts[i];
        const b = pts[i + 1];
        const cx = (a.x + b.x) / 2;
        const cz = (a.z + b.z) / 2;
        const l = Math.hypot(b.x - a.x, b.z - a.z);
        const along = Math.abs(b.x - a.x) > Math.abs(b.z - a.z);
        g.box(cx, 0, cz, along ? l : r.medianWidth - 0.3, 0.2, along ? r.medianWidth - 0.3 : l, MEDIAN);
        if (r.kind === "avenue") {
          for (let s = 6; s < l - 4; s += 14) trees.push(new THREE.Vector3(a.x + ((b.x - a.x) * s) / l, 0, a.z + ((b.z - a.z) * s) / l));
        }
      }
    }
    if (!g.empty) this.group.add(new THREE.Mesh(g.build(), new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.9 })));
    this.addTrees(trees);
  }

  private lanePts(l: Lane, offset: number, s0: number, s1: number): THREE.Vector3[] {
    const out: THREE.Vector3[] = [];
    const tmp = { x: 0, y: 0, heading: 0 };
    const n = Math.max(1, Math.ceil((s1 - s0) / 8));
    for (let i = 0; i <= n; i++) {
      const s = s0 + ((s1 - s0) * i) / n;
      l.path.sample(s, tmp);
      const nx = -Math.sin(tmp.heading);
      const ny = Math.cos(tmp.heading);
      out.push(toV3(tmp.x + nx * offset, tmp.y + ny * offset, 0));
    }
    return out;
  }

  private dashed(g: GeomBuilder, l: Lane, offset: number, color: THREE.Color, dash: number, gap: number): void {
    for (let s = 2; s < l.length - 2; s += dash + gap) {
      g.strip(this.lanePts(l, offset, s, Math.min(s + dash, l.length - 2)), 0.13, color, 0.025);
    }
  }

  private buildMarkings(net: Network): void {
    const g = new GeomBuilder();
    for (const r of net.roads) {
      for (const group of [r.lanesAB, r.lanesBA]) {
        if (group.length === 0) continue;
        const twoWay = r.lanesAB.length > 0 && r.lanesBA.length > 0;
        for (let i = 0; i < group.length; i++) {
          const l = group[i];
          const half = l.width / 2;
          // Between same-direction lanes: dashed white (solid near the stop line).
          if (i < group.length - 1) {
            this.dashed(g, l, -half, WHITE, 3, 6);
            if (l.length > 30) g.strip(this.lanePts(l, -half, l.length - 22, l.length - 0.5), 0.15, WHITE, 0.026);
          } else {
            g.strip(this.lanePts(l, -half + 0.15, 0.5, l.length - 0.5), 0.15, WHITE, 0.025);
          }
          if (i === 0) {
            if (twoWay && r.medianWidth < 1.4) {
              // Double yellow centre line (only draw once, from the A->B side).
              if (group === r.lanesAB) {
                g.strip(this.lanePts(l, half + r.medianWidth / 2 - 0.1, 0.5, l.length - 0.5), 0.12, YELLOW, 0.025);
                g.strip(this.lanePts(l, half + r.medianWidth / 2 + 0.1, 0.5, l.length - 0.5), 0.12, YELLOW, 0.025);
              }
            } else {
              g.strip(this.lanePts(l, half - 0.15, 0.5, l.length - 0.5), 0.13, YELLOW, 0.025);
            }
          }
        }
      }
    }
    // Stop lines and crosswalks.
    for (const n of net.nodes) {
      if (n.roads.length < 3) continue;
      for (const ap of n.approaches) {
        const a = ap.stopA;
        const b = ap.stopB;
        const back = 0.4;
        const ox = -Math.cos(ap.heading) * back;
        const oy = -Math.sin(ap.heading) * back;
        if (ap.sign === "yield") {
          // Shark teeth.
          const steps = Math.floor(Math.hypot(b.x - a.x, b.y - a.y) / 0.9);
          for (let k = 0; k < steps; k++) {
            const t = (k + 0.5) / steps;
            const px = a.x + (b.x - a.x) * t + ox;
            const py = a.y + (b.y - a.y) * t + oy;
            g.groundRect(px, -py, 0.5, 0.5, ap.heading, 0.026, WHITE);
          }
        } else if (ap.sign !== "none" || n.control === "signal") {
          g.strip([toV3(a.x + ox, a.y + oy, 0), toV3(b.x + ox, b.y + oy, 0)], 0.45, WHITE, 0.027);
        }
      }
      if (n.control === "none") continue;
      for (const r of n.roads) {
        const h = r.headingFrom(n);
        const dirx = Math.cos(h);
        const diry = Math.sin(h);
        const px = -diry;
        const py = dirx;
        const mid = n.radius - 2.6;
        const w = r.width + 1;
        for (let off = -w / 2 + 0.4; off <= w / 2 - 0.4; off += 1.15) {
          const cx = n.x + dirx * mid + px * off;
          const cy = n.y + diry * mid + py * off;
          g.groundRect(cx, -cy, 0.6, 2.8, h + Math.PI / 2, 0.026, WHITE);
        }
      }
    }
    const mat = new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.6, emissive: 0x222222, polygonOffset: true, polygonOffsetFactor: -2 });
    this.group.add(new THREE.Mesh(g.build(), mat));
  }

  private buildArrows(net: Network): void {
    const groups = new Map<string, { pos: THREE.Vector3; rot: number }[]>();
    const tmp = { x: 0, y: 0, heading: 0 };
    for (const l of net.lanes) {
      if (l.to.roads.length < 3 || l.length < 20) continue;
      const key = ["left", "straight", "right"].filter((t) => l.turns.includes(t as never)).join("+");
      if (key === "" || key === "straight") continue;
      l.path.sample(l.length - 9, tmp);
      let list = groups.get(key);
      if (list === undefined) {
        list = [];
        groups.set(key, list);
      }
      list.push({ pos: toV3(tmp.x, tmp.y, 0.03), rot: tmp.heading });
    }
    const m = new THREE.Matrix4();
    const q = new THREE.Quaternion();
    groups.forEach((list, key) => {
      const geo = new THREE.PlaneGeometry(1.5, 3.0);
      geo.rotateX(-Math.PI / 2);
      geo.rotateY(-Math.PI / 2); // texture "up" points along +x
      const mat = new THREE.MeshStandardMaterial({ map: laneArrowTexture(key.split("+")), transparent: true, depthWrite: false, emissive: 0x222222, polygonOffset: true, polygonOffsetFactor: -3 });
      const im = new THREE.InstancedMesh(geo, mat, list.length);
      list.forEach((a, i) => {
        q.setFromAxisAngle(new THREE.Vector3(0, 1, 0), a.rot);
        m.compose(a.pos, q, new THREE.Vector3(1, 1, 1));
        im.setMatrixAt(i, m);
      });
      this.group.add(im);
    });
  }

  private buildBuildings(city: City): void {
    const g = new GeomBuilder();
    const neon = new Map<number, { pos: THREE.Vector3; rot: number; w: number }[]>();
    const beacons: number[] = [];
    for (const b of city.buildings) {
      if (b.kind === "park" || b.kind === "plaza") continue;
      const tint = new THREE.Color().setHSL(0.6 + b.hue * 0.15, 0.08, 0.55 + b.hue * 0.25);
      const roof = new THREE.Color(0x16171d);
      g.box(b.x, 0.16, -b.y, b.w, b.height, b.h, tint, 16, 28, roof);
      if (b.kind === "tower") {
        // Setback crown.
        g.box(b.x, 0.16 + b.height, -b.y, b.w * 0.6, b.height * 0.08 + 3, b.h * 0.6, tint, 16, 28, roof);
        beacons.push(b.x, b.height * 1.08 + 4, -b.y);
      }
      // Neon shop signs low on a street-facing side.
      if (rnd() < 0.75) {
        const k = Math.floor(rnd() * NEON_WORDS.length * NEON_COLORS.length);
        const faceX = rnd() < 0.5;
        const sgn = rnd() < 0.5 ? 1 : -1;
        const hgt = 4 + rnd() * Math.min(10, b.height - 6);
        const pos = faceX ? new THREE.Vector3(b.x + sgn * (b.w / 2 + 0.06), hgt, -b.y) : new THREE.Vector3(b.x, hgt, -b.y + sgn * (b.h / 2 + 0.06));
        const rot = faceX ? (sgn > 0 ? Math.PI / 2 : -Math.PI / 2) : sgn > 0 ? 0 : Math.PI;
        let list = neon.get(k);
        if (list === undefined) {
          list = [];
          neon.set(k, list);
        }
        list.push({ pos, rot, w: Math.min(9, (faceX ? b.h : b.w) * 0.6) });
      }
    }
    const mesh = new THREE.Mesh(g.build(), this.buildingMat);
    this.group.add(mesh);
    const m = new THREE.Matrix4();
    const q = new THREE.Quaternion();
    neon.forEach((list, k) => {
      const word = NEON_WORDS[k % NEON_WORDS.length];
      const color = NEON_COLORS[Math.floor(k / NEON_WORDS.length) % NEON_COLORS.length];
      const mat = new THREE.MeshBasicMaterial({ map: neonTexture(word, color), toneMapped: false, transparent: true });
      this.neonMats.push(mat);
      const im = new THREE.InstancedMesh(new THREE.PlaneGeometry(1, 0.25), mat, list.length);
      list.forEach((s, i) => {
        q.setFromAxisAngle(new THREE.Vector3(0, 1, 0), s.rot);
        m.compose(s.pos, q, new THREE.Vector3(s.w, s.w, 1));
        im.setMatrixAt(i, m);
      });
      this.group.add(im);
    });
    const bg = new THREE.BufferGeometry();
    bg.setAttribute("position", new THREE.Float32BufferAttribute(beacons, 3));
    this.beaconPoints = new THREE.Points(bg, undefined);
  }

  private beaconPoints: THREE.Points | undefined;

  private buildSkyline(net: Network, map: THREE.Texture, emissive: THREE.Texture): void {
    const g = new GeomBuilder();
    const cx = (net.minX + net.maxX) / 2;
    const cy = (net.minY + net.maxY) / 2;
    const rx = (net.maxX - net.minX) / 2 + 90;
    const ry = (net.maxY - net.minY) / 2 + 90;
    for (let i = 0; i < 90; i++) {
      const a = (i / 90) * Math.PI * 2;
      const dist = 1 + rnd() * 0.9;
      const x = cx + Math.cos(a) * rx * dist;
      const y = cy + Math.sin(a) * ry * dist;
      const w = 25 + rnd() * 45;
      const h = 30 + rnd() * 170;
      const tint = new THREE.Color().setHSL(0.62, 0.1, 0.35 + rnd() * 0.2);
      g.box(x, 0, -y, w, h, w * (0.6 + rnd() * 0.8), tint, 16, 28, new THREE.Color(0x101118));
    }
    const mat = new THREE.MeshStandardMaterial({ map, emissiveMap: emissive, emissive: 0xffffff, emissiveIntensity: 0.8, vertexColors: true, roughness: 0.9 });
    this.skylineMat = mat;
    this.group.add(new THREE.Mesh(g.build(), mat));
  }

  private skylineMat: THREE.MeshStandardMaterial | undefined;

  private buildWalls(): void {
    const g = new GeomBuilder();
    const obs = this.world.obstacles;
    const walls = obs.slice(obs.length - 4);
    for (const w of walls) g.box(w.x, 0, -w.y, w.hl * 2, 1.1, w.hw * 2, new THREE.Color(0x77787f));
    this.group.add(new THREE.Mesh(g.build(), new THREE.MeshStandardMaterial({ vertexColors: true, map: concreteTexture(), roughness: 0.9 })));
  }

  private lampPositions: THREE.Vector3[] = [];

  private buildLamps(net: Network): void {
    const tmp = { x: 0, y: 0, heading: 0 };
    const pos: THREE.Vector3[] = [];
    const rot: number[] = [];
    for (const r of net.roads) {
      for (const group of [r.lanesAB, r.lanesBA]) {
        if (group.length === 0) continue;
        const outer = group[group.length - 1];
        const off = -(outer.width / 2 + 0.9);
        for (let s = 10; s < outer.length - 6; s += 30) {
          outer.path.sample(s, tmp);
          const nx = -Math.sin(tmp.heading);
          const ny = Math.cos(tmp.heading);
          pos.push(toV3(tmp.x + nx * off, tmp.y + ny * off, 0));
          rot.push(tmp.heading);
        }
      }
    }
    this.lampPositions = pos;
    const n = pos.length;
    const poleMat = new THREE.MeshStandardMaterial({ color: 0x2d2f36, roughness: 0.6, metalness: 0.5 });
    const pole = new THREE.InstancedMesh(new THREE.CylinderGeometry(0.09, 0.13, 7.5, 6), poleMat, n);
    const armGeo = new THREE.BoxGeometry(0.12, 0.12, 2.2);
    armGeo.translate(0, 0, -1.0);
    const arm = new THREE.InstancedMesh(armGeo, poleMat, n);
    const headGeo = new THREE.BoxGeometry(0.35, 0.18, 0.7);
    const head = new THREE.InstancedMesh(headGeo, new THREE.MeshBasicMaterial({ color: 0xffd9a0, toneMapped: false }), n);
    const pool = new THREE.InstancedMesh(new THREE.PlaneGeometry(14, 14).rotateX(-Math.PI / 2), this.poolMat, n);
    const m = new THREE.Matrix4();
    const q = new THREE.Quaternion();
    const glowPos: number[] = [];
    const glowCol: number[] = [];
    for (let i = 0; i < n; i++) {
      const p = pos[i];
      // Arm points toward the road (left of travel = toward lanes).
      const h = rot[i];
      q.setFromAxisAngle(new THREE.Vector3(0, 1, 0), h);
      m.compose(new THREE.Vector3(p.x, 3.75, p.z), q, new THREE.Vector3(1, 1, 1));
      pole.setMatrixAt(i, m);
      const ax = -Math.sin(h);
      const ay = Math.cos(h);
      const toward = toV3(0, 0, 0);
      toward.set(ax, 0, -ay);
      const armQ = new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 0, -1), toward);
      m.compose(new THREE.Vector3(p.x, 7.4, p.z), armQ, new THREE.Vector3(1, 1, 1));
      arm.setMatrixAt(i, m);
      const hx = p.x + toward.x * 2.0;
      const hz = p.z + toward.z * 2.0;
      m.compose(new THREE.Vector3(hx, 7.3, hz), armQ, new THREE.Vector3(1, 1, 1));
      head.setMatrixAt(i, m);
      m.compose(new THREE.Vector3(hx, 0.05, hz), new THREE.Quaternion(), new THREE.Vector3(1, 1, 1));
      pool.setMatrixAt(i, m);
      glowPos.push(hx, 7.15, hz);
      glowCol.push(1, 0.78, 0.45);
    }
    this.group.add(pole, arm, head, pool);
    const gg = new THREE.BufferGeometry();
    gg.setAttribute("position", new THREE.Float32BufferAttribute(glowPos, 3));
    gg.setAttribute("color", new THREE.Float32BufferAttribute(glowCol, 3));
    this.group.add(new THREE.Points(gg, this.lampGlowMat));
    if (this.beaconPoints !== undefined) {
      this.beaconPoints.material = this.beaconMat;
      this.group.add(this.beaconPoints);
    }
  }

  private buildSigns(net: Network): void {
    const tmp = { x: 0, y: 0, heading: 0 };
    type Placement = { pos: THREE.Vector3; rot: number };
    const byTex = new Map<string, { tex: THREE.Texture; w: number; h: number; list: Placement[] }>();
    const posts: Placement[] = [];
    const add = (key: string, make: () => THREE.Texture, w: number, h: number, p: Placement) => {
      let e = byTex.get(key);
      if (e === undefined) {
        e = { tex: make(), w, h, list: [] };
        byTex.set(key, e);
      }
      e.list.push(p);
    };
    const curbPoint = (l: Lane, s: number, extra: number): THREE.Vector3 => {
      l.path.sample(s, tmp);
      const nx = -Math.sin(tmp.heading);
      const ny = Math.cos(tmp.heading);
      const off = -(l.width / 2 + extra);
      return toV3(tmp.x + nx * off, tmp.y + ny * off, 0);
    };
    for (const r of net.roads) {
      for (const group of [r.lanesAB, r.lanesBA]) {
        if (group.length === 0) continue;
        const outer = group[group.length - 1];
        // Speed limit after each junction.
        if (outer.length > 40) {
          const kmh = Math.round(r.speedLimit * 3.6);
          const p = curbPoint(outer, 12, 1.4);
          outer.path.sample(12, tmp);
          add(`speed${kmh}`, () => signTexture("speed", `${kmh}`), 0.75, 0.95, { pos: p, rot: tmp.heading });
          posts.push({ pos: p, rot: tmp.heading });
        }
        // Stop / yield signs at the end of controlled approaches.
        const ap = outer.approach;
        if (ap !== undefined && (ap.sign === "stop" || ap.sign === "yield") && ap.node.roads.length >= 3) {
          const p = curbPoint(outer, outer.length - 1, 1.3);
          outer.path.sample(outer.length - 1, tmp);
          add(ap.sign, () => signTexture(ap.sign), 0.9, 0.9, { pos: p, rot: tmp.heading });
          posts.push({ pos: p, rot: tmp.heading });
        }
        if (r.speedCamera && outer.length > 60) {
          const p = curbPoint(outer, outer.length / 2 - 25, 1.4);
          outer.path.sample(outer.length / 2 - 25, tmp);
          add("speedcam", () => signTexture("camera", "SPEED"), 0.7, 0.9, { pos: p, rot: tmp.heading });
          posts.push({ pos: p, rot: tmp.heading });
        }
      }
    }
    // Street name blades and one-way signs at junction corners.
    for (const n of net.nodes) {
      if (n.roads.length < 3) continue;
      for (const r of n.roads) {
        const h = r.headingFrom(n);
        const corner = this.cornerPos(n, r, h);
        add(`street:${r.name}`, () => signTexture("street", r.name), 2.4, 0.45, { pos: corner, rot: h + Math.PI / 2 });
        if (r.lanesAB.length === 0 || r.lanesBA.length === 0) {
          const dirAway = (r.lanesOutOf(n).length > 0 ? h : h + Math.PI) - Math.PI / 2;
          add("oneway", () => signTexture("oneway"), 1.4, 0.47, { pos: corner.clone().setY(-0.6), rot: dirAway + Math.PI / 2 });
        }
      }
    }
    const m = new THREE.Matrix4();
    const q = new THREE.Quaternion();
    byTex.forEach((e, key) => {
      const geo = new THREE.PlaneGeometry(e.w, e.h);
      // Sign faces oncoming traffic: plane normal +z -> face -heading direction.
      geo.rotateY(-Math.PI / 2);
      const mat = new THREE.MeshStandardMaterial({ map: e.tex, side: THREE.DoubleSide, roughness: 0.5, emissive: 0x333333, emissiveMap: e.tex });
      const im = new THREE.InstancedMesh(geo, mat, e.list.length);
      e.list.forEach((p, i) => {
        q.setFromAxisAngle(new THREE.Vector3(0, 1, 0), p.rot);
        const hgt = key.startsWith("street") ? 3.3 : key === "oneway" ? 2.7 : 2.3;
        m.compose(new THREE.Vector3(p.pos.x, hgt + (p.pos.y < 0 ? 0 : 0), p.pos.z), q, new THREE.Vector3(1, 1, 1));
        im.setMatrixAt(i, m);
      });
      this.group.add(im);
    });
    const postMesh = new THREE.InstancedMesh(new THREE.CylinderGeometry(0.04, 0.04, 2.4, 5), new THREE.MeshStandardMaterial({ color: 0x9a9ca3, metalness: 0.6, roughness: 0.4 }), Math.max(1, posts.length));
    posts.forEach((p, i) => {
      m.makeTranslation(p.pos.x, 1.2, p.pos.z);
      postMesh.setMatrixAt(i, m);
    });
    this.group.add(postMesh);
  }

  /** Right-hand corner of a junction for a road leaving node n with heading h. */
  cornerPos(n: Node, r: Road, h: number): THREE.Vector3 {
    const d = n.radius + 1.0;
    const side = r.width / 2 + 1.6;
    const x = n.x + Math.cos(h) * d + Math.sin(h) * side;
    const y = n.y + Math.sin(h) * d - Math.cos(h) * side;
    return toV3(x, y, 0);
  }

  /** Reflections for wet asphalt. */
  setEnvMap(t: THREE.Texture): void {
    this.roadMat.envMap = t;
    this.roadMat.envMapIntensity = 0;
    this.roadMat.needsUpdate = true;
  }

  update(dt: number, s: CityViewUpdate): void {
    this.buildingMat.emissiveIntensity = 0.1 + s.night * 0.95;
    if (this.skylineMat !== undefined) this.skylineMat.emissiveIntensity = 0.1 + s.night * 0.9;
    this.lampGlowMat.opacity = s.night;
    this.lampGlowMat.size = 3.2;
    this.poolMat.opacity = s.night * 0.33;
    for (const m of this.neonMats) m.color.setScalar(0.35 + s.night * 0.85);
    this.beaconClock += dt;
    this.beaconMat.opacity = (Math.sin(this.beaconClock * 3) > 0.2 ? 1 : 0.1) * (0.3 + s.night * 0.7);
    // Wet roads look darker and glossier.
    this.roadMat.roughness = 0.92 - s.wetness * 0.55;
    this.roadMat.color.setScalar(1 - s.wetness * 0.35);
    this.roadMat.metalness = s.wetness * 0.25;
    this.roadMat.envMapIntensity = s.wetness * (0.15 + (1 - s.night) * 0.35);
  }

  get lamps(): THREE.Vector3[] {
    return this.lampPositions;
  }
}
