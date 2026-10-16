/**
 * Vehicle archetypes. Numbers are loosely based on real cars in each class so
 * the dynamics feel plausible (0-100 km/h times, top speeds, braking).
 */
export type VehicleClass =
  | "compact"
  | "sedan"
  | "sports"
  | "muscle"
  | "suv"
  | "van"
  | "truck"
  | "bus"
  | "taxi"
  | "police"
  | "ambulance";

export type DriveLayout = "fwd" | "rwd" | "awd";

export interface VehicleSpec {
  id: VehicleClass;
  name: string;
  length: number;
  width: number;
  height: number;
  wheelbase: number;
  /** Fraction of static weight on the front axle. */
  frontWeight: number;
  cgHeight: number;
  mass: number;
  wheelRadius: number;
  idleRpm: number;
  redline: number;
  /** [rpm, torque N·m] at full throttle. */
  torqueCurve: ReadonlyArray<readonly [number, number]>;
  /** Engine braking torque at redline with throttle closed (N·m). */
  engineBrake: number;
  gears: ReadonlyArray<number>;
  reverseRatio: number;
  finalDrive: number;
  shiftTime: number;
  drive: DriveLayout;
  /** Max total brake torque at the wheels (N·m), split by brakeBias. */
  brakeTorque: number;
  brakeBias: number;
  /** Peak tyre friction coefficient on dry asphalt. */
  grip: number;
  /** Lateral tyre stiffness factor (B in the magic formula). */
  tireB: number;
  tireC: number;
  /** Drag area Cd*A (m^2). */
  dragArea: number;
  rollResist: number;
  maxSteer: number;
  fuelCapacity: number;
  /** Rough peak power (kW) for AI acceleration limits. */
  powerKw: number;
  emergency: boolean;
  /** Audio hint: number of cylinders. */
  cylinders: number;
  /** Paint choices (hex RGB numbers). */
  colors: ReadonlyArray<number>;
  /** Relative spawn frequency in civilian traffic. */
  spawnWeight: number;
}

const civColors = [0xe8e8e8, 0x1a1a1a, 0x8a8f99, 0x3a4a6b, 0x7a1f1f, 0x2b5d3a, 0xc9b28a, 0x2a2a35, 0xd8d0c0, 0x4b2f5e];
const brightColors = [0xff2a6d, 0x05d9e8, 0xf9c80e, 0xff6c11, 0x7b2ff7, 0x01ff89, 0xe0e0e0, 0x111111];

export const SPECS: Record<VehicleClass, VehicleSpec> = {
  compact: {
    id: "compact", name: "Makigai-style Compact", length: 3.9, width: 1.72, height: 1.45, wheelbase: 2.5, frontWeight: 0.6, cgHeight: 0.5,
    mass: 1100, wheelRadius: 0.29, idleRpm: 800, redline: 6500,
    torqueCurve: [[800, 90], [2000, 125], [3500, 140], [5000, 132], [6500, 105]], engineBrake: 35,
    gears: [3.5, 2.0, 1.35, 1.0, 0.8], reverseRatio: 3.3, finalDrive: 4.1, shiftTime: 0.35, drive: "fwd",
    brakeTorque: 4200, brakeBias: 0.68, grip: 1.0, tireB: 9, tireC: 1.4, dragArea: 0.66, rollResist: 0.013, maxSteer: 0.62,
    fuelCapacity: 40, powerKw: 75, emergency: false, cylinders: 4, colors: civColors.concat([0xf2e14c, 0x6fb7d9]), spawnWeight: 22,
  },
  sedan: {
    id: "sedan", name: "Archer-style Sedan", length: 4.8, width: 1.85, height: 1.45, wheelbase: 2.85, frontWeight: 0.57, cgHeight: 0.52,
    mass: 1550, wheelRadius: 0.32, idleRpm: 750, redline: 6300,
    torqueCurve: [[750, 160], [2000, 240], [4000, 260], [5500, 240], [6300, 200]], engineBrake: 45,
    gears: [3.6, 2.2, 1.5, 1.15, 0.92, 0.75], reverseRatio: 3.4, finalDrive: 3.7, shiftTime: 0.3, drive: "fwd",
    brakeTorque: 6200, brakeBias: 0.66, grip: 1.0, tireB: 9, tireC: 1.45, dragArea: 0.62, rollResist: 0.012, maxSteer: 0.6,
    fuelCapacity: 60, powerKw: 140, emergency: false, cylinders: 4, colors: civColors, spawnWeight: 30,
  },
  sports: {
    id: "sports", name: "Quadra-style Sports Coupe", length: 4.5, width: 1.95, height: 1.22, wheelbase: 2.65, frontWeight: 0.48, cgHeight: 0.42,
    mass: 1380, wheelRadius: 0.34, idleRpm: 900, redline: 8000,
    torqueCurve: [[900, 250], [3000, 420], [5500, 470], [7000, 440], [8000, 380]], engineBrake: 60,
    gears: [3.1, 2.1, 1.6, 1.28, 1.05, 0.86], reverseRatio: 3.0, finalDrive: 3.6, shiftTime: 0.12, drive: "rwd",
    brakeTorque: 9000, brakeBias: 0.62, grip: 1.15, tireB: 10, tireC: 1.5, dragArea: 0.55, rollResist: 0.011, maxSteer: 0.56,
    fuelCapacity: 65, powerKw: 360, emergency: false, cylinders: 8, colors: brightColors, spawnWeight: 6,
  },
  muscle: {
    id: "muscle", name: "Thorton-style Muscle Car", length: 4.9, width: 1.95, height: 1.35, wheelbase: 2.9, frontWeight: 0.55, cgHeight: 0.5,
    mass: 1650, wheelRadius: 0.34, idleRpm: 700, redline: 6200,
    torqueCurve: [[700, 380], [2500, 560], [4000, 600], [5500, 540], [6200, 470]], engineBrake: 70,
    gears: [2.9, 1.9, 1.35, 1.0, 0.8], reverseRatio: 2.9, finalDrive: 3.3, shiftTime: 0.25, drive: "rwd",
    brakeTorque: 7600, brakeBias: 0.64, grip: 1.0, tireB: 8.5, tireC: 1.45, dragArea: 0.72, rollResist: 0.013, maxSteer: 0.58,
    fuelCapacity: 70, powerKw: 320, emergency: false, cylinders: 8, colors: brightColors.concat(civColors), spawnWeight: 8,
  },
  suv: {
    id: "suv", name: "Chevillon-style SUV", length: 4.95, width: 1.98, height: 1.8, wheelbase: 2.95, frontWeight: 0.55, cgHeight: 0.72,
    mass: 2150, wheelRadius: 0.37, idleRpm: 700, redline: 6000,
    torqueCurve: [[700, 260], [2000, 380], [3500, 420], [5000, 380], [6000, 320]], engineBrake: 60,
    gears: [4.0, 2.4, 1.6, 1.2, 1.0, 0.8], reverseRatio: 3.6, finalDrive: 3.5, shiftTime: 0.3, drive: "awd",
    brakeTorque: 9500, brakeBias: 0.65, grip: 0.95, tireB: 8, tireC: 1.4, dragArea: 1.0, rollResist: 0.015, maxSteer: 0.6,
    fuelCapacity: 80, powerKw: 210, emergency: false, cylinders: 6, colors: civColors, spawnWeight: 16,
  },
  van: {
    id: "van", name: "Mahir-style Van", length: 5.3, width: 2.0, height: 2.2, wheelbase: 3.3, frontWeight: 0.55, cgHeight: 0.85,
    mass: 2400, wheelRadius: 0.36, idleRpm: 750, redline: 5000,
    torqueCurve: [[750, 260], [1800, 380], [3000, 390], [4200, 340], [5000, 280]], engineBrake: 60,
    gears: [4.2, 2.5, 1.6, 1.2, 1.0, 0.82], reverseRatio: 3.8, finalDrive: 3.9, shiftTime: 0.35, drive: "rwd",
    brakeTorque: 9500, brakeBias: 0.65, grip: 0.9, tireB: 7.5, tireC: 1.4, dragArea: 1.4, rollResist: 0.016, maxSteer: 0.62,
    fuelCapacity: 80, powerKw: 130, emergency: false, cylinders: 4, colors: [0xffffff, 0xd0d0d0, 0x28324a, 0x8a1c1c, 0xe0a030], spawnWeight: 7,
  },
  truck: {
    id: "truck", name: "Kaukaz-style Box Truck", length: 7.8, width: 2.45, height: 3.4, wheelbase: 4.8, frontWeight: 0.5, cgHeight: 1.2,
    mass: 7500, wheelRadius: 0.5, idleRpm: 600, redline: 2600,
    torqueCurve: [[600, 900], [1200, 1400], [1800, 1450], [2300, 1200], [2600, 1000]], engineBrake: 400,
    gears: [6.5, 4.0, 2.6, 1.8, 1.3, 1.0], reverseRatio: 6.0, finalDrive: 4.1, shiftTime: 0.6, drive: "rwd",
    brakeTorque: 38000, brakeBias: 0.55, grip: 0.85, tireB: 7, tireC: 1.35, dragArea: 5.5, rollResist: 0.008, maxSteer: 0.6,
    fuelCapacity: 300, powerKw: 220, emergency: false, cylinders: 6, colors: [0xf0f0f0, 0xd04020, 0x204070, 0x2f2f2f], spawnWeight: 4,
  },
  bus: {
    id: "bus", name: "City Bus", length: 12, width: 2.55, height: 3.2, wheelbase: 6.2, frontWeight: 0.38, cgHeight: 1.1,
    mass: 12000, wheelRadius: 0.5, idleRpm: 600, redline: 2400,
    torqueCurve: [[600, 1200], [1100, 1700], [1600, 1750], [2100, 1500], [2400, 1250]], engineBrake: 500,
    gears: [4.7, 2.6, 1.6, 1.15, 1.0, 0.8], reverseRatio: 4.5, finalDrive: 5.2, shiftTime: 0.5, drive: "rwd",
    brakeTorque: 60000, brakeBias: 0.5, grip: 0.85, tireB: 7, tireC: 1.35, dragArea: 7.0, rollResist: 0.008, maxSteer: 0.62,
    fuelCapacity: 350, powerKw: 230, emergency: false, cylinders: 6, colors: [0xf2c94c, 0x2d9cdb, 0xe0e0e0], spawnWeight: 2,
  },
  taxi: {
    id: "taxi", name: "Delamain-style Cab", length: 4.85, width: 1.9, height: 1.5, wheelbase: 2.85, frontWeight: 0.56, cgHeight: 0.52,
    mass: 1600, wheelRadius: 0.32, idleRpm: 750, redline: 6300,
    torqueCurve: [[750, 200], [2000, 300], [4000, 320], [5500, 290], [6300, 250]], engineBrake: 45,
    gears: [3.6, 2.2, 1.5, 1.15, 0.92, 0.75], reverseRatio: 3.4, finalDrive: 3.7, shiftTime: 0.25, drive: "awd",
    brakeTorque: 6600, brakeBias: 0.66, grip: 1.02, tireB: 9, tireC: 1.45, dragArea: 0.6, rollResist: 0.012, maxSteer: 0.6,
    fuelCapacity: 60, powerKw: 180, emergency: false, cylinders: 6, colors: [0xf2b705, 0x111111], spawnWeight: 7,
  },
  police: {
    id: "police", name: "Patrol Interceptor", length: 5.0, width: 1.95, height: 1.5, wheelbase: 2.95, frontWeight: 0.54, cgHeight: 0.5,
    mass: 1800, wheelRadius: 0.34, idleRpm: 750, redline: 6800,
    torqueCurve: [[750, 300], [2500, 470], [4500, 500], [6000, 460], [6800, 400]], engineBrake: 55,
    gears: [3.2, 2.1, 1.5, 1.15, 0.92, 0.75], reverseRatio: 3.2, finalDrive: 3.5, shiftTime: 0.18, drive: "awd",
    brakeTorque: 9800, brakeBias: 0.63, grip: 1.1, tireB: 9.5, tireC: 1.5, dragArea: 0.66, rollResist: 0.012, maxSteer: 0.6,
    fuelCapacity: 75, powerKw: 300, emergency: true, cylinders: 8, colors: [0x101820], spawnWeight: 0,
  },
  ambulance: {
    id: "ambulance", name: "Trauma Response Unit", length: 6.4, width: 2.2, height: 2.7, wheelbase: 3.9, frontWeight: 0.5, cgHeight: 1.0,
    mass: 4200, wheelRadius: 0.4, idleRpm: 700, redline: 5000,
    torqueCurve: [[700, 450], [1800, 640], [3000, 660], [4200, 580], [5000, 500]], engineBrake: 120,
    gears: [4.0, 2.4, 1.6, 1.2, 1.0, 0.8], reverseRatio: 3.8, finalDrive: 4.1, shiftTime: 0.35, drive: "rwd",
    brakeTorque: 20000, brakeBias: 0.6, grip: 0.9, tireB: 7.5, tireC: 1.4, dragArea: 2.6, rollResist: 0.014, maxSteer: 0.62,
    fuelCapacity: 120, powerKw: 240, emergency: true, cylinders: 8, colors: [0xf4f4f4], spawnWeight: 0,
  },
};

export const CIVILIAN_CLASSES: VehicleClass[] = ["compact", "sedan", "sports", "muscle", "suv", "van", "truck", "bus", "taxi"];

/** Approximate max acceleration (m/s^2) at speed v, used by lane-mode AI. */
export function maxAccelAt(spec: VehicleSpec, v: number): number {
  const traction = spec.grip * 9.81 * (spec.drive === "awd" ? 0.95 : spec.drive === "rwd" ? 1 - spec.frontWeight + 0.08 : spec.frontWeight - 0.06);
  const power = (spec.powerKw * 1000 * 0.85) / (spec.mass * Math.max(v, 3));
  const drag = (0.5 * 1.2 * spec.dragArea * v * v) / spec.mass + spec.rollResist * 9.81;
  return Math.max(0.3, Math.min(traction, power) - drag);
}

/** Approximate top speed (m/s) from power and drag. */
export function topSpeed(spec: VehicleSpec): number {
  // Solve P = (0.5 rho CdA v^2 + Crr m g) v by bisection.
  let lo = 1;
  let hi = 120;
  const p = spec.powerKw * 1000 * 0.85;
  for (let i = 0; i < 40; i++) {
    const v = (lo + hi) / 2;
    const need = (0.5 * 1.2 * spec.dragArea * v * v + spec.rollResist * spec.mass * 9.81) * v;
    if (need > p) hi = v;
    else lo = v;
  }
  return lo;
}
