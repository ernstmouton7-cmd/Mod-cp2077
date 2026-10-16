import { clamp, mod, smoothstep } from "../math/scalar";
import { Rng } from "../math/rng";

export type Weather = "clear" | "cloudy" | "rain" | "storm" | "fog";

/** Time of day and weather, with their effects on driving. */
export class Environment {
  /** Hours 0..24. */
  hour: number;
  /** Simulated hours per real second (default: 1 game minute per second). */
  timeScale = 1 / 60;
  weather: Weather = "clear";
  /** 0..1 rain intensity (smoothed). */
  rain = 0;
  /** 0..1 fog density (smoothed). */
  fog = 0;
  /** Road wetness lags rain. */
  wetness = 0;
  private weatherTimer = 600;
  autoWeather = true;
  private rng: Rng;

  constructor(seed: number, startHour: number) {
    this.hour = startHour;
    this.rng = new Rng(seed);
  }

  /** 0 (night) .. 1 (day). */
  get daylight(): number {
    const h = this.hour;
    return smoothstep(5.5, 7.5, h) * (1 - smoothstep(18.5, 20.5, h));
  }

  get dark(): boolean {
    return this.daylight < 0.45 || this.fog > 0.6;
  }

  /** Tyre grip multiplier from road wetness. */
  get grip(): number {
    return 1 - 0.32 * this.wetness;
  }

  /** Visibility (m) for AI caution and HUD. */
  get visibility(): number {
    const night = 1 - this.daylight;
    return clamp(1200 * (1 - this.fog * 0.9) * (1 - this.rain * 0.5) * (1 - night * 0.4), 40, 1200);
  }

  /** Traffic demand multiplier by hour (rush hours peak). */
  get trafficDensity(): number {
    const h = this.hour;
    const bell = (c: number, w: number) => Math.exp(-((h - c) * (h - c)) / (2 * w * w));
    return clamp(0.25 + 0.75 * bell(8.2, 1.4) + 0.85 * bell(17.6, 1.7) + 0.45 * bell(13, 2.5) + 0.25 * bell(21.5, 1.5), 0.2, 1.1);
  }

  /** Late night: some signals switch to flashing operation. */
  get nightFlash(): boolean {
    return this.hour >= 1 && this.hour < 5;
  }

  setWeather(w: Weather): void {
    this.weather = w;
    this.weatherTimer = 900;
  }

  update(dt: number): void {
    this.hour = mod(this.hour + dt * this.timeScale, 24);
    if (this.autoWeather) {
      this.weatherTimer -= dt;
      if (this.weatherTimer <= 0) {
        this.weather = this.rng.weighted([
          { item: "clear" as Weather, weight: 45 },
          { item: "cloudy" as Weather, weight: 25 },
          { item: "rain" as Weather, weight: 18 },
          { item: "storm" as Weather, weight: 6 },
          { item: "fog" as Weather, weight: 6 },
        ]);
        this.weatherTimer = this.rng.range(400, 1200);
      }
    }
    const targetRain = this.weather === "rain" ? 0.6 : this.weather === "storm" ? 1 : 0;
    const targetFog = this.weather === "fog" ? 0.8 : this.weather === "storm" ? 0.3 : 0;
    this.rain += (targetRain - this.rain) * Math.min(1, dt * 0.1);
    this.fog += (targetFog - this.fog) * Math.min(1, dt * 0.05);
    const wetTarget = this.rain > 0.05 ? Math.min(1, this.rain * 1.5) : 0;
    this.wetness += (wetTarget - this.wetness) * Math.min(1, dt * (wetTarget > this.wetness ? 0.08 : 0.01));
  }
}
