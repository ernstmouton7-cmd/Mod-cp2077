//! C ABI for the Gridlock game layer, so engines like Unreal Engine 5 can host the simulation.
//!
//! All functions are `extern "C"`, take an opaque `GlGame*`, and copy plain structs out. Arrays
//! are filled up to a caller-supplied capacity and the total count is returned (call with
//! `cap = 0` to size a buffer). Pointers returned inside `GlMesh` stay valid until the next
//! call that returns a mesh or until `gl_game_update`.
//!
//! With `GlConfig::coords = GL_COORDS_UNREAL` every position is converted to Unreal units and
//! axes (centimetres, left-handed, Z up: X = east, Y = south, Z = up), rotations accordingly,
//! and mesh triangle winding is flipped to stay front-facing. See `include/gridlock.h`.

#![allow(clippy::missing_safety_doc)]

use gridlock_game::input::{GamepadState, KeyboardState, PadAxis, PadButton};
use gridlock_game::physics::glam::{Quat, Vec3};
use gridlock_game::physics::vehicle::presets::VehicleClass as PClass;
use gridlock_game::physics::Pose;
use gridlock_game::traffic::{Event, Lamp, PedLamp, VehicleClass as TClass, VehicleId};
use gridlock_game::{Game, GameConfig, GameEvent, MarkKind, MeshData};
use std::ffi::{c_char, CStr};

pub const GL_COORDS_NATIVE: u32 = 0;
pub const GL_COORDS_UNREAL: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GlConfig {
    pub seed: i64,
    pub traffic: u32,
    pub police: u32,
    pub start_hour: f32,
    /// Vehicle class index (0 compact .. 10 ambulance).
    pub player_class: u32,
    pub coords: u32,
    /// Non-zero: spawn street furniture.
    pub props: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlPad {
    /// left x, left y (up +), right x, right y, left trigger, right trigger.
    pub axes: [f32; 6],
    /// Bit i = button i (A, B, X, Y, LB, RB, View, Menu, LS, RS, Up, Down, Left, Right, Guide).
    pub buttons: u32,
    pub connected: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlPose {
    pub pos: [f32; 3],
    /// Quaternion x, y, z, w.
    pub rot: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlCarView {
    pub id: u32,
    pub class_index: u32,
    pub color: u32,
    /// bit0 player, bit1 police, bit2 physics, bit3 horn, bit4 siren, bits 8..11 wheel detached.
    pub flags: u32,
    pub pose: GlPose,
    pub wheels: [GlPose; 4],
    pub speed: f32,
    pub rpm: f32,
    pub brake: f32,
    /// bit0 low beam, bit1 high beam, bit2 reverse, bit3 left ind., bit4 right ind., bit5 light bar,
    /// bits 8..15 light-bar phase, bit16 left head broken, bit17 right head broken.
    pub lamps: u32,
    pub body_revision: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlSignalView {
    pub index: u32,
    /// 0 red, 1 yellow, 2 green, 3 off.
    pub ball: u8,
    /// Same values, 255 = no arrow head.
    pub arrow: u8,
    /// 0 don't walk, 1 flashing don't walk, 2 walk, 3 off.
    pub ped: u8,
    /// bit0 ball flashing.
    pub flags: u8,
    pub ped_countdown: f32,
    pub pole: [f32; 3],
    pub head: [f32; 3],
    pub facing: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlPedView {
    pub pos: [f32; 3],
    pub heading: f32,
    pub stride: f32,
    pub height: f32,
    pub shirt: u32,
    /// bit0 walking, bit1 down.
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlPropView {
    pub kind: u32,
    pub broken: u32,
    pub pose: GlPose,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlEnv {
    pub hour: f32,
    pub daylight: f32,
    pub rain: f32,
    pub fog: f32,
    pub wetness: f32,
    pub sun_dir: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlRumble {
    pub low: f32,
    pub high: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GlHud {
    pub speed_kmh: f32,
    pub rpm: f32,
    pub redline: f32,
    pub limit_kmh: f32,
    pub money: f32,
    pub heat: f32,
    pub engine_health: f32,
    pub hour: f32,
    /// < 0 when cruise is off.
    pub cruise_kmh: f32,
    pub points: u32,
    pub max_points: u32,
    /// bit0 left ind, bit1 right ind, bit2 hazards, bit3 high beam, bit4 seatbelt, bit5 limiter,
    /// bit6 ABS active, bit7 TCS active, bit8 ESC active, bit9 engine running.
    pub flags: u32,
    /// 0 off, 1 auto, 2 on.
    pub headlights: u32,
    pub gear: [c_char; 8],
    pub street: [c_char; 64],
}

impl Default for GlHud {
    fn default() -> Self {
        Self {
            speed_kmh: 0.0,
            rpm: 0.0,
            redline: 0.0,
            limit_kmh: 0.0,
            money: 0.0,
            heat: 0.0,
            engine_health: 1.0,
            hour: 0.0,
            cruise_kmh: -1.0,
            points: 0,
            max_points: 0,
            flags: 0,
            headlights: 0,
            gear: [0; 8],
            street: [0; 64],
        }
    }
}

pub const GL_EVENT_CRASH: u32 = 1;
pub const GL_EVENT_HORN: u32 = 2;
pub const GL_EVENT_SPEECH: u32 = 3;
pub const GL_EVENT_VIOLATION: u32 = 4;
pub const GL_EVENT_RAGE_START: u32 = 5;
pub const GL_EVENT_PART_DETACHED: u32 = 6;
pub const GL_EVENT_GLASS: u32 = 7;
pub const GL_EVENT_PROP_SNAPPED: u32 = 8;
pub const GL_EVENT_HYDRANT: u32 = 9;
pub const GL_EVENT_SHATTER: u32 = 10;
pub const GL_EVENT_OTHER: u32 = 99;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GlEvent {
    pub kind: u32,
    pub a: u32,
    pub b: u32,
    pub pos: [f32; 3],
    pub value: f32,
    pub text: [c_char; 96],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GlMesh {
    pub positions: *const f32,
    pub normals: *const f32,
    pub uvs: *const f32,
    pub indices: *const u32,
    pub vertex_count: u32,
    pub index_count: u32,
    /// 0 body paint, 1 trim, 2 glass, 3 head lamp, 4 tail lamp, 5 tyre, 6 rim.
    pub paint: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlBuilding {
    pub base: [f32; 3],
    pub size: [f32; 3],
    /// 0 tower, 1 block, 2 garage, 3 park, 4 plaza.
    pub style: u32,
    pub hue: f32,
    pub seed: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GlBlock {
    pub point_count: u32,
    pub outer: u32,
    pub height: f32,
    /// Outline (x, y, z) triplets, up to 32 points.
    pub outline: [[f32; 3]; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlMarking {
    /// 0 white solid, 1 white dashed, 2 yellow solid, 3 stop line, 4 crosswalk.
    pub kind: u32,
    pub point_count: u32,
    pub width: f32,
}

pub struct GlGame {
    game: Game,
    unreal: bool,
    kb: KeyboardState,
    mesh_pos: Vec<f32>,
    mesh_nrm: Vec<f32>,
    mesh_uv: Vec<f32>,
    mesh_idx: Vec<u32>,
    pending_events: Vec<GlEvent>,
}

impl GlGame {
    #[inline]
    fn p(&self, v: Vec3) -> [f32; 3] {
        if self.unreal {
            [v.x * 100.0, v.z * 100.0, v.y * 100.0]
        } else {
            [v.x, v.y, v.z]
        }
    }
    #[inline]
    fn d(&self, v: Vec3) -> [f32; 3] {
        if self.unreal {
            [v.x, v.z, v.y]
        } else {
            [v.x, v.y, v.z]
        }
    }
    #[inline]
    fn q(&self, q: Quat) -> [f32; 4] {
        if self.unreal {
            // Swapping Y and Z mirrors handedness: imaginary part (x, z, y) and w negated.
            [q.x, q.z, q.y, -q.w]
        } else {
            [q.x, q.y, q.z, q.w]
        }
    }
    fn pose(&self, p: &Pose) -> GlPose {
        GlPose {
            pos: self.p(p.p),
            rot: self.q(p.q),
        }
    }
    fn export_mesh(
        &mut self,
        m: &MeshData,
        positions: Option<&[Vec3]>,
        normals: Option<&[Vec3]>,
        paint: u32,
        out: &mut GlMesh,
    ) {
        let pos = positions.unwrap_or(&m.positions);
        let nrm = normals.unwrap_or(&m.normals);
        self.mesh_pos.clear();
        self.mesh_nrm.clear();
        self.mesh_uv.clear();
        self.mesh_idx.clear();
        for v in pos {
            let p = self.p(*v);
            self.mesh_pos.extend_from_slice(&p);
        }
        for n in nrm {
            let d = self.d(*n);
            self.mesh_nrm.extend_from_slice(&d);
        }
        for uv in &m.uvs {
            self.mesh_uv.extend_from_slice(&[uv.x, uv.y]);
        }
        for t in m.indices.chunks_exact(3) {
            if self.unreal {
                self.mesh_idx.extend_from_slice(&[t[0], t[2], t[1]]);
            } else {
                self.mesh_idx.extend_from_slice(t);
            }
        }
        *out = GlMesh {
            positions: self.mesh_pos.as_ptr(),
            normals: self.mesh_nrm.as_ptr(),
            uvs: self.mesh_uv.as_ptr(),
            indices: self.mesh_idx.as_ptr(),
            vertex_count: pos.len() as u32,
            index_count: self.mesh_idx.len() as u32,
            paint,
        };
    }
}

fn write_str<const N: usize>(dst: &mut [c_char; N], s: &str) {
    let bytes = s.as_bytes();
    let n = bytes.len().min(N - 1);
    for (i, b) in bytes[..n].iter().enumerate() {
        dst[i] = *b as c_char;
    }
    dst[n] = 0;
}

fn class_of(i: u32) -> PClass {
    PClass::ALL[(i as usize).min(PClass::ALL.len() - 1)]
}

/// Create a game. Returns null on invalid input.
#[no_mangle]
pub unsafe extern "C" fn gl_game_create(cfg: *const GlConfig) -> *mut GlGame {
    let cfg = if cfg.is_null() {
        GlConfig {
            seed: 2077,
            traffic: 70,
            police: 2,
            start_hour: 17.5,
            player_class: 1,
            coords: 0,
            props: 1,
        }
    } else {
        *cfg
    };
    let tclasses = [
        TClass::Compact,
        TClass::Sedan,
        TClass::Sports,
        TClass::Muscle,
        TClass::Suv,
        TClass::Van,
        TClass::Truck,
        TClass::Bus,
        TClass::Taxi,
        TClass::Police,
        TClass::Ambulance,
    ];
    let config = GameConfig {
        seed: cfg.seed,
        traffic: cfg.traffic as usize,
        police: cfg.police as usize,
        start_hour: cfg.start_hour as f64,
        player_class: tclasses[(cfg.player_class as usize).min(tclasses.len() - 1)],
        props: cfg.props != 0,
    };
    let game = Game::new(config);
    Box::into_raw(Box::new(GlGame {
        game,
        unreal: cfg.coords == GL_COORDS_UNREAL,
        kb: KeyboardState::default(),
        mesh_pos: Vec::new(),
        mesh_nrm: Vec::new(),
        mesh_uv: Vec::new(),
        mesh_idx: Vec::new(),
        pending_events: Vec::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn gl_game_destroy(g: *mut GlGame) {
    if !g.is_null() {
        drop(Box::from_raw(g));
    }
}

/// Advance by real time. `keys` is an optional array of `KeyboardEvent.code` strings held down.
/// Returns the number of fixed steps taken.
#[no_mangle]
pub unsafe extern "C" fn gl_game_update(
    g: *mut GlGame,
    real_dt: f64,
    pad: *const GlPad,
    keys: *const *const c_char,
    key_count: u32,
) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let mut ps = GamepadState::default();
    if let Some(p) = pad.as_ref() {
        ps.connected = p.connected != 0;
        for (i, b) in PadButton::ALL.iter().enumerate() {
            ps.set_button(*b, p.buttons & (1 << i) != 0);
        }
        for (i, a) in [
            PadAxis::LeftX,
            PadAxis::LeftY,
            PadAxis::RightX,
            PadAxis::RightY,
            PadAxis::LeftTrigger,
            PadAxis::RightTrigger,
        ]
        .iter()
        .enumerate()
        {
            ps.set_axis(*a, p.axes[i]);
        }
    }
    let mut down = Vec::new();
    if !keys.is_null() {
        for i in 0..key_count as usize {
            let k = *keys.add(i);
            if !k.is_null() {
                if let Ok(s) = CStr::from_ptr(k).to_str() {
                    down.push(s.to_string());
                }
            }
        }
    }
    let prev: Vec<String> = g.kb.down.iter().cloned().collect();
    for k in &prev {
        if !down.contains(k) {
            g.kb.key_up(k);
        }
    }
    for k in &down {
        g.kb.key_down(k);
    }
    let steps = g.game.update(real_dt, &ps, &mut g.kb);
    // Collect events from this update for gl_game_events.
    let events: Vec<GameEvent> = g.game.events.clone();
    for e in events {
        if let Some(ev) = convert_event(g, &e) {
            if g.pending_events.len() < 512 {
                g.pending_events.push(ev);
            }
        }
    }
    steps as u32
}

fn ev(kind: u32, a: u32, b: u32, pos: [f32; 3], value: f32, text: &str) -> GlEvent {
    let mut e = GlEvent {
        kind,
        a,
        b,
        pos,
        value,
        text: [0; 96],
    };
    write_str(&mut e.text, text);
    e
}

fn convert_event(g: &GlGame, e: &GameEvent) -> Option<GlEvent> {
    let none = u32::MAX;
    let id = |v: Option<VehicleId>| v.map_or(none, |v| v.0);
    Some(match e {
        GameEvent::Crash {
            a,
            b,
            point,
            impulse,
            speed,
        } => ev(
            GL_EVENT_CRASH,
            id(*a),
            id(*b),
            g.p(*point),
            *impulse,
            &format!("{speed:.1}"),
        ),
        GameEvent::Traffic(Event::Horn { vehicle, long }) => ev(
            GL_EVENT_HORN,
            vehicle.0,
            none,
            [0.0; 3],
            if *long { 1.0 } else { 0.0 },
            "",
        ),
        GameEvent::Traffic(Event::Speech { vehicle, text, .. }) => {
            ev(GL_EVENT_SPEECH, vehicle.0, none, [0.0; 3], 0.0, text)
        }
        GameEvent::Traffic(Event::Violation { vehicle, label, .. }) => {
            ev(GL_EVENT_VIOLATION, vehicle.0, none, [0.0; 3], 0.0, label)
        }
        GameEvent::Traffic(Event::RageStart {
            vehicle, target, ..
        }) => ev(GL_EVENT_RAGE_START, vehicle.0, target.0, [0.0; 3], 0.0, ""),
        GameEvent::Damage { vehicle, event } => match event {
            gridlock_game::physics::deform::DamageEvent::PartDetached { part, .. } => ev(
                GL_EVENT_PART_DETACHED,
                vehicle.0,
                *part as u32,
                [0.0; 3],
                0.0,
                "",
            ),
            gridlock_game::physics::deform::DamageEvent::Shattered { part, point } => ev(
                GL_EVENT_GLASS,
                vehicle.0,
                *part as u32,
                g.p(*point),
                0.0,
                "",
            ),
            _ => return None,
        },
        GameEvent::Prop(gridlock_game::physics::props::PropEvent::Snapped { prop, position }) => {
            ev(
                GL_EVENT_PROP_SNAPPED,
                *prop as u32,
                none,
                g.p(*position),
                0.0,
                "",
            )
        }
        GameEvent::Prop(gridlock_game::physics::props::PropEvent::HydrantBurst {
            prop,
            position,
        }) => ev(
            GL_EVENT_HYDRANT,
            *prop as u32,
            none,
            g.p(*position),
            0.0,
            "",
        ),
        GameEvent::Shattered { point, pieces } => ev(
            GL_EVENT_SHATTER,
            none,
            none,
            g.p(*point),
            *pieces as f32,
            "",
        ),
        _ => return None,
    })
}

/// Drain events gathered since the last call (up to `cap`). Returns how many were written.
#[no_mangle]
pub unsafe extern "C" fn gl_game_events(g: *mut GlGame, out: *mut GlEvent, cap: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let n = (cap as usize).min(g.pending_events.len());
    if !out.is_null() {
        for (i, e) in g.pending_events.drain(..n).enumerate() {
            *out.add(i) = e;
        }
    }
    n as u32
}

/// Press an action by its snake-case id (e.g. "indicator_left"), as from a UI button.
#[no_mangle]
pub unsafe extern "C" fn gl_game_action(g: *mut GlGame, action: *const c_char) -> u32 {
    let (Some(g), false) = (g.as_mut(), action.is_null()) else {
        return 0;
    };
    let Ok(name) = CStr::from_ptr(action).to_str() else {
        return 0;
    };
    for a in gridlock_game::traffic::ActionId::ALL {
        if gridlock_game::traffic::action_def(*a).name == name {
            g.game.traffic.input.press(*a);
            return 1;
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn gl_game_cars(g: *mut GlGame, out: *mut GlCarView, cap: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let scene = g.game.scene();
    if !out.is_null() {
        for (i, c) in scene.cars.iter().take(cap as usize).enumerate() {
            let mut flags = (c.player as u32)
                | (c.police as u32) << 1
                | (c.physics as u32) << 2
                | (c.horn as u32) << 3
                | (c.siren as u32) << 4;
            for (k, d) in c.wheel_detached.iter().enumerate().take(4) {
                flags |= (*d as u32) << (8 + k);
            }
            let l = &c.lamps;
            let lamps = (l.low_beam as u32)
                | (l.high_beam as u32) << 1
                | (l.reverse as u32) << 2
                | (l.left_indicator as u32) << 3
                | (l.right_indicator as u32) << 4
                | (l.lightbar.is_some() as u32) << 5
                | (l.lightbar.unwrap_or(0) as u32) << 8
                | (l.left_head_broken as u32) << 16
                | (l.right_head_broken as u32) << 17;
            let mut wheels = [GlPose::default(); 4];
            for (k, w) in c.wheels.iter().enumerate().take(4) {
                wheels[k] = g.pose(w);
            }
            *out.add(i) = GlCarView {
                id: c.id,
                class_index: PClass::ALL.iter().position(|x| *x == c.class).unwrap_or(1) as u32,
                color: c.color,
                flags,
                pose: g.pose(&c.pose),
                wheels,
                speed: c.speed,
                rpm: c.rpm,
                brake: l.brake,
                lamps,
                body_revision: c.body_revision,
            };
        }
    }
    scene.cars.len() as u32
}

fn lamp_code(l: Lamp) -> u8 {
    match l {
        Lamp::Red => 0,
        Lamp::Yellow => 1,
        Lamp::Green => 2,
        Lamp::Off => 3,
    }
}

#[no_mangle]
pub unsafe extern "C" fn gl_game_signals(g: *mut GlGame, out: *mut GlSignalView, cap: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let scene = g.game.scene();
    if !out.is_null() {
        for (i, s) in scene.signals.iter().take(cap as usize).enumerate() {
            let geom = g.game.city.signal_heads[s.index];
            let ped = match s.head.ped {
                PedLamp::Dont => 0,
                PedLamp::Flash => 1,
                PedLamp::Walk => 2,
                PedLamp::Off => 3,
            };
            *out.add(i) = GlSignalView {
                index: s.index as u32,
                ball: lamp_code(s.head.ball),
                arrow: s.head.arrow.map_or(255, lamp_code),
                ped,
                flags: s.head.ball_flashing as u8,
                ped_countdown: s.head.ped_countdown as f32,
                pole: g.p(geom.pole),
                head: g.p(geom.head),
                facing: g.d(geom.facing),
            };
        }
    }
    scene.signals.len() as u32
}

#[no_mangle]
pub unsafe extern "C" fn gl_game_peds(g: *mut GlGame, out: *mut GlPedView, cap: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let scene = g.game.scene();
    if !out.is_null() {
        for (i, p) in scene.peds.iter().take(cap as usize).enumerate() {
            *out.add(i) = GlPedView {
                pos: g.p(p.pos),
                heading: p.heading,
                stride: p.stride,
                height: p.height,
                shirt: p.shirt,
                flags: (p.walking as u32) | (p.down as u32) << 1,
            };
        }
    }
    scene.peds.len() as u32
}

#[no_mangle]
pub unsafe extern "C" fn gl_game_props(g: *mut GlGame, out: *mut GlPropView, cap: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let scene = g.game.scene();
    if !out.is_null() {
        for (i, p) in scene.props.iter().take(cap as usize).enumerate() {
            *out.add(i) = GlPropView {
                kind: p.kind as u32,
                broken: p.broken as u32,
                pose: g.pose(&p.pose),
            };
        }
    }
    scene.props.len() as u32
}

#[no_mangle]
pub unsafe extern "C" fn gl_game_env(g: *mut GlGame, out: *mut GlEnv) {
    let (Some(g), Some(out)) = (g.as_mut(), out.as_mut()) else {
        return;
    };
    let e = g.game.scene().env;
    *out = GlEnv {
        hour: e.hour,
        daylight: e.daylight,
        rain: e.rain,
        fog: e.fog,
        wetness: e.wetness,
        sun_dir: g.d(e.sun_dir),
    };
}

#[no_mangle]
pub unsafe extern "C" fn gl_game_rumble(g: *const GlGame, out: *mut GlRumble) {
    let (Some(g), Some(out)) = (g.as_ref(), out.as_mut()) else {
        return;
    };
    let r = g.game.last_rumble;
    *out = GlRumble {
        low: r.low,
        high: r.high,
        left_trigger: r.left_trigger,
        right_trigger: r.right_trigger,
    };
}

#[no_mangle]
pub unsafe extern "C" fn gl_game_hud(g: *const GlGame, out: *mut GlHud) {
    let (Some(g), Some(out)) = (g.as_ref(), out.as_mut()) else {
        return;
    };
    let h = g.game.hud();
    let mut o = GlHud {
        speed_kmh: h.speed_kmh,
        rpm: h.rpm,
        redline: h.redline,
        limit_kmh: h.limit_kmh,
        money: h.money,
        heat: h.heat,
        engine_health: h.engine_health,
        hour: h.hour,
        cruise_kmh: h.cruise_kmh.unwrap_or(-1.0),
        points: h.points,
        max_points: h.max_points,
        flags: (h.left_indicator as u32)
            | (h.right_indicator as u32) << 1
            | (h.hazards as u32) << 2
            | (h.high_beam as u32) << 3
            | (h.seatbelt as u32) << 4
            | (h.limiter as u32) << 5
            | (h.abs_active as u32) << 6
            | (h.tcs_active as u32) << 7
            | (h.esc_active as u32) << 8
            | (h.engine_running as u32) << 9,
        headlights: h.headlights as u32,
        ..Default::default()
    };
    write_str(&mut o.gear, &h.gear);
    write_str(&mut o.street, &h.street);
    *out = o;
}

/// Number of meshes in a car model: index 0 is the body shell, 1..=parts, then tyre and rim.
#[no_mangle]
pub unsafe extern "C" fn gl_car_model_mesh_count(g: *mut GlGame, class_index: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    g.game.model(class_of(class_index)).parts.len() as u32 + 3
}

/// Fetch one undamaged car model mesh in the chassis frame.
#[no_mangle]
pub unsafe extern "C" fn gl_car_model_mesh(
    g: *mut GlGame,
    class_index: u32,
    mesh: u32,
    out: *mut GlMesh,
) -> u32 {
    let (Some(g), Some(out)) = (g.as_mut(), out.as_mut()) else {
        return 0;
    };
    let model = g.game.model(class_of(class_index)).clone();
    let n = model.parts.len() as u32;
    let (m, paint) = if mesh == 0 {
        (&model.body, 0)
    } else if mesh <= n {
        let p = &model.parts[(mesh - 1) as usize];
        (&p.mesh, p.paint as u32)
    } else if mesh == n + 1 {
        (&model.tyre, 5)
    } else if mesh == n + 2 {
        (&model.rim, 6)
    } else {
        return 0;
    };
    g.export_mesh(m, None, None, paint, out);
    1
}

/// Deformed mesh of a crashed car (same indexing as `gl_car_model_mesh`; body and parts only).
/// Returns the body revision (0 if the car has no damage model, e.g. it is lane-following).
#[no_mangle]
pub unsafe extern "C" fn gl_car_damaged_mesh(
    g: *mut GlGame,
    car_id: u32,
    mesh: u32,
    out: *mut GlMesh,
) -> u64 {
    let (Some(g), Some(out)) = (g.as_mut(), out.as_mut()) else {
        return 0;
    };
    let id = VehicleId(car_id);
    let Some(class) = g.game.links.get(&id).map(|l| l.class) else {
        return 0;
    };
    let Some(dmg) = g.game.damage_of(id) else {
        return 0;
    };
    let (positions, normals, rev) = if mesh == 0 {
        (
            dmg.body.positions.clone(),
            dmg.body.normals.clone(),
            dmg.body.revision,
        )
    } else if let Some(p) = dmg.parts.get((mesh - 1) as usize) {
        (
            p.desc.mesh.positions.clone(),
            p.desc.mesh.normals.clone(),
            dmg.body.revision,
        )
    } else {
        return 0;
    };
    let model = g.game.model(class).clone();
    let (m, paint) = if mesh == 0 {
        (&model.body, 0)
    } else {
        (
            &model.parts[(mesh - 1) as usize].mesh,
            model.parts[(mesh - 1) as usize].paint as u32,
        )
    };
    g.export_mesh(m, Some(&positions), Some(&normals), paint, out);
    rev.max(1)
}

#[no_mangle]
pub unsafe extern "C" fn gl_city_buildings(g: *mut GlGame, out: *mut GlBuilding, cap: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let n = g.game.city.buildings.len();
    if !out.is_null() {
        for (i, b) in g.game.city.buildings.iter().take(cap as usize).enumerate() {
            let size = if g.unreal {
                [b.size.x * 100.0, b.size.z * 100.0, b.size.y * 100.0]
            } else {
                [b.size.x, b.size.y, b.size.z]
            };
            *out.add(i) = GlBuilding {
                base: g.p(b.base),
                size,
                style: b.style as u32,
                hue: b.hue,
                seed: b.seed,
            };
        }
    }
    n as u32
}

#[no_mangle]
pub unsafe extern "C" fn gl_city_blocks(g: *mut GlGame, out: *mut GlBlock, cap: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let n = g.game.city.blocks.len();
    if !out.is_null() {
        for (i, b) in g.game.city.blocks.iter().take(cap as usize).enumerate() {
            let outline = b.outline(if b.outer {
                0.0
            } else {
                gridlock_game::city::CORNER_RADIUS
            });
            let mut o = GlBlock {
                point_count: outline.len().min(32) as u32,
                outer: b.outer as u32,
                height: gridlock_game::city::CURB_HEIGHT * if g.unreal { 100.0 } else { 1.0 },
                outline: [[0.0; 3]; 32],
            };
            for (k, p) in outline.iter().take(32).enumerate() {
                o.outline[k] = g.p(Vec3::new(p.x, 0.0, p.y));
            }
            *out.add(i) = o;
        }
    }
    n as u32
}

/// Road markings: fills headers; points for marking `i` come from `gl_city_marking_points`.
#[no_mangle]
pub unsafe extern "C" fn gl_city_markings(g: *mut GlGame, out: *mut GlMarking, cap: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let total = g.game.city.markings.len() + g.game.city.crosswalks.len();
    if !out.is_null() {
        for i in 0..total.min(cap as usize) {
            let m = if i < g.game.city.markings.len() {
                let m = &g.game.city.markings[i];
                let kind = match m.kind {
                    MarkKind::WhiteSolid => 0,
                    MarkKind::WhiteDashed => 1,
                    MarkKind::YellowSolid => 2,
                    MarkKind::StopLine => 3,
                };
                GlMarking {
                    kind,
                    point_count: m.points.len() as u32,
                    width: m.width,
                }
            } else {
                let c = &g.game.city.crosswalks[i - g.game.city.markings.len()];
                GlMarking {
                    kind: 4,
                    point_count: 2,
                    width: c.width,
                }
            };
            *out.add(i) = m;
        }
    }
    total as u32
}

/// Points of marking `index` as xyz triplets. Returns the point count.
#[no_mangle]
pub unsafe extern "C" fn gl_city_marking_points(
    g: *mut GlGame,
    index: u32,
    out: *mut f32,
    cap_points: u32,
) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let i = index as usize;
    let pts: Vec<Vec3> = if i < g.game.city.markings.len() {
        g.game.city.markings[i].points.clone()
    } else if let Some(c) = g.game.city.crosswalks.get(i - g.game.city.markings.len()) {
        vec![c.a, c.b]
    } else {
        return 0;
    };
    if !out.is_null() {
        for (k, p) in pts.iter().take(cap_points as usize).enumerate() {
            let v = g.p(*p);
            *out.add(k * 3) = v[0];
            *out.add(k * 3 + 1) = v[1];
            *out.add(k * 3 + 2) = v[2];
        }
    }
    pts.len() as u32
}

/// Street-lamp light positions (lamp heads).
#[no_mangle]
pub unsafe extern "C" fn gl_city_lamp_lights(g: *mut GlGame, out: *mut f32, cap: u32) -> u32 {
    let Some(g) = g.as_mut() else { return 0 };
    let n = g.game.city.lamp_lights.len();
    if !out.is_null() {
        for (k, p) in g
            .game
            .city
            .lamp_lights
            .iter()
            .take(cap as usize)
            .enumerate()
        {
            let v = g.p(*p);
            *out.add(k * 3) = v[0];
            *out.add(k * 3 + 1) = v[1];
            *out.add(k * 3 + 2) = v[2];
        }
    }
    n as u32
}

#[no_mangle]
pub unsafe extern "C" fn gl_game_set_hour(g: *mut GlGame, hour: f32) {
    if let Some(g) = g.as_mut() {
        g.game.traffic.env_mut().hour = hour.rem_euclid(24.0) as f64;
    }
}

/// Engine version string (static, null-terminated).
#[no_mangle]
pub extern "C" fn gl_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_update_and_query() {
        unsafe {
            let cfg = GlConfig {
                seed: 7,
                traffic: 30,
                police: 1,
                start_hour: 12.0,
                player_class: 1,
                coords: GL_COORDS_UNREAL,
                props: 1,
            };
            let g = gl_game_create(&cfg);
            assert!(!g.is_null());
            let pad = GlPad {
                axes: [0.0, 0.0, 0.0, 0.0, 0.0, 0.6],
                buttons: 0,
                connected: 1,
            };
            for _ in 0..120 {
                gl_game_update(g, 1.0 / 60.0, &pad, std::ptr::null(), 0);
            }
            let n = gl_game_cars(g, std::ptr::null_mut(), 0);
            assert!(n > 10);
            let mut cars = vec![GlCarView::default(); n as usize];
            assert_eq!(gl_game_cars(g, cars.as_mut_ptr(), n), n);
            let player = cars.iter().find(|c| c.flags & 1 != 0).expect("player car");
            // Unreal: Z up, cm. Cars sit on the ground (Z close to 0).
            assert!(
                player.pose.pos[2].abs() < 50.0,
                "player z {}",
                player.pose.pos[2]
            );
            let mut hud = GlHud::default();
            gl_game_hud(g, &mut hud);
            assert!(
                hud.speed_kmh > 1.0,
                "player should be moving under throttle: {}",
                hud.speed_kmh
            );
            let mut mesh = GlMesh {
                positions: std::ptr::null(),
                normals: std::ptr::null(),
                uvs: std::ptr::null(),
                indices: std::ptr::null(),
                vertex_count: 0,
                index_count: 0,
                paint: 0,
            };
            let count = gl_car_model_mesh_count(g, 1);
            assert!(count > 5);
            assert_eq!(gl_car_model_mesh(g, 1, 0, &mut mesh), 1);
            assert!(mesh.vertex_count > 500 && mesh.index_count % 3 == 0);
            let nb = gl_city_buildings(g, std::ptr::null_mut(), 0);
            assert!(nb > 20);
            let nm = gl_city_markings(g, std::ptr::null_mut(), 0);
            assert!(nm > 100);
            let mut pts = vec![0.0f32; 3 * 256];
            assert!(gl_city_marking_points(g, 0, pts.as_mut_ptr(), 256) >= 2);
            assert!(gl_game_action(g, c"indicator_left".as_ptr()) == 1);
            gl_game_destroy(g);
        }
    }

    #[test]
    fn unreal_rotation_matches_mirrored_axes() {
        let g = GlGame {
            game: Game::new(GameConfig {
                traffic: 0,
                props: false,
                ..Default::default()
            }),
            unreal: true,
            kb: Default::default(),
            mesh_pos: vec![],
            mesh_nrm: vec![],
            mesh_uv: vec![],
            mesh_idx: vec![],
            pending_events: vec![],
        };
        let q = Quat::from_rotation_y(0.7) * Quat::from_rotation_x(0.3);
        let v = Vec3::new(0.3, -1.2, 2.0);
        let rotated = g.d(q * v);
        let qu = g.q(q);
        let qu = Quat::from_xyzw(qu[0], qu[1], qu[2], qu[3]);
        let vu = g.d(v);
        let r2 = qu * Vec3::new(vu[0], vu[1], vu[2]);
        assert!(
            (r2 - Vec3::from(rotated)).length() < 1e-4,
            "{r2:?} vs {rotated:?}"
        );
    }
}
