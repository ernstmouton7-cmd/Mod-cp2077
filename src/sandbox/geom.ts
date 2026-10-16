import * as THREE from "three";

/** Accumulates triangles (with normals, uvs, colors) into one BufferGeometry. */
export class GeomBuilder {
  readonly pos: number[] = [];
  readonly nrm: number[] = [];
  readonly uv: number[] = [];
  readonly col: number[] = [];

  /** Quad from 4 corners (counter-clockwise when seen from the normal side). */
  quad(a: THREE.Vector3, b: THREE.Vector3, c: THREE.Vector3, d: THREE.Vector3, n: THREE.Vector3, color: THREE.Color, uvs?: number[]): void {
    const u = uvs ?? [0, 0, 1, 0, 1, 1, 0, 1];
    const pts = [a, b, c, a, c, d];
    const us = [0, 1, 2, 0, 2, 3];
    for (let i = 0; i < 6; i++) {
      const p = pts[i];
      this.pos.push(p.x, p.y, p.z);
      this.nrm.push(n.x, n.y, n.z);
      this.uv.push(u[us[i] * 2], u[us[i] * 2 + 1]);
      this.col.push(color.r, color.g, color.b);
    }
  }

  /** Flat rectangle on the ground plane (three coords), rotated by `rot` about Y. */
  groundRect(cx: number, cz: number, w: number, d: number, rot: number, y: number, color: THREE.Color, uvs?: number[]): void {
    const c = Math.cos(rot);
    const s = Math.sin(rot);
    const corner = (lx: number, lz: number) => new THREE.Vector3(cx + lx * c + lz * s, y, cz - lx * s + lz * c);
    const hw = w / 2;
    const hd = d / 2;
    this.quad(corner(-hw, hd), corner(hw, hd), corner(hw, -hd), corner(-hw, -hd), new THREE.Vector3(0, 1, 0), color, uvs);
  }

  /** Axis-aligned box (three coords) with optional facade UV scaling (metres per texture tile). */
  box(cx: number, y0: number, cz: number, w: number, h: number, d: number, color: THREE.Color, tileW = 0, tileH = 0, roofColor?: THREE.Color): void {
    const x0 = cx - w / 2;
    const x1 = cx + w / 2;
    const z0 = cz - d / 2;
    const z1 = cz + d / 2;
    const y1 = y0 + h;
    const V = (x: number, y: number, z: number) => new THREE.Vector3(x, y, z);
    const side = (a: THREE.Vector3, b: THREE.Vector3, c: THREE.Vector3, dd: THREE.Vector3, n: THREE.Vector3, len: number) => {
      const uvs = tileW > 0 ? [0, 0, len / tileW, 0, len / tileW, h / tileH, 0, h / tileH] : undefined;
      this.quad(a, b, c, dd, n, color, uvs);
    };
    side(V(x0, y0, z1), V(x1, y0, z1), V(x1, y1, z1), V(x0, y1, z1), V(0, 0, 1), w);
    side(V(x1, y0, z0), V(x0, y0, z0), V(x0, y1, z0), V(x1, y1, z0), V(0, 0, -1), w);
    side(V(x1, y0, z1), V(x1, y0, z0), V(x1, y1, z0), V(x1, y1, z1), V(1, 0, 0), d);
    side(V(x0, y0, z0), V(x0, y0, z1), V(x0, y1, z1), V(x0, y1, z0), V(-1, 0, 0), d);
    const roofUv = [0, 0, 0.005, 0, 0.005, 0.005, 0, 0.005];
    this.quad(V(x0, y1, z1), V(x1, y1, z1), V(x1, y1, z0), V(x0, y1, z0), V(0, 1, 0), roofColor ?? color, tileW > 0 ? roofUv : undefined);
  }

  /** A strip following a polyline (three coords already), constant width. */
  strip(points: THREE.Vector3[], width: number, color: THREE.Color, y: number): void {
    for (let i = 0; i < points.length - 1; i++) {
      const a = points[i];
      const b = points[i + 1];
      const dx = b.x - a.x;
      const dz = b.z - a.z;
      const l = Math.hypot(dx, dz);
      if (l < 1e-4) continue;
      const nx = (-dz / l) * (width / 2);
      const nz = (dx / l) * (width / 2);
      this.quad(
        new THREE.Vector3(a.x + nx, y, a.z + nz),
        new THREE.Vector3(b.x + nx, y, b.z + nz),
        new THREE.Vector3(b.x - nx, y, b.z - nz),
        new THREE.Vector3(a.x - nx, y, a.z - nz),
        new THREE.Vector3(0, 1, 0),
        color,
      );
    }
  }

  build(): THREE.BufferGeometry {
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.Float32BufferAttribute(this.pos, 3));
    g.setAttribute("normal", new THREE.Float32BufferAttribute(this.nrm, 3));
    g.setAttribute("uv", new THREE.Float32BufferAttribute(this.uv, 2));
    g.setAttribute("color", new THREE.Float32BufferAttribute(this.col, 3));
    g.computeBoundingSphere();
    return g;
  }

  get empty(): boolean {
    return this.pos.length === 0;
  }
}
