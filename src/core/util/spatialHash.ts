/**
 * Uniform grid spatial hash for broad-phase proximity queries. Cell keys are
 * numbers (not strings) to keep the Lua build fast.
 */
export class SpatialHash<T> {
  private cells: Map<number, T[]> = new Map();
  private readonly inv: number;

  constructor(readonly cellSize: number) {
    this.inv = 1 / cellSize;
  }

  private key(cx: number, cy: number): number {
    // Offset so negative coordinates map to unique positive keys.
    return (cx + 32768) * 65536 + (cy + 32768);
  }

  clear(): void {
    this.cells = new Map();
  }

  insert(item: T, x: number, y: number, radius: number): void {
    const x0 = Math.floor((x - radius) * this.inv);
    const x1 = Math.floor((x + radius) * this.inv);
    const y0 = Math.floor((y - radius) * this.inv);
    const y1 = Math.floor((y + radius) * this.inv);
    for (let cx = x0; cx <= x1; cx++) {
      for (let cy = y0; cy <= y1; cy++) {
        const k = this.key(cx, cy);
        let cell = this.cells.get(k);
        if (cell === undefined) {
          cell = [];
          this.cells.set(k, cell);
        }
        cell.push(item);
      }
    }
  }

  /** Items whose cells overlap the query circle. May contain duplicates when items span cells. */
  query(x: number, y: number, radius: number, out: T[]): T[] {
    const x0 = Math.floor((x - radius) * this.inv);
    const x1 = Math.floor((x + radius) * this.inv);
    const y0 = Math.floor((y - radius) * this.inv);
    const y1 = Math.floor((y + radius) * this.inv);
    for (let cx = x0; cx <= x1; cx++) {
      for (let cy = y0; cy <= y1; cy++) {
        const cell = this.cells.get(this.key(cx, cy));
        if (cell !== undefined) {
          for (let i = 0; i < cell.length; i++) {
            const it = cell[i];
            if (out.indexOf(it) < 0) out.push(it);
          }
        }
      }
    }
    return out;
  }
}
