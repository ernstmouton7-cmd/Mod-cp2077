//! Gridlock: drive a simulated city with full traffic, physics and crash damage.
//!
//! ```text
//! gridlock                       # windowed, gamepad or keyboard
//! gridlock --quality ultra --scale 1.5
//! gridlock --shot out.jpg --hour 21 --seconds 8 --width 1280 --height 720 [--rain]   # headless capture
//! ```

mod assets;
mod cam;
mod hud;
mod view;

use anyhow::Result;
use cam::{CamMode, CameraRig};
use glam::Vec2;
use gridlock_game::input::{
    device::GamepadManager, Action, ControlFrame, Device, GamepadState, KeyboardState, PadKind,
};
use gridlock_game::{Game, GameConfig};
use gridlock_render::{wgpu, Quality, RenderSettings, Renderer};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

#[derive(Clone, Debug)]
struct Args {
    shot: Option<String>,
    hour: Option<f32>,
    seconds: f32,
    width: u32,
    height: u32,
    quality: Quality,
    scale: f32,
    seed: i64,
    traffic: usize,
    rain: bool,
    cam: CamMode,
    throttle: f32,
}

fn parse_args() -> Args {
    let mut a = Args {
        shot: None,
        hour: None,
        seconds: 6.0,
        width: 1600,
        height: 900,
        quality: Quality::High,
        scale: 1.0,
        seed: 2077,
        traffic: 70,
        rain: false,
        cam: CamMode::Chase,
        throttle: 0.35,
    };
    let v: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < v.len() {
        let next = |i: usize| v.get(i + 1).cloned().unwrap_or_default();
        match v[i].as_str() {
            "--shot" => a.shot = Some(next(i)),
            "--hour" => a.hour = next(i).parse().ok(),
            "--seconds" => a.seconds = next(i).parse().unwrap_or(a.seconds),
            "--width" => a.width = next(i).parse().unwrap_or(a.width),
            "--height" => a.height = next(i).parse().unwrap_or(a.height),
            "--scale" => a.scale = next(i).parse().unwrap_or(1.0),
            "--seed" => a.seed = next(i).parse().unwrap_or(a.seed),
            "--traffic" => a.traffic = next(i).parse().unwrap_or(a.traffic),
            "--throttle" => a.throttle = next(i).parse().unwrap_or(a.throttle),
            "--quality" => {
                a.quality = match next(i).to_ascii_lowercase().as_str() {
                    "low" => Quality::Low,
                    "medium" => Quality::Medium,
                    "ultra" => Quality::Ultra,
                    _ => Quality::High,
                }
            }
            "--cam" => {
                a.cam = match next(i).as_str() {
                    "far" => CamMode::FarChase,
                    "hood" => CamMode::Hood,
                    "cockpit" => CamMode::Cockpit,
                    "side" => CamMode::Side,
                    _ => CamMode::Chase,
                }
            }
            "--rain" => {
                a.rain = true;
                i -= 1;
            }
            _ => {
                i -= 1;
            }
        }
        i += 2;
    }
    a
}

fn settings(q: Quality, scale: f32) -> RenderSettings {
    let mut s = RenderSettings::preset(q);
    s.render_scale = scale;
    s
}

/// Everything except the window/GPU surface.
struct Session {
    game: Game,
    view: view::WorldView,
    rig: CameraRig,
    hud: hud::Hud,
    st: hud::Settings,
    last: Instant,
    sim_ms: f32,
}

impl Session {
    fn new(r: &mut Renderer, args: &Args) -> Self {
        let mut game = Game::new(GameConfig {
            seed: args.seed,
            traffic: args.traffic,
            ..Default::default()
        });
        if let Some(h) = args.hour {
            game.traffic.env_mut().hour = h as f64;
        }
        if args.rain {
            let env = game.traffic.env_mut();
            env.weather = gridlock_game::traffic::Weather::Rain;
            env.rain = 0.8;
            env.wetness = 1.0;
            env.auto_weather = false;
        }
        let mut a = assets::Assets::new(2048);
        let view = view::WorldView::new(r, &mut a, &mut game);
        let mut rig = CameraRig::default();
        rig.mode = args.cam;
        let st = hud::Settings {
            hour: game.traffic.env().hour as f32,
            ..Default::default()
        };
        Self {
            game,
            view,
            rig,
            hud: hud::Hud::default(),
            st,
            last: Instant::now(),
            sim_ms: 0.0,
        }
    }

    /// Simulate and update renderer state. Returns the camera to render with.
    fn frame(
        &mut self,
        r: &mut Renderer,
        dt: f32,
        pad: &GamepadState,
        kb: &mut KeyboardState,
    ) -> gridlock_render::Camera {
        let t0 = Instant::now();
        // Settings -> game.
        let cfg = &mut self.game.input.config;
        cfg.steering.deadzone = self.st.steer_deadzone;
        cfg.steering.exponent = self.st.steer_exponent;
        cfg.steering.sensitivity = self.st.steer_sensitivity;
        self.game.rumble.strength = self.st.rumble;
        if let Some(vh) = self.game.player_vehicle() {
            if let Some(v) = self.game.physics.vehicle_mut(vh) {
                v.assists.abs = self.st.abs;
                v.assists.tcs = self.st.tcs;
                v.assists.esc = self.st.esc;
                v.assists.steering = self.st.steer_assist;
                v.assists.auto_clutch = self.st.auto_clutch;
            }
        }
        if self.st.hour_changed {
            self.game.traffic.env_mut().hour = self.st.hour as f64;
            self.st.hour_changed = false;
        } else {
            self.st.hour = self.game.traffic.env().hour as f32;
        }
        let (pad_in, mut kb_none) = (GamepadState::default(), KeyboardState::default());
        let paused = self.hud.menu_open;
        if paused {
            self.game.update(0.0, &pad_in, &mut kb_none);
        } else {
            self.game.update(dt as f64, pad, kb);
        }
        let frame: ControlFrame = self.game.last_frame.clone();
        if frame.pressed(Action::Pause) {
            self.hud.menu_open = !self.hud.menu_open;
        }
        if frame.pressed(Action::Camera) {
            self.rig.mode = self.rig.mode.next();
            self.rig.reset();
        }
        self.rig.look_back = frame.is_held(Action::LookBack);
        self.sim_ms = t0.elapsed().as_secs_f32() * 1000.0;
        let player = self.game.player_id().map(|p| p.0);
        self.hud.on_events(&self.game.frame_events, player);

        let scene = self.game.scene();
        let pose = scene.player_pose.unwrap_or_default();
        let speed = self.game.hud().speed_kmh / 3.6;
        let length = scene
            .cars
            .iter()
            .find(|c| c.player)
            .map_or(4.6, |c| c.class.spec().length);
        self.rig.fov = self.st.fov;
        let camera = self.rig.update(
            dt,
            &pose,
            speed,
            Vec2::new(frame.look.x, frame.look.y),
            length,
        );
        self.hud.camera_label = self.rig.mode.label();
        let env = cam::environment(&scene.env, self.game.time as f32);
        self.view.night = (1.0 - scene.env.daylight * 1.4).clamp(0.0, 1.0);
        r.set_environment(&env);
        let (lights, particles) = self.view.sync(r, &mut self.game, &scene, camera.position);
        r.set_lights(&lights);
        r.set_particles(&particles);
        camera
    }
}

fn headless(args: &Args) -> Result<()> {
    let mut r =
        Renderer::new_headless(args.width, args.height, settings(args.quality, args.scale))?;
    let mut s = Session::new(&mut r, args);
    let pad = GamepadState {
        connected: true,
        axes: [0.0, 0.0, 0.0, 0.0, 0.0, args.throttle],
        ..Default::default()
    };
    let mut kb = KeyboardState::default();
    let steps = (args.seconds * 60.0) as usize;
    let t0 = Instant::now();
    let mut cam = None;
    for _ in 0..steps {
        cam = Some(s.frame(&mut r, 1.0 / 60.0, &pad, &mut kb));
    }
    println!(
        "simulated {:.1}s in {:.2}s",
        args.seconds,
        t0.elapsed().as_secs_f32()
    );
    let path = args.shot.clone().unwrap();
    r.capture_png(&cam.unwrap(), std::path::Path::new(&path))?;
    println!("wrote {path} ({:?})", r.stats().render_width);
    Ok(())
}

struct Gfx {
    window: Arc<Window>,
    renderer: Renderer,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

struct App {
    args: Args,
    gfx: Option<Gfx>,
    session: Option<Session>,
    pads: GamepadManager,
    kb: KeyboardState,
    frame_times: Vec<f32>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Gridlock")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.args.width,
                self.args.height,
            ));
        let window = Arc::new(el.create_window(attrs).expect("window"));
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(
                Box::new(el.owned_display_handle()),
            ));
        let surface = instance.create_surface(window.clone()).expect("surface");
        let size = window.inner_size();
        let mut renderer = Renderer::new_windowed(
            &instance,
            surface,
            size.width,
            size.height,
            settings(self.args.quality, self.args.scale),
        )
        .expect("renderer");
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            renderer.device(),
            renderer.surface_format(),
            egui_wgpu::RendererOptions::default(),
        );
        window.set_title(&format!("Gridlock — {}", renderer.adapter_info().name));
        let session = Session::new(&mut renderer, &self.args);
        self.session = Some(session);
        self.gfx = Some(Gfx {
            window,
            renderer,
            egui_ctx,
            egui_state,
            egui_renderer,
        });
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(g) = self.gfx.as_mut() else { return };
        let ui_wants = g.egui_state.on_window_event(&g.window, &event).consumed;
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => g.renderer.resize(s.width.max(1), s.height.max(1)),
            WindowEvent::KeyboardInput { event, .. } if !ui_wants => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let name = format!("{code:?}");
                    match event.state {
                        ElementState::Pressed => self.kb.key_down(&name),
                        ElementState::Released => self.kb.key_up(&name),
                    }
                }
            }
            WindowEvent::Focused(false) => self.kb = KeyboardState::default(),
            WindowEvent::RedrawRequested => {
                let Some(s) = self.session.as_mut() else {
                    return;
                };
                let now = Instant::now();
                let dt = (now - s.last).as_secs_f32().min(0.1);
                s.last = now;
                self.frame_times.push(dt);
                if self.frame_times.len() > 60 {
                    self.frame_times.remove(0);
                }
                let avg = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
                s.hud.fps = 1.0 / avg.max(1e-4);
                s.hud.frame_ms = avg * 1000.0;
                let pad = self.pads.poll().clone();
                let camera = s.frame(&mut g.renderer, dt, &pad, &mut self.kb);
                self.pads.set_rumble(if s.hud.menu_open {
                    Default::default()
                } else {
                    s.game.last_rumble
                });
                // HUD.
                let raw = g.egui_state.take_egui_input(&g.window);
                let hud_info = s.game.hud();
                let device = s.game.input.device;
                let kind = if pad.connected {
                    pad.kind
                } else {
                    PadKind::Xbox
                };
                let out = g.egui_ctx.run_pass(raw, |ctx| {
                    s.hud.draw(ctx, dt, &hud_info, device, kind, &mut s.st)
                });
                g.egui_state
                    .handle_platform_output(&g.window, out.platform_output);
                let ppp = g.egui_ctx.pixels_per_point();
                let jobs = g.egui_ctx.tessellate(out.shapes, ppp);
                let (w, h) = g.renderer.output_size();
                let desc = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [w, h],
                    pixels_per_point: ppp,
                };
                let er = &mut g.egui_renderer;
                let textures = out.textures_delta;
                let mut overlay = |device: &wgpu::Device,
                                   queue: &wgpu::Queue,
                                   enc: &mut wgpu::CommandEncoder,
                                   view: &wgpu::TextureView| {
                    for (id, deltas) in &textures.set {
                        for delta in deltas.iter() {
                            er.update_texture(device, queue, *id, delta);
                        }
                    }
                    let extra = er.update_buffers(device, queue, enc, &jobs, &desc);
                    if !extra.is_empty() {
                        queue.submit(extra);
                    }
                    let mut pass = enc
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("egui"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view,
                                resolve_target: None,
                                depth_slice: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        })
                        .forget_lifetime();
                    er.render(&mut pass, &jobs, &desc);
                };
                if let Err(e) = g.renderer.render(&camera, Some(&mut overlay)) {
                    eprintln!("render error: {e}");
                }
                for id in &textures.free {
                    g.egui_renderer.free_texture(id);
                }
                if s.st.quality_changed {
                    g.renderer
                        .set_settings(settings(s.st.quality, s.st.render_scale));
                    s.st.quality_changed = false;
                }
                if s.st.quit {
                    el.exit();
                }
                let _ = Device::Gamepad;
                g.window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(g) = &self.gfx {
            g.window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    let args = parse_args();
    if args.shot.is_some() {
        return headless(&args);
    }
    let el = EventLoop::new()?;
    el.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        args,
        gfx: None,
        session: None,
        pads: GamepadManager::new(),
        kb: KeyboardState::default(),
        frame_times: Vec::new(),
    };
    el.run_app(&mut app)?;
    Ok(())
}
