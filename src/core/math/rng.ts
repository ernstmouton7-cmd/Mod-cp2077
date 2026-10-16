/**
 * Deterministic pseudo random generator (Park–Miller minimal standard).
 * Uses only double arithmetic that is exact below 2^53, so JS and Lua produce
 * the same sequence for the same seed.
 */
export class Rng {
  private state: number;

  constructor(seed: number) {
    let s = Math.floor(Math.abs(seed)) - Math.floor(Math.abs(seed) / 2147483646) * 2147483646;
    if (s <= 0) s = s + 2147483646;
    this.state = s;
  }

  /** Uniform in [0, 1). */
  next(): number {
    const prod = this.state * 16807;
    this.state = prod - Math.floor(prod / 2147483647) * 2147483647;
    return (this.state - 1) / 2147483646;
  }

  range(lo: number, hi: number): number {
    return lo + (hi - lo) * this.next();
  }

  /** Integer in [lo, hi] inclusive. */
  int(lo: number, hi: number): number {
    return lo + Math.floor(this.next() * (hi - lo + 1));
  }

  chance(p: number): boolean {
    return this.next() < p;
  }

  pick<T>(items: ReadonlyArray<T>): T {
    return items[Math.floor(this.next() * items.length)];
  }

  /** Approximately normal (sum of uniforms), clamped to +-3 sigma. */
  gauss(mean: number, sigma: number): number {
    const u = this.next() + this.next() + this.next() + this.next() - 2;
    return mean + sigma * u * 1.7320508;
  }

  /** Pick from weighted entries. */
  weighted<T>(entries: ReadonlyArray<{ readonly item: T; readonly weight: number }>): T {
    let total = 0;
    for (const e of entries) total += e.weight;
    let r = this.next() * total;
    for (const e of entries) {
      r -= e.weight;
      if (r <= 0) return e.item;
    }
    return entries[entries.length - 1].item;
  }

  fork(): Rng {
    return new Rng(Math.floor(this.next() * 2147483000) + 1);
  }
}
