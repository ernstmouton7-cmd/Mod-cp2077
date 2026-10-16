import * as THREE from "three";
import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer.js";
import { RenderPass } from "three/examples/jsm/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/examples/jsm/postprocessing/UnrealBloomPass.js";
import { OutputPass } from "three/examples/jsm/postprocessing/OutputPass.js";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { World } from "../core/world/world";
import { ActionId } from "../core/player/actions";
import { VehicleClass } from "../core/vehicle/specs";
import { AudioEngine } from "./audio";
import { BrowserInput, setupTouch } from "./browserInput";
import { CAMERA_LABEL, CameraRig } from "./cameraRig";
import { CityView } from "./cityView";
import { Effects, Windshield } from "./effects";
import { Hud } from "./hud";
import { loadSettings, Menu, saveSettings, Settings } from "./menu";
import { SignalView } from "./signalView";
import { PoseHistory, VehicleView } from "./vehicleView";
import { PedView } from "./pedView";

declare global {
  interface Window {
    gridlock?: unknown;
    claude?: { hot?: { ready?: (fn: (data: Record<string, unknown>) => void) => void; data?: Record<string, unknown>; snapshot?: (fn: () => Record<string, unknown>) => void } };
  }
}

const mobile = window.matchMedia("(pointer: coarse)").matches;

function boot(snap: Record<string, unknown>): void {
  const stored = loadSettings();
  const settings: Settings = {
    unit: stored.unit ?? "kmh",
    volume: stored.volume ?? 0.7,
    bloom: stored.bloom ?? !mobile,
    quality: stored.quality ?? (mobile ? "low" : "high"),
    vehicle: (stored.vehicle as VehicleClass | undefined) ?? "sports",
  };
  const world = new World({ seed: 2077, traffic: mobile ? 70 : 110, police: 4, startHour: typeof snap.hour === "number" ? snap.hour : 20.75, playerVehicle: settings.vehicle });
  if (stored.keys !== undefined) for (const id of Object.keys(stored.keys)) world.input.bindKeys(id as ActionId, stored.keys[id]);
  window.gridlock = world;

  // --- Renderer & scene.
  const canvas = document.getElementById("view") as HTMLCanvasElement;
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: settings.quality === "high", powerPreference: "high-performance" });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, settings.quality === "high" ? 1.75 : 1));
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  const scene = new THREE.Scene();
  const night = new THREE.Color(0x0a0c18);
  const dusk = new THREE.Color(0x3a2a4a);
  const day = new THREE.Color(0x9db4cc);
  scene.background = night.clone();
  scene.fog = new THREE.FogExp2(0x0a0c18, 0.0035);
  const hemi = new THREE.HemisphereLight(0x8899cc, 0x221a20, 0.4);
  const sun = new THREE.DirectionalLight(0xffffff, 0.2);
  sun.position.set(200, 300, 100);
  scene.add(hemi, sun, new THREE.AmbientLight(0x404060, 0.25));
  const pmrem = new THREE.PMREMGenerator(renderer);
  const envMap = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  const rig = new CameraRig(window.innerWidth / window.innerHeight);
  const city = new CityView(world);
  const signals = new SignalView(world);
  const vehicles = new VehicleView();
  vehicles.maxDistance = settings.quality === "high" ? 330 : 200;
  vehicles.bodyMat.envMap = envMap;
  city.setEnvMap(envMap);
  const fx = new Effects();
  const peds = new PedView();
  scene.add(city.group, signals.group, vehicles.group, fx.group, peds.group);
  const composer = new EffectComposer(renderer);
  composer.addPass(new RenderPass(scene, rig.camera));
  const bloom = new UnrealBloomPass(new THREE.Vector2(window.innerWidth, window.innerHeight), 0.9, 0.5, 0.82);
  composer.addPass(bloom);
  composer.addPass(new OutputPass());

  const resize = () => {
    const w = window.innerWidth;
    const h = window.innerHeight;
    renderer.setSize(w, h, false);
    composer.setSize(w, h);
    rig.camera.aspect = w / h;
    rig.camera.updateProjectionMatrix();
  };
  window.addEventListener("resize", resize);
  resize();

  // --- HUD, audio, menus, input.
  const hud = new Hud(world);
  hud.unit = settings.unit;
  const audio = new AudioEngine();
  audio.volume = settings.volume;
  const windshield = new Windshield(document.getElementById("glass") as HTMLCanvasElement);
  const menu = new Menu(world, settings);
  const hist = new PoseHistory();
  let started = false;
  let paused = false;
  const title = document.getElementById("title") as HTMLElement;
  const start = () => {
    if (started) return;
    started = true;
    title.hidden = true;
    audio.start();
    audio.setVolume(settings.volume);
  };
  (document.getElementById("startBtn") as HTMLButtonElement).addEventListener("click", start);
  (document.getElementById("controlsBtn") as HTMLButtonElement).addEventListener("click", () => {
    start();
    paused = true;
    menu.show("controls");
  });
  menu.onChange = () => {
    hud.unit = settings.unit;
    audio.setVolume(settings.volume);
    saveSettings(settings, world);
  };
  menu.onVehicle = (cls) => {
    world.setPlayerVehicle(cls);
    hud.toast(`Now driving: ${world.player?.spec.name ?? cls}`, "info");
  };
  const ui = (a: ActionId) => {
    if (!started) {
      if (a === "pause") return;
      start();
    }
    switch (a) {
      case "pause":
        if (menu.open) {
          menu.hide();
          paused = false;
        } else {
          paused = true;
          menu.show("controls");
        }
        break;
      case "help":
        if (menu.open) {
          menu.hide();
          paused = false;
        } else {
          paused = true;
          menu.show("controls");
        }
        break;
      case "map":
        hud.toggleBigMap();
        break;
      case "debug":
        hud.setDebug(!hud.debug);
        break;
      default:
        break;
    }
  };
  (document.getElementById("resume") as HTMLButtonElement).addEventListener("click", () => (paused = false));
  (document.getElementById("menuBtn") as HTMLButtonElement).addEventListener("click", () => ui("pause"));
  const input = new BrowserInput(world, { ui, orbit: (dx, dy) => rig.orbit(dx, dy), gesture: () => audio.ctx !== undefined && void audio.ctx.resume(), rebind: (c) => menu.rebind(c) }, canvas);
  setupTouch(world, { ui, orbit: () => {}, gesture: start });

  // Restore a few things after a live page update.
  if (typeof snap.money === "number") world.record.money = snap.money;
  if (typeof snap.points === "number") world.record.points = snap.points;
  window.claude?.hot?.snapshot?.(() => ({ hour: world.env.hour, money: world.record.money, points: world.record.points }));

  // --- Main loop.
  let last = performance.now();
  let acc = 0;
  const fixed = world.fixedDt;
  const pose = { x: 0, y: 0, h: 0 };
  const camPos = new THREE.Vector3();
  let fpsTime = 0;
  let fpsFrames = 0;
  const fpsEl = document.getElementById("fps") as HTMLElement;
  const frame = (now: number) => {
    requestAnimationFrame(frame);
    // rAF timestamps can predate the end of a slow boot: never let dt go negative.
    const dt = Math.max(0, Math.min(0.1, (now - last) / 1000));
    last = now;
    input.pollGamepad();
    const running = started && !paused;
    if (running) {
      acc += dt;
      let steps = 0;
      while (acc >= fixed && steps < 6) {
        hist.capture(world);
        world.step(fixed);
        acc -= fixed;
        steps++;
      }
      if (steps === 6) acc = 0;
    } else {
      // Attract mode behind the title: traffic keeps flowing, the player waits.
      if (!started) {
        acc += dt;
        while (acc >= fixed) {
          hist.capture(world);
          world.step(fixed);
          acc -= fixed;
        }
      }
    }
    const alpha = running || !started ? acc / fixed : 1;
    // Events.
    for (const e of world.events.drain()) {
      hud.onEvent(e);
      audio.onEvent(e, world);
      if (e.type === "collision") {
        fx.sparksAt(e.x, 0.6, -e.y, Math.min(60, Math.round(e.severity * 6)));
        const p = world.player;
        if (p !== undefined && (e.a === p.id || e.b === p.id)) rig.bump(Math.min(1, e.severity / 8));
      } else if (e.type === "crash_static") {
        fx.sparksAt(e.x, 0.5, -e.y, Math.min(40, Math.round(e.severity * 5)));
        if (e.vehicle === world.player?.id) rig.bump(Math.min(1, e.severity / 8));
      } else if (e.type === "camera_flash") signals.flashAt(e.x, e.y);
    }
    for (const a of world.drainHostActions()) {
      if (a === "camera") hud.toast(`Camera: ${CAMERA_LABEL[rig.cycle()]}`, "info");
      else if (a === "reset_vehicle") hud.toast("Vehicle reset to the nearest road", "info");
      else if (a === "time_forward") hud.toast("Clock +1 hour", "info");
      else if (a === "weather_cycle") hud.toast(`Weather: ${world.env.weather}`, "info");
    }
    const p = world.player;
    if (p !== undefined) {
      hist.pose(p, alpha, pose);
      const inp = world.input;
      rig.look = inp.isHeld("look_back") ? 2 : inp.isHeld("look_left") ? -1 : inp.isHeld("look_right") ? 1 : 0;
      rig.update(dt, p, pose.x, pose.y, pose.h);
    }
    camPos.copy(rig.camera.position);
    // Lighting from time of day and weather.
    const env = world.env;
    const nightK = 1 - env.daylight;
    const duskK = Math.max(0, 1 - Math.abs(env.daylight - 0.45) * 3);
    const sky = night.clone().lerp(day, env.daylight).lerp(dusk, duskK * 0.5);
    sky.lerp(new THREE.Color(0x3c4250), Math.min(1, env.rain * 0.6 + env.fog * 0.5) * env.daylight);
    (scene.background as THREE.Color).copy(sky);
    const fog = scene.fog as THREE.FogExp2;
    fog.color.copy(sky);
    fog.density = 0.0022 + env.fog * 0.016 + env.rain * 0.003 + nightK * 0.001;
    hemi.intensity = 0.25 + env.daylight * 1.0;
    vehicles.bodyMat.envMapIntensity = 0.25 + env.daylight * 0.8;
    sun.intensity = env.daylight * 1.6 * (1 - env.rain * 0.6);
    city.update(dt, { night: nightK, wetness: env.wetness });
    signals.update(dt, world.time);
    vehicles.hidePlayerGlow = rig.onGlass;
    vehicles.update(world, hist, alpha, camPos, nightK);
    fx.update(dt, world, camPos, env.rain);
    peds.update(world);
    const pd = world.playerDriver;
    if (p !== undefined && pd !== undefined) {
      hud.update(dt, rig.camera, pd.gearText(), pd.cruise, pd.limiterOn, pd.seatbelt, pd.notes);
      audio.update(dt, world, rig.onGlass);
      windshield.update(dt, env.rain, p.absSpeed(), p.lights.wiperPos, rig.onGlass);
    }
    bloom.strength = 0.55 + nightK * 0.45;
    if (settings.bloom) composer.render();
    else renderer.render(scene, rig.camera);
    fpsFrames++;
    fpsTime += dt;
    if (fpsTime > 1) {
      fpsEl.textContent = `${Math.round(fpsFrames / fpsTime)} fps · ${world.vehicles.length} vehicles${input.padName !== "" ? " · gamepad" : ""}`;
      fpsFrames = 0;
      fpsTime = 0;
    }
  };
  requestAnimationFrame(frame);
}

const hot = window.claude?.hot;
if (hot?.ready !== undefined) hot.ready(boot);
else boot(hot?.data ?? {});
