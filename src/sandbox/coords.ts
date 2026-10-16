import * as THREE from "three";

/** Simulation plane (x east, y north) -> three.js (x right, y up, z toward viewer). */
export function toV3(x: number, y: number, h: number, out?: THREE.Vector3): THREE.Vector3 {
  const v = out ?? new THREE.Vector3();
  return v.set(x, h, -y);
}

/** Simulation heading (CCW from +x) -> three.js rotation about Y. */
export function yawOf(heading: number): number {
  return heading;
}
