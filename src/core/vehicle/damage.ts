import { clamp } from "../math/scalar";

export type DamageZone = "front" | "rear" | "left" | "right";

/** Per-zone body damage plus derived mechanical effects. */
export class DamageModel {
  front = 1;
  rear = 1;
  left = 1;
  right = 1;
  /** Total impulse absorbed (for stats). */
  impacts = 0;

  /** Overall body health 0..1. */
  get body(): number {
    return (this.front + this.rear + this.left + this.right) / 4;
  }

  get totaled(): boolean {
    return this.body < 0.2 || this.front < 0.05;
  }

  /**
   * Apply an impact. `localX/localY` is the contact point in the body frame
   * (x forward), `severity` the change of velocity (m/s).
   * Returns the zone hit and the health lost.
   */
  hit(localX: number, localY: number, halfLen: number, halfWid: number, severity: number): { zone: DamageZone; lost: number } {
    const nx = localX / halfLen;
    const ny = localY / halfWid;
    let zone: DamageZone;
    if (Math.abs(nx) >= Math.abs(ny)) zone = nx >= 0 ? "front" : "rear";
    else zone = ny >= 0 ? "left" : "right";
    // Small bumps (< 1.5 m/s) do nothing; 15 m/s wrecks a zone.
    const lost = clamp((severity - 1.5) / 14, 0, 1);
    if (lost > 0) {
      if (zone === "front") this.front = Math.max(0, this.front - lost);
      else if (zone === "rear") this.rear = Math.max(0, this.rear - lost);
      else if (zone === "left") this.left = Math.max(0, this.left - lost);
      else this.right = Math.max(0, this.right - lost);
      this.impacts += severity;
    }
    return { zone, lost };
  }

  repair(): void {
    this.front = 1;
    this.rear = 1;
    this.left = 1;
    this.right = 1;
  }
}
