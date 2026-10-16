import * as THREE from "three";
import { Vehicle } from "../core/vehicle/vehicle";

export type CameraMode = "chase" | "far" | "hood" | "cockpit" | "top" | "cinematic";
export const CAMERA_MODES: CameraMode[] = ["chase", "far", "hood", "cockpit", "top", "cinematic"];
export const CAMERA_LABEL: Record<CameraMode, string> = {
  chase: "Chase",
  far: "Far chase",
  hood: "Hood",
  cockpit: "Cockpit",
  top: "Top-down",
  cinematic: "Cinematic",
};

/** Follow cameras with spring smoothing, look-around and speed FOV. */
export class CameraRig {
  readonly camera: THREE.PerspectiveCamera;
  mode: CameraMode = "chase";
  /** -1 left, 1 right, 2 back, 0 forward */
  look = 0;
  private pos = new THREE.Vector3();
  private target = new THREE.Vector3();
  private initialized = false;
  private orbitYaw = 0;
  private orbitPitch = 0;
  private orbitTimer = 0;
  private cineSpot = new THREE.Vector3();
  private cineTimer = 0;
  private shake = 0;

  constructor(aspect: number) {
    this.camera = new THREE.PerspectiveCamera(65, aspect, 0.1, 2500);
  }

  cycle(): CameraMode {
    const i = CAMERA_MODES.indexOf(this.mode);
    this.mode = CAMERA_MODES[(i + 1) % CAMERA_MODES.length];
    this.initialized = false;
    return this.mode;
  }

  /** Mouse/stick orbit offset (chase cameras), springs back after a moment. */
  orbit(dx: number, dy: number): void {
    this.orbitYaw -= dx;
    this.orbitPitch = THREE.MathUtils.clamp(this.orbitPitch + dy, -0.3, 0.9);
    this.orbitTimer = 2.5;
  }

  bump(amount: number): void {
    this.shake = Math.min(1, this.shake + amount);
  }

  update(dt: number, v: Vehicle, x: number, y: number, heading: number): void {
    const cam = this.camera;
    const speed = v.absSpeed();
    const fwd = new THREE.Vector3(Math.cos(heading), 0, -Math.sin(heading));
    const right = new THREE.Vector3(Math.sin(heading), 0, Math.cos(heading));
    const base = new THREE.Vector3(x, 0, -y);
    let lookYaw = this.look === 2 ? Math.PI : this.look === -1 ? Math.PI / 2 : this.look === 1 ? -Math.PI / 2 : 0;
    if (this.orbitTimer > 0) this.orbitTimer -= dt;
    else {
      this.orbitYaw *= Math.max(0, 1 - dt * 3);
      this.orbitPitch *= Math.max(0, 1 - dt * 3);
    }
    lookYaw += this.orbitYaw;
    const dir = fwd.clone().applyAxisAngle(new THREE.Vector3(0, 1, 0), lookYaw);
    let wantPos = new THREE.Vector3();
    let wantTarget = new THREE.Vector3();
    let stiffness = 6;
    let fov = 62;
    const L = v.spec.length;
    const H = v.spec.height;
    switch (this.mode) {
      case "chase":
      case "far": {
        const far = this.mode === "far";
        const back = (far ? 11 : 5.2) + L * 0.55 + speed * 0.03;
        const up = (far ? 4.2 : 1.6) + H * 0.6 + this.orbitPitch * 6;
        wantPos = base.clone().addScaledVector(dir, -back).setY(up);
        wantTarget = base.clone().addScaledVector(dir, 4).setY(H * 0.75);
        stiffness = far ? 4 : 7;
        fov = 60 + Math.min(speed, 50) * 0.32;
        break;
      }
      case "hood": {
        wantPos = base.clone().addScaledVector(fwd, L * 0.12).setY(H + 0.12);
        wantTarget = wantPos.clone().addScaledVector(dir, 20).setY(H * 0.8);
        stiffness = 60;
        fov = 68 + Math.min(speed, 50) * 0.2;
        break;
      }
      case "cockpit": {
        wantPos = base.clone().addScaledVector(fwd, -L * 0.08).addScaledVector(right, -0.36).setY(Math.max(1.12, H * 0.9));
        wantTarget = wantPos.clone().addScaledVector(dir, 20).setY(H * 0.74);
        stiffness = 80;
        fov = 72;
        break;
      }
      case "top": {
        wantPos = base.clone().addScaledVector(fwd, 8).setY(55 + speed * 0.5);
        wantTarget = base.clone().addScaledVector(fwd, 9);
        stiffness = 4;
        fov = 55;
        break;
      }
      case "cinematic": {
        this.cineTimer -= dt;
        const d = this.cineSpot.distanceTo(base);
        if (this.cineTimer <= 0 || d > 70) {
          this.cineTimer = 7;
          const ahead = base.clone().addScaledVector(fwd, 25 + speed * 2.5).addScaledVector(right, (Math.random() < 0.5 ? -1 : 1) * (6 + Math.random() * 6));
          this.cineSpot.copy(ahead).setY(1.2 + Math.random() * 5);
          this.initialized = false;
        }
        wantPos = this.cineSpot.clone();
        wantTarget = base.clone().setY(1);
        stiffness = 100;
        fov = THREE.MathUtils.clamp(50 - d * 0.25, 22, 55);
        break;
      }
    }
    if (!this.initialized) {
      this.pos.copy(wantPos);
      this.target.copy(wantTarget);
      this.initialized = true;
    }
    const k = 1 - Math.exp(-stiffness * dt);
    this.pos.lerp(wantPos, k);
    this.target.lerp(wantTarget, Math.min(1, k * 1.6));
    if (this.mode === "hood" || this.mode === "cockpit") {
      this.pos.copy(wantPos);
      this.target.copy(wantTarget);
    }
    cam.position.copy(this.pos);
    // Shake from impacts and rough speed.
    this.shake = Math.max(0, this.shake - dt * 2.5);
    const s = this.shake * 0.25 + (this.mode === "chase" ? Math.min(speed / 60, 1) * 0.008 : 0);
    if (s > 0) cam.position.add(new THREE.Vector3((Math.random() - 0.5) * s, (Math.random() - 0.5) * s, (Math.random() - 0.5) * s));
    cam.lookAt(this.target);
    if (this.mode === "cockpit") cam.rotateZ(-v.latAccel * 0.004);
    cam.fov += (fov - cam.fov) * Math.min(1, dt * 3);
    cam.updateProjectionMatrix();
  }

  /** True when the view is from inside/at the windshield (for rain-on-glass). */
  get onGlass(): boolean {
    return this.mode === "hood" || this.mode === "cockpit";
  }
}
