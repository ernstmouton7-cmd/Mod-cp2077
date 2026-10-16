import * as THREE from "three";
import { World } from "../core/world/world";

const CAP = 160;
const SKIN = [0xf1c27d, 0xe0ac69, 0xc68642, 0x8d5524, 0xffdbac, 0x5c3a21];

/** Instanced pedestrians: torso+legs (shirt-tinted) and heads. */
export class PedView {
  readonly group = new THREE.Group();
  private body: THREE.InstancedMesh;
  private head: THREE.InstancedMesh;
  private m = new THREE.Matrix4();
  private q = new THREE.Quaternion();
  private e = new THREE.Euler();
  private col = new THREE.Color();

  constructor() {
    // Body: legs (dark) + torso (white, tinted per instance), 1 m tall, origin at feet.
    const legs = new THREE.BoxGeometry(0.34, 0.5, 0.22).translate(0, 0.25, 0);
    const torso = new THREE.BoxGeometry(0.42, 0.42, 0.26).translate(0, 0.71, 0);
    const paint = (g: THREE.BufferGeometry, c: number) => {
      const n = g.getAttribute("position").count;
      const arr: number[] = [];
      for (let i = 0; i < n; i++) arr.push(c, c, c);
      g.setAttribute("color", new THREE.Float32BufferAttribute(arr, 3));
      return g;
    };
    paint(legs, 0.12);
    paint(torso, 1);
    const merged = mergeTwo(legs, torso);
    this.body = new THREE.InstancedMesh(merged, new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.85 }), CAP);
    this.body.instanceColor = new THREE.InstancedBufferAttribute(new Float32Array(CAP * 3), 3);
    this.head = new THREE.InstancedMesh(new THREE.SphereGeometry(0.12, 10, 8), new THREE.MeshStandardMaterial({ roughness: 0.7 }), CAP);
    this.head.instanceColor = new THREE.InstancedBufferAttribute(new Float32Array(CAP * 3), 3);
    for (const im of [this.body, this.head]) {
      im.count = 0;
      im.frustumCulled = false;
    }
    this.group.add(this.body, this.head);
  }

  update(world: World): void {
    let n = 0;
    for (const p of world.peds.peds) {
      if (n >= CAP) break;
      const h = p.height;
      const scale = h / 1.0;
      const bob = p.state === "crossing" || p.state === "leaving" ? Math.abs(Math.sin(p.stride * Math.PI)) * 0.04 : 0;
      const down = p.state === "down";
      // Model faces +x; heading is the walking direction.
      this.e.set(down ? Math.PI / 2 : 0, p.heading, 0, "YXZ");
      this.q.setFromEuler(this.e);
      const y = down ? 0.12 : 0.17 + bob;
      this.m.compose(new THREE.Vector3(p.x, y, -p.y), this.q, new THREE.Vector3(0.9, scale * 0.9, 0.9));
      this.body.setMatrixAt(n, this.m);
      this.col.setHex(p.shirt).convertSRGBToLinear();
      this.body.setColorAt(n, this.col);
      const headPos = down ? new THREE.Vector3(p.x + Math.cos(p.heading + Math.PI / 2) * 0.8, 0.14, -(p.y + Math.sin(p.heading + Math.PI / 2) * 0.8)) : new THREE.Vector3(p.x, 0.17 + bob + scale * 0.9 * 0.92 + 0.08, -p.y);
      this.m.compose(headPos, this.q, new THREE.Vector3(1, 1, 1));
      this.head.setMatrixAt(n, this.m);
      this.col.setHex(SKIN[p.id % SKIN.length]).convertSRGBToLinear();
      this.head.setColorAt(n, this.col);
      n++;
    }
    for (const im of [this.body, this.head]) {
      im.count = n;
      im.instanceMatrix.needsUpdate = true;
      if (im.instanceColor !== null) im.instanceColor.needsUpdate = true;
    }
  }
}

function mergeTwo(a: THREE.BufferGeometry, b: THREE.BufferGeometry): THREE.BufferGeometry {
  const ai = a.toNonIndexed();
  const bi = b.toNonIndexed();
  const g = new THREE.BufferGeometry();
  for (const name of ["position", "normal", "color"]) {
    const x = ai.getAttribute(name).array as Float32Array;
    const y = bi.getAttribute(name).array as Float32Array;
    const out = new Float32Array(x.length + y.length);
    out.set(x, 0);
    out.set(y, x.length);
    g.setAttribute(name, new THREE.BufferAttribute(out, 3));
  }
  return g;
}
