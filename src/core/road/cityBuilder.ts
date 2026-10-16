import { Rng } from "../math/rng";
import { Network, Node, Road, RoadOptions } from "./network";
import { assignApproachSigns } from "./controls";
import { ControlType, RoadKind } from "./types";

export interface Building {
  x: number;
  y: number;
  w: number;
  h: number;
  /** Height in metres (0 for parks/plazas). */
  height: number;
  kind: "tower" | "block" | "park" | "plaza" | "garage";
  /** 0..1 hue used by renderers for neon accents. */
  hue: number;
}

export interface SpeedCamera {
  roadId: number;
  /** Arc length along the road centreline. */
  s: number;
  forward: boolean;
}

export interface City {
  network: Network;
  buildings: Building[];
  speedCameras: SpeedCamera[];
  /** Grid lines for convenience. */
  xs: number[];
  ys: number[];
  name: string;
}

export interface CityOptions {
  seed?: number;
  cols?: number;
  rows?: number;
  /** Typical block size (m). */
  blockW?: number;
  blockH?: number;
}

interface LineSpec {
  kind: RoadKind;
  /** Lanes in the increasing-coordinate direction and the decreasing one. */
  inc: number;
  dec: number;
  speed: number;
  name: string;
  width: number;
}

const STREET_NAMES = ["Kestrel", "Ashgrove", "Ozone", "Delamain", "Ironside", "Halcyon", "Corvid", "Lumen", "Tidewater", "Saffron", "Quartz", "Juniper", "Mercury", "Bellwether"];
const AVENUE_NAMES = ["Meridian Ave", "Vanta Ave", "Arcadia Ave", "Helix Ave"];

function lineSpecs(count: number, rng: Rng, axis: "col" | "row"): LineSpec[] {
  const specs: LineSpec[] = [];
  let oneWayToggle = rng.chance(0.5);
  let avenueIdx = 0;
  let streetIdx = axis === "col" ? 0 : 7;
  for (let i = 0; i < count; i++) {
    let s: LineSpec;
    if (i === 0 || i === count - 1) {
      s = { kind: "ring", inc: 2, dec: 2, speed: 70, name: "Perimeter Blvd", width: 0 };
    } else if (i === Math.floor(count / 2) || (count > 8 && i === 2)) {
      s = { kind: "avenue", inc: 3, dec: 3, speed: 60, name: AVENUE_NAMES[avenueIdx], width: 0 };
      avenueIdx = avenueIdx + 1 < AVENUE_NAMES.length ? avenueIdx + 1 : 0;
    } else if (rng.chance(0.35)) {
      oneWayToggle = !oneWayToggle;
      const lanes = rng.chance(0.5) ? 2 : 3;
      s = {
        kind: "oneway",
        inc: oneWayToggle ? lanes : 0,
        dec: oneWayToggle ? 0 : lanes,
        speed: 50,
        name: STREET_NAMES[streetIdx] + " St",
        width: 0,
      };
      streetIdx = streetIdx + 1 < STREET_NAMES.length ? streetIdx + 1 : 0;
    } else {
      const lanes = rng.chance(0.3) ? 2 : 1;
      s = { kind: "street", inc: lanes, dec: lanes, speed: lanes === 2 ? 50 : 40, name: STREET_NAMES[streetIdx] + " St", width: 0 };
      streetIdx = streetIdx + 1 < STREET_NAMES.length ? streetIdx + 1 : 0;
    }
    const median = s.inc > 0 && s.dec > 0 ? (s.kind === "avenue" ? 2 : s.kind === "ring" ? 1.5 : 0.3) : 0;
    s.width = (s.inc + s.dec) * 3.5 + median;
    specs.push(s);
  }
  return specs;
}

function chooseControl(a: LineSpec, b: LineSpec, rng: Rng, roads: number): ControlType {
  if (roads <= 2) return "none";
  const big = (s: LineSpec) => s.kind === "avenue" || s.kind === "ring";
  if (a.kind === "ring" && b.kind === "ring") return "none";
  if (big(a) && big(b)) return "signal";
  if (a.kind === "avenue" || b.kind === "avenue") return "signal";
  if (a.kind === "ring" || b.kind === "ring") return rng.chance(0.4) ? "signal" : "twoway_stop";
  if (a.kind === "oneway" && b.kind === "oneway") return rng.chance(0.6) ? "signal" : "allway_stop";
  if (a.kind === "oneway" || b.kind === "oneway") return rng.chance(0.5) ? "twoway_stop" : "signal";
  return rng.chance(0.65) ? "allway_stop" : "twoway_stop";
}

/**
 * Procedural grid city: a perimeter boulevard, multi-lane avenues with
 * protected left turns, one-way streets and small streets with stop signs.
 */
export function buildCity(opts: CityOptions): City {
  const rng = new Rng(opts.seed ?? 2077);
  const cols = opts.cols ?? 7;
  const rows = opts.rows ?? 6;
  const bw = opts.blockW ?? 110;
  const bh = opts.blockH ?? 95;
  const xs: number[] = [];
  const ys: number[] = [];
  let x = 0;
  for (let i = 0; i < cols; i++) {
    xs.push(x);
    x += bw * rng.range(0.8, 1.2);
  }
  let y = 0;
  for (let j = 0; j < rows; j++) {
    ys.push(y);
    y += bh * rng.range(0.8, 1.2);
  }
  // Centre the city on the origin.
  const cx = xs[cols - 1] / 2;
  const cy = ys[rows - 1] / 2;
  for (let i = 0; i < cols; i++) xs[i] = Math.floor(xs[i] - cx);
  for (let j = 0; j < rows; j++) ys[j] = Math.floor(ys[j] - cy);

  const colSpec = lineSpecs(cols, rng, "col");
  const rowSpec = lineSpecs(rows, rng, "row");
  const net = new Network();
  const grid: Node[][] = [];
  for (let i = 0; i < cols; i++) {
    const colNodes: Node[] = [];
    for (let j = 0; j < rows; j++) {
      const onEdgeX = i === 0 || i === cols - 1;
      const onEdgeY = j === 0 || j === rows - 1;
      const degree = onEdgeX && onEdgeY ? 2 : onEdgeX || onEdgeY ? 3 : 4;
      const n = net.addNode(xs[i], ys[j], chooseControl(colSpec[i], rowSpec[j], rng, degree));
      n.name = `${colSpec[i].name} & ${rowSpec[j].name}`;
      colNodes.push(n);
    }
    grid.push(colNodes);
  }
  const mk = (a: Node, b: Node, s: LineSpec): Road => {
    const o: RoadOptions = {
      lanesFwd: s.inc,
      lanesBack: s.dec,
      speedLimitKmh: s.speed,
      kind: s.kind,
      name: s.name,
      medianWidth: s.inc > 0 && s.dec > 0 ? (s.kind === "avenue" ? 2 : s.kind === "ring" ? 1.5 : 0.3) : 0,
      sidewalkWidth: s.kind === "ring" ? 4 : 3,
    };
    return net.addRoad(a, b, o);
  };
  // Vertical roads (along columns, increasing y = "north").
  for (let i = 0; i < cols; i++) for (let j = 0; j < rows - 1; j++) mk(grid[i][j], grid[i][j + 1], colSpec[i]);
  // Horizontal roads (along rows, increasing x = "east").
  for (let j = 0; j < rows; j++) for (let i = 0; i < cols - 1; i++) mk(grid[i][j], grid[i + 1][j], rowSpec[j]);
  net.finalize();
  assignApproachSigns(net);

  for (const n of net.nodes) if (n.control === "signal" && rng.chance(0.3)) n.redLightCamera = true;

  const speedCameras: SpeedCamera[] = [];
  for (const r of net.roads) {
    if ((r.kind === "avenue" || r.kind === "ring") && rng.chance(0.15) && r.centerPath.length > 60) {
      r.speedCamera = true;
      speedCameras.push({ roadId: r.id, s: r.centerPath.length / 2, forward: rng.chance(0.5) });
    }
  }

  // Buildings fill each block, inset from the curbs by the sidewalk.
  const buildings: Building[] = [];
  const maxD = Math.sqrt(cx * cx + cy * cy);
  for (let i = 0; i < cols - 1; i++) {
    for (let j = 0; j < rows - 1; j++) {
      const x0 = xs[i] + colSpec[i].width / 2 + 3.5;
      const x1 = xs[i + 1] - colSpec[i + 1].width / 2 - 3.5;
      const y0 = ys[j] + rowSpec[j].width / 2 + 3.5;
      const y1 = ys[j + 1] - rowSpec[j + 1].width / 2 - 3.5;
      if (x1 - x0 < 10 || y1 - y0 < 10) continue;
      const mx = (x0 + x1) / 2;
      const my = (y0 + y1) / 2;
      const centrality = 1 - Math.sqrt(mx * mx + my * my) / maxD;
      if (rng.chance(0.08)) {
        buildings.push({ x: mx, y: my, w: x1 - x0, h: y1 - y0, height: 0, kind: rng.chance(0.5) ? "park" : "plaza", hue: rng.next() });
        continue;
      }
      const nx = x1 - x0 > 70 ? 2 : 1;
      const ny = y1 - y0 > 60 ? 2 : 1;
      const gap = 5;
      const cw = (x1 - x0 - gap * (nx - 1)) / nx;
      const ch = (y1 - y0 - gap * (ny - 1)) / ny;
      for (let a = 0; a < nx; a++) {
        for (let b = 0; b < ny; b++) {
          const bx = x0 + a * (cw + gap) + cw / 2;
          const by = y0 + b * (ch + gap) + ch / 2;
          const tall = rng.chance(0.25 + centrality * 0.5);
          const height = tall ? rng.range(45, 90) + centrality * 120 : rng.range(10, 30);
          buildings.push({
            x: bx,
            y: by,
            w: cw * rng.range(0.82, 1),
            h: ch * rng.range(0.82, 1),
            height,
            kind: tall ? "tower" : rng.chance(0.1) ? "garage" : "block",
            hue: rng.next(),
          });
        }
      }
    }
  }
  return { network: net, buildings, speedCameras, xs, ys, name: "Gridlock City" };
}
