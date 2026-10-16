import * as THREE from "three";
import { World } from "../core/world/world";
import { glowTexture, smokeTexture } from "./textures";

/** Rain, smoke, sparks and skid marks. */
export class Effects {
  readonly group = new THREE.Group();
  private rain: THREE.LineSegments;
  private rainPos: Float32Array;
  private rainCount = 2600;
  private smoke: THREE.Points;
  private smokePos: Float32Array;
  private smokeLife: Float32Array;
  private smokeVel: Float32Array;
  private smokeN = 400;
  private smokeNext = 0;
  private sparks: THREE.Points;
  private sparkPos: Float32Array;
  private sparkVel: Float32Array;
  private sparkLife: Float32Array;
  private sparkN = 300;
  private sparkNext = 0;
  private skids: THREE.InstancedMesh;
  private skidN = 1500;
  private skidNext = 0;
  private lastSkid: Map<string, THREE.Vector3> = new Map();
  private m = new THREE.Matrix4();

  constructor() {
    this.rainPos = new Float32Array(this.rainCount * 6);
    const rg = new THREE.BufferGeometry();
    rg.setAttribute("position", new THREE.BufferAttribute(this.rainPos, 3));
    this.rain = new THREE.LineSegments(rg, new THREE.LineBasicMaterial({ color: 0x9fb4d6, transparent: true, opacity: 0.35 }));
    this.rain.frustumCulled = false;
    this.rain.visible = false;
    for (let i = 0; i < this.rainCount; i++) this.resetDrop(i, new THREE.Vector3(), true);

    this.smokePos = new Float32Array(this.smokeN * 3);
    this.smokeLife = new Float32Array(this.smokeN);
    this.smokeVel = new Float32Array(this.smokeN * 3);
    const sg = new THREE.BufferGeometry();
    sg.setAttribute("position", new THREE.BufferAttribute(this.smokePos, 3));
    this.smoke = new THREE.Points(sg, new THREE.PointsMaterial({ size: 2.4, map: smokeTexture(), color: 0x666666, transparent: true, opacity: 0.45, depthWrite: false }));
    this.smoke.frustumCulled = false;

    this.sparkPos = new Float32Array(this.sparkN * 3);
    this.sparkVel = new Float32Array(this.sparkN * 3);
    this.sparkLife = new Float32Array(this.sparkN);
    const spg = new THREE.BufferGeometry();
    spg.setAttribute("position", new THREE.BufferAttribute(this.sparkPos, 3));
    this.sparks = new THREE.Points(spg, new THREE.PointsMaterial({ size: 0.35, map: glowTexture(), color: new THREE.Color(4, 2.4, 0.8), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, toneMapped: false }));
    this.sparks.frustumCulled = false;
    for (let i = 0; i < this.sparkN; i++) this.sparkPos[i * 3 + 1] = -100;
    for (let i = 0; i < this.smokeN; i++) this.smokePos[i * 3 + 1] = -100;

    const skidGeo = new THREE.PlaneGeometry(1, 1);
    skidGeo.rotateX(-Math.PI / 2);
    this.skids = new THREE.InstancedMesh(skidGeo, new THREE.MeshBasicMaterial({ color: 0x050505, transparent: true, opacity: 0.55, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -4 }), this.skidN);
    this.skids.count = 0;
    this.skids.frustumCulled = false;
    this.group.add(this.rain, this.smoke, this.sparks, this.skids);
  }

  private resetDrop(i: number, around: THREE.Vector3, randomY: boolean): void {
    const x = around.x + (Math.random() - 0.5) * 80;
    const z = around.z + (Math.random() - 0.5) * 80;
    const y = randomY ? Math.random() * 30 : 30 + Math.random() * 5;
    const o = i * 6;
    this.rainPos[o] = x;
    this.rainPos[o + 1] = y;
    this.rainPos[o + 2] = z;
    this.rainPos[o + 3] = x + 0.05;
    this.rainPos[o + 4] = y - 0.7;
    this.rainPos[o + 5] = z;
  }

  sparksAt(x: number, y: number, z: number, count: number): void {
    for (let k = 0; k < count; k++) {
      const i = this.sparkNext;
      this.sparkNext = (this.sparkNext + 1) % this.sparkN;
      this.sparkPos[i * 3] = x;
      this.sparkPos[i * 3 + 1] = y;
      this.sparkPos[i * 3 + 2] = z;
      this.sparkVel[i * 3] = (Math.random() - 0.5) * 9;
      this.sparkVel[i * 3 + 1] = Math.random() * 5 + 1;
      this.sparkVel[i * 3 + 2] = (Math.random() - 0.5) * 9;
      this.sparkLife[i] = 0.4 + Math.random() * 0.5;
    }
  }

  private puff(x: number, y: number, z: number): void {
    const i = this.smokeNext;
    this.smokeNext = (this.smokeNext + 1) % this.smokeN;
    this.smokePos[i * 3] = x + (Math.random() - 0.5) * 0.6;
    this.smokePos[i * 3 + 1] = y;
    this.smokePos[i * 3 + 2] = z + (Math.random() - 0.5) * 0.6;
    this.smokeVel[i * 3] = (Math.random() - 0.5) * 0.6;
    this.smokeVel[i * 3 + 1] = 1.2 + Math.random();
    this.smokeVel[i * 3 + 2] = (Math.random() - 0.5) * 0.6;
    this.smokeLife[i] = 2.5;
  }

  update(dt: number, world: World, cam: THREE.Vector3, rain: number): void {
    // Rain follows the camera.
    this.rain.visible = rain > 0.03;
    if (this.rain.visible) {
      const active = Math.floor(this.rainCount * Math.min(1, rain * 1.2));
      const fall = 26 * dt;
      for (let i = 0; i < this.rainCount; i++) {
        const o = i * 6;
        if (i >= active) {
          this.rainPos[o + 1] = -50;
          this.rainPos[o + 4] = -50;
          continue;
        }
        this.rainPos[o + 1] -= fall;
        this.rainPos[o + 4] -= fall;
        if (this.rainPos[o + 4] < 0 || Math.abs(this.rainPos[o] - cam.x) > 45 || Math.abs(this.rainPos[o + 2] - cam.z) > 45) this.resetDrop(i, cam, this.rainPos[o + 4] < -10);
      }
      (this.rain.material as THREE.LineBasicMaterial).opacity = 0.2 + rain * 0.3;
      this.rain.geometry.getAttribute("position").needsUpdate = true;
    }
    // Smoke from damaged engines; skid marks from sliding tyres.
    for (const v of world.vehicles) {
      const dx = v.x - cam.x;
      const dz = -v.y - cam.z;
      if (dx * dx + dz * dz > 150 * 150) continue;
      const hp = v.pt.health;
      if ((hp < 0.45 || v.wrecked) && Math.random() < dt * (v.wrecked ? 8 : (0.45 - hp) * 20)) {
        const fx = v.x + Math.cos(v.heading) * v.halfLen * 0.7;
        const fy = v.y + Math.sin(v.heading) * v.halfLen * 0.7;
        this.puff(fx, v.spec.height * 0.8, -fy);
      }
      if (v.mode === "physics" && v.dyn.skid > 0.45 && v.absSpeed() > 2) {
        for (const side of [-1, 1]) {
          const key = `${v.id}:${side}`;
          const rx = v.x - Math.cos(v.heading) * v.spec.wheelbase * 0.5 + Math.sin(v.heading) * side * (v.halfWid - 0.2);
          const ry = v.y - Math.sin(v.heading) * v.spec.wheelbase * 0.5 - Math.cos(v.heading) * side * (v.halfWid - 0.2);
          const now = new THREE.Vector3(rx, 0.035, -ry);
          const last = this.lastSkid.get(key);
          if (last !== undefined && last.distanceTo(now) > 0.4 && last.distanceTo(now) < 3) {
            const mid = last.clone().add(now).multiplyScalar(0.5);
            const len = last.distanceTo(now);
            const ang = Math.atan2(-(now.z - last.z), now.x - last.x);
            this.m.compose(mid, new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), ang), new THREE.Vector3(len + 0.05, 1, 0.24));
            this.skids.setMatrixAt(this.skidNext, this.m);
            this.skidNext = (this.skidNext + 1) % this.skidN;
            this.skids.count = Math.min(this.skidN, Math.max(this.skids.count, this.skidNext));
            this.skids.instanceMatrix.needsUpdate = true;
            this.lastSkid.set(key, now);
          } else if (last === undefined || last.distanceTo(now) >= 3) this.lastSkid.set(key, now);
        }
      } else {
        this.lastSkid.delete(`${v.id}:-1`);
        this.lastSkid.delete(`${v.id}:1`);
      }
    }
    for (let i = 0; i < this.smokeN; i++) {
      if (this.smokeLife[i] <= 0) continue;
      this.smokeLife[i] -= dt;
      this.smokePos[i * 3] += this.smokeVel[i * 3] * dt;
      this.smokePos[i * 3 + 1] += this.smokeVel[i * 3 + 1] * dt;
      this.smokePos[i * 3 + 2] += this.smokeVel[i * 3 + 2] * dt;
      if (this.smokeLife[i] <= 0) this.smokePos[i * 3 + 1] = -100;
    }
    this.smoke.geometry.getAttribute("position").needsUpdate = true;
    for (let i = 0; i < this.sparkN; i++) {
      if (this.sparkLife[i] <= 0) continue;
      this.sparkLife[i] -= dt;
      this.sparkVel[i * 3 + 1] -= 9.8 * dt;
      this.sparkPos[i * 3] += this.sparkVel[i * 3] * dt;
      this.sparkPos[i * 3 + 1] = Math.max(0.02, this.sparkPos[i * 3 + 1] + this.sparkVel[i * 3 + 1] * dt);
      this.sparkPos[i * 3 + 2] += this.sparkVel[i * 3 + 2] * dt;
      if (this.sparkLife[i] <= 0) this.sparkPos[i * 3 + 1] = -100;
    }
    this.sparks.geometry.getAttribute("position").needsUpdate = true;
  }
}

/** Raindrops on the windshield (2D overlay) that the wipers clear. */
export class Windshield {
  private ctx: CanvasRenderingContext2D;
  private drops: { x: number; y: number; r: number; age: number }[] = [];
  private lastWiper = 0;

  constructor(readonly canvas: HTMLCanvasElement) {
    this.ctx = canvas.getContext("2d") as CanvasRenderingContext2D;
  }

  update(dt: number, rain: number, speed: number, wiperPos: number, visible: boolean): void {
    const c = this.canvas;
    const w = c.clientWidth;
    const h = c.clientHeight;
    if (c.width !== w || c.height !== h) {
      c.width = w;
      c.height = h;
    }
    const g = this.ctx;
    g.clearRect(0, 0, w, h);
    if (!visible) return;
    // New drops; fewer stick at speed (airflow).
    const rate = rain * 160 * (1 - Math.min(speed / 40, 0.7));
    let add = rate * dt;
    while (add > 0) {
      if (Math.random() < add) this.drops.push({ x: Math.random() * w, y: Math.random() * h, r: 2 + Math.random() * 5, age: 0 });
      add -= 1;
    }
    if (this.drops.length > 700) this.drops.splice(0, this.drops.length - 700);
    // Wiper arms sweep from the bottom corners; clear drops under the blade.
    const sweeping = wiperPos > 0.001;
    if (sweeping) {
      const ang = Math.PI - wiperPos * Math.PI * 0.95;
      const prev = Math.PI - this.lastWiper * Math.PI * 0.95;
      for (const pivot of [{ x: w * 0.28 }, { x: w * 0.72 }]) {
        const px = pivot.x;
        const py = h * 1.05;
        const len = h * 0.95;
        this.drops = this.drops.filter((d) => {
          const a = Math.atan2(py - d.y, d.x - px);
          const dist = Math.hypot(d.x - px, d.y - py);
          const lo = Math.min(ang, prev) - 0.04;
          const hi = Math.max(ang, prev) + 0.04;
          return !(dist < len && a >= lo && a <= hi);
        });
        g.strokeStyle = "rgba(10,10,12,0.85)";
        g.lineWidth = 5;
        g.beginPath();
        g.moveTo(px, py);
        g.lineTo(px + Math.cos(ang) * len, py - Math.sin(ang) * len);
        g.stroke();
      }
    }
    this.lastWiper = wiperPos;
    for (const d of this.drops) {
      d.age += dt;
      if (speed > 8) d.y -= speed * dt * 2;
      g.fillStyle = "rgba(190,210,235,0.22)";
      g.beginPath();
      g.arc(d.x, d.y, d.r, 0, Math.PI * 2);
      g.fill();
      g.fillStyle = "rgba(255,255,255,0.35)";
      g.beginPath();
      g.arc(d.x - d.r * 0.3, d.y - d.r * 0.3, d.r * 0.35, 0, Math.PI * 2);
      g.fill();
    }
    this.drops = this.drops.filter((d) => d.y > -10);
  }
}
