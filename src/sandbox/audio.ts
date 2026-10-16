import { SimEvent } from "../core/world/events";
import { Vehicle } from "../core/vehicle/vehicle";
import { World } from "../core/world/world";

function noiseBuffer(ctx: AudioContext, seconds: number): AudioBuffer {
  const b = ctx.createBuffer(1, Math.floor(ctx.sampleRate * seconds), ctx.sampleRate);
  const d = b.getChannelData(0);
  for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
  return b;
}

/** Procedural vehicle audio (no samples). */
export class AudioEngine {
  ctx: AudioContext | undefined;
  private master!: GainNode;
  private engOsc1!: OscillatorNode;
  private engOsc2!: OscillatorNode;
  private engFilter!: BiquadFilterNode;
  private engGain!: GainNode;
  private intakeGain!: GainNode;
  private intakeFilter!: BiquadFilterNode;
  private skidGain!: GainNode;
  private windGain!: GainNode;
  private rainGain!: GainNode;
  private hornGain!: GainNode;
  private sirenOsc!: OscillatorNode;
  private sirenGain!: GainNode;
  private sirenPan!: StereoPannerNode;
  private noise!: AudioBuffer;
  private lastBlink = false;
  private reverseBeep = 0;
  private sirenPhase = 0;
  volume = 0.7;
  muted = false;

  start(): void {
    if (this.ctx !== undefined) {
      void this.ctx.resume();
      return;
    }
    const ctx = new AudioContext();
    this.ctx = ctx;
    this.noise = noiseBuffer(ctx, 2);
    this.master = ctx.createGain();
    this.master.gain.value = this.volume;
    const comp = ctx.createDynamicsCompressor();
    this.master.connect(comp).connect(ctx.destination);
    // Engine: two detuned oscillators through a load-dependent low-pass.
    this.engOsc1 = ctx.createOscillator();
    this.engOsc1.type = "sawtooth";
    this.engOsc2 = ctx.createOscillator();
    this.engOsc2.type = "square";
    this.engFilter = ctx.createBiquadFilter();
    this.engFilter.type = "lowpass";
    this.engFilter.Q.value = 4;
    this.engGain = ctx.createGain();
    this.engGain.gain.value = 0;
    const g2 = ctx.createGain();
    g2.gain.value = 0.5;
    this.engOsc1.connect(this.engFilter);
    this.engOsc2.connect(g2).connect(this.engFilter);
    this.engFilter.connect(this.engGain).connect(this.master);
    this.engOsc1.start();
    this.engOsc2.start();
    // Intake/exhaust roar (filtered noise).
    const intake = this.loopNoise();
    this.intakeFilter = ctx.createBiquadFilter();
    this.intakeFilter.type = "bandpass";
    this.intakeFilter.Q.value = 1.2;
    this.intakeGain = ctx.createGain();
    this.intakeGain.gain.value = 0;
    intake.connect(this.intakeFilter).connect(this.intakeGain).connect(this.master);
    // Tyre squeal.
    const skid = this.loopNoise();
    const skidF = ctx.createBiquadFilter();
    skidF.type = "bandpass";
    skidF.frequency.value = 1400;
    skidF.Q.value = 6;
    this.skidGain = ctx.createGain();
    this.skidGain.gain.value = 0;
    skid.connect(skidF).connect(this.skidGain).connect(this.master);
    // Wind.
    const wind = this.loopNoise();
    const windF = ctx.createBiquadFilter();
    windF.type = "lowpass";
    windF.frequency.value = 500;
    this.windGain = ctx.createGain();
    this.windGain.gain.value = 0;
    wind.connect(windF).connect(this.windGain).connect(this.master);
    // Rain.
    const rain = this.loopNoise();
    const rainF = ctx.createBiquadFilter();
    rainF.type = "highpass";
    rainF.frequency.value = 2500;
    this.rainGain = ctx.createGain();
    this.rainGain.gain.value = 0;
    rain.connect(rainF).connect(this.rainGain).connect(this.master);
    // Horn: two square tones (a dissonant 4th like real dual horns).
    this.hornGain = ctx.createGain();
    this.hornGain.gain.value = 0;
    const hf = ctx.createBiquadFilter();
    hf.type = "lowpass";
    hf.frequency.value = 2200;
    for (const f of [415, 500]) {
      const o = ctx.createOscillator();
      o.type = "square";
      o.frequency.value = f;
      o.connect(hf);
      o.start();
    }
    hf.connect(this.hornGain).connect(this.master);
    // Siren of the nearest emergency vehicle.
    this.sirenOsc = ctx.createOscillator();
    this.sirenOsc.type = "triangle";
    this.sirenGain = ctx.createGain();
    this.sirenGain.gain.value = 0;
    this.sirenPan = ctx.createStereoPanner();
    this.sirenOsc.connect(this.sirenGain).connect(this.sirenPan).connect(this.master);
    this.sirenOsc.start();
  }

  private loopNoise(): AudioBufferSourceNode {
    const ctx = this.ctx as AudioContext;
    const s = ctx.createBufferSource();
    s.buffer = this.noise;
    s.loop = true;
    s.loopStart = Math.random();
    s.start(0, Math.random() * 1.5);
    return s;
  }

  setVolume(v: number): void {
    this.volume = v;
    if (this.ctx !== undefined) this.master.gain.setTargetAtTime(this.muted ? 0 : v, this.ctx.currentTime, 0.05);
  }

  toggleMute(): boolean {
    this.muted = !this.muted;
    this.setVolume(this.volume);
    return this.muted;
  }

  private ramp(p: AudioParam, v: number, t = 0.04): void {
    p.setTargetAtTime(v, (this.ctx as AudioContext).currentTime, t);
  }

  /** One-shot tone burst. */
  private beep(freq: number, dur: number, gain: number, type: OscillatorType = "sine", pan = 0, when = 0): void {
    const ctx = this.ctx as AudioContext;
    const t = ctx.currentTime + when;
    const o = ctx.createOscillator();
    o.type = type;
    o.frequency.value = freq;
    const g = ctx.createGain();
    g.gain.setValueAtTime(0, t);
    g.gain.linearRampToValueAtTime(gain, t + 0.005);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    const p = ctx.createStereoPanner();
    p.pan.value = pan;
    o.connect(g).connect(p).connect(this.master);
    o.start(t);
    o.stop(t + dur + 0.05);
  }

  private noiseBurst(dur: number, gain: number, freq: number, pan = 0): void {
    const ctx = this.ctx as AudioContext;
    const s = ctx.createBufferSource();
    s.buffer = this.noise;
    const f = ctx.createBiquadFilter();
    f.type = "lowpass";
    f.frequency.value = freq;
    const g = ctx.createGain();
    const t = ctx.currentTime;
    g.gain.setValueAtTime(gain, t);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    const p = ctx.createStereoPanner();
    p.pan.value = pan;
    s.connect(f).connect(g).connect(p).connect(this.master);
    s.start(t, Math.random());
    s.stop(t + dur + 0.05);
  }

  /** Remote horn of another car: pitch varies per car, attenuated by distance. */
  private remoteHorn(v: Vehicle, listener: Vehicle, long: boolean): void {
    const ctx = this.ctx as AudioContext;
    const dx = v.x - listener.x;
    const dy = v.y - listener.y;
    const d = Math.hypot(dx, dy);
    if (d > 160) return;
    const gain = 0.22 / (1 + d / 12);
    const pan = this.panFor(listener, dx, dy);
    const base = 330 + ((v.id * 37) % 9) * 22 + (v.spec.id === "truck" || v.spec.id === "bus" ? -140 : 0);
    const dur = long ? 1.4 : 0.35;
    const t = ctx.currentTime;
    for (const mul of [1, 1.26]) {
      const o = ctx.createOscillator();
      o.type = "square";
      o.frequency.value = base * mul;
      const g = ctx.createGain();
      g.gain.setValueAtTime(0, t);
      g.gain.linearRampToValueAtTime(gain, t + 0.02);
      g.gain.setValueAtTime(gain, t + dur);
      g.gain.linearRampToValueAtTime(0, t + dur + 0.05);
      const f = ctx.createBiquadFilter();
      f.type = "lowpass";
      f.frequency.value = 1800;
      const p = ctx.createStereoPanner();
      p.pan.value = pan;
      o.connect(f).connect(g).connect(p).connect(this.master);
      o.start(t);
      o.stop(t + dur + 0.1);
    }
  }

  private panFor(listener: Vehicle, dx: number, dy: number): number {
    const right = dx * Math.sin(listener.heading) - dy * Math.cos(listener.heading);
    const d = Math.hypot(dx, dy) + 1;
    return Math.max(-1, Math.min(1, right / d));
  }

  onEvent(e: SimEvent, world: World): void {
    if (this.ctx === undefined) return;
    const p = world.player;
    if (p === undefined) return;
    switch (e.type) {
      case "horn": {
        if (e.vehicle === p.id) return;
        const v = world.vehicle(e.vehicle);
        if (v !== undefined) this.remoteHorn(v, p, e.long);
        break;
      }
      case "collision": {
        const d = Math.hypot(e.x - p.x, e.y - p.y);
        if (d > 120) return;
        const g = Math.min(1, e.severity / 6) / (1 + d / 10);
        this.noiseBurst(0.35 + g * 0.4, 0.9 * g, 900);
        this.beep(70, 0.3, 0.6 * g, "sine");
        if (e.severity > 3) this.noiseBurst(0.6, 0.25 * g, 6000);
        break;
      }
      case "crash_static":
        if (e.vehicle === p.id) {
          this.noiseBurst(0.4, Math.min(0.9, e.severity / 8), 700);
          this.beep(60, 0.25, Math.min(0.6, e.severity / 10));
        }
        break;
      case "camera_flash":
        this.beep(2400, 0.04, 0.12, "square");
        this.beep(1800, 0.05, 0.08, "square", 0, 0.06);
        break;
      case "ticket":
      case "violation":
        if (e.type === "violation" && e.witnessed === "none") return;
        this.beep(880, 0.12, 0.12, "triangle");
        this.beep(660, 0.18, 0.12, "triangle", 0, 0.13);
        break;
      case "grind":
        if (e.vehicle === p.id) {
          this.noiseBurst(0.35, 0.35, 2500);
          this.beep(180, 0.3, 0.12, "sawtooth");
        }
        break;
      case "chime":
        if (e.vehicle === p.id) this.chime();
        break;
      case "shift":
        if (e.vehicle === p.id) this.noiseBurst(0.05, 0.12, 3000);
        break;
      case "stall":
        if (e.vehicle === p.id) this.noiseBurst(0.5, 0.3, 300);
        break;
      case "engine":
        if (e.vehicle === p.id && e.on) this.noiseBurst(0.8, 0.12, 400);
        break;
      default:
        break;
    }
  }

  /** Per-frame continuous sounds. */
  update(dt: number, world: World, inCabin: boolean): void {
    if (this.ctx === undefined) return;
    const p = world.player;
    if (p === undefined) return;
    const pt = p.pt;
    const spec = p.spec;
    const rpm = pt.running || pt.cranking > 0 ? pt.rpm : 0;
    // Firing frequency: cylinders/2 pulses per crank revolution.
    const fire = (rpm / 60) * (spec.cylinders / 2);
    this.ramp(this.engOsc1.frequency, Math.max(20, fire), 0.02);
    this.ramp(this.engOsc2.frequency, Math.max(15, fire * 0.5), 0.02);
    const load = pt.load;
    const running = pt.running ? 1 : pt.cranking > 0 ? 0.5 : 0;
    this.ramp(this.engFilter.frequency, 180 + load * 1800 + (rpm / spec.redline) * 900);
    const lim = pt.limiterActive ? 0.4 : 1;
    this.ramp(this.engGain.gain, running * (0.05 + load * 0.11) * lim * (inCabin ? 0.8 : 1));
    this.ramp(this.intakeFilter.frequency, 300 + rpm * 0.35);
    this.ramp(this.intakeGain.gain, running * load * (rpm / spec.redline) * 0.12);
    const speed = p.absSpeed();
    this.ramp(this.skidGain.gain, Math.min(1, p.dyn.skid) * Math.min(1, speed / 5) * 0.22, 0.03);
    this.ramp(this.windGain.gain, Math.min(0.25, (speed / 60) * (speed / 60) * 0.3) * (inCabin ? 0.6 : 1));
    this.ramp(this.rainGain.gain, world.env.rain * (inCabin ? 0.12 : 0.07), 0.3);
    this.ramp(this.hornGain.gain, p.lights.horn ? 0.12 : 0, 0.01);
    // Indicator relay: tick on, tock off.
    const blink = p.lights.blinkOn() && (p.lights.indicator !== "off" || p.lights.hazard);
    if (blink !== this.lastBlink) {
      if (p.lights.indicator !== "off" || p.lights.hazard) this.noiseBurst(0.025, blink ? 0.22 : 0.15, blink ? 5000 : 3000);
      this.lastBlink = blink;
    }
    // Reverse warning beeper on commercial vehicles.
    const beeper = spec.id === "truck" || spec.id === "bus" || spec.id === "van" || spec.id === "ambulance";
    if (beeper && p.lights.reverse && pt.running) {
      this.reverseBeep -= dt;
      if (this.reverseBeep <= 0) {
        this.reverseBeep = 1.0;
        this.beep(1050, 0.45, 0.1, "square");
      }
    } else this.reverseBeep = 0;
    // Nearest siren.
    let best: Vehicle | undefined = undefined;
    let bd = 220;
    for (const v of world.vehicles) {
      if (!v.sirenOn()) continue;
      const d = Math.hypot(v.x - p.x, v.y - p.y);
      if (d < bd) {
        bd = d;
        best = v;
      }
    }
    if (best !== undefined) {
      const yelp = best.lights.siren === "yelp";
      this.sirenPhase += dt * (yelp ? 3.2 : 0.25) * Math.PI * 2;
      const f = yelp ? 900 + Math.sin(this.sirenPhase) * 350 : 950 + Math.sin(this.sirenPhase) * 420;
      // Doppler-ish: approaching raises pitch slightly.
      const rel = ((best.vx() - p.vx()) * (p.x - best.x) + (best.vy() - p.vy()) * (p.y - best.y)) / (bd + 1);
      this.ramp(this.sirenOsc.frequency, f * (1 + rel / 340), 0.02);
      this.ramp(this.sirenGain.gain, best === p ? 0.12 : 0.25 / (1 + bd / 15), 0.05);
      this.ramp(this.sirenPan.pan, best === p ? 0 : this.panFor(p, best.x - p.x, best.y - p.y), 0.05);
    } else this.ramp(this.sirenGain.gain, 0, 0.1);
  }

  chime(): void {
    if (this.ctx === undefined) return;
    this.beep(1320, 0.18, 0.08);
    this.beep(990, 0.25, 0.08, "sine", 0, 0.2);
  }
}
