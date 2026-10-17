//! Headless smoke tests: they skip (pass) when no GPU adapter is available.

use glam::{Mat4, Vec3, Vec4};
use gridlock_render::*;

fn renderer(w: u32, h: u32) -> Option<Renderer> {
    let mut s = RenderSettings::software();
    s.capture_warmup_frames = 4;
    s.shadow_resolution = 1024;
    match Renderer::new_headless(w, h, s) {
        Ok(r) => Some(r),
        Err(e) if format!("{e}").contains("no GPU adapter") => {
            eprintln!("skipping: {e:#}");
            None
        }
        Err(e) => panic!("{e:#}"),
    }
}

fn save(name: &str, w: u32, h: u32, px: &[u8]) {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = image::RgbaImage::from_raw(w, h, px.to_vec()).map(|i| i.save(&path));
    println!("saved {}", path.display());
}

fn mean_luma(px: &[u8]) -> f32 {
    let mut s = 0.0f64;
    for p in px.chunks_exact(4) {
        s += 0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64;
    }
    (s / (px.len() / 4) as f64 / 255.0) as f32
}

fn simple_scene(r: &mut Renderer) {
    let ground = r.create_mesh(&procedural::plane(200.0, 200.0));
    let ball = r.create_mesh(&procedural::sphere(1.0, 32, 16));
    let gm = r.create_material(&Material::solid(Vec3::splat(0.3), 0.8, 0.0));
    let paint = r.create_material(&Material::car_paint(Vec3::new(0.6, 0.02, 0.02), true));
    let glass = r.create_material(&Material::glass(Vec3::splat(0.2), 0.2));
    r.add_instance(ground, gm, Mat4::IDENTITY);
    r.add_instance(
        ball,
        paint,
        Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)),
    );
    r.add_instance(
        ball,
        glass,
        Mat4::from_translation(Vec3::new(2.5, 1.0, 0.0)),
    );
}

#[test]
fn day_frame_is_sane() {
    let Some(mut r) = renderer(480, 270) else {
        return;
    };
    simple_scene(&mut r);
    r.set_environment(&Environment {
        sun_direction: Environment::sun_from_angles(40.0, 30.0),
        ..Default::default()
    });
    let cam = Camera::look_at(Vec3::new(0.0, 1.7, 8.0), Vec3::new(0.0, 1.0, 0.0), 60.0);
    let (w, h, px) = r.capture_rgba8(&cam).expect("capture");
    assert_eq!(px.len(), (w * h * 4) as usize);
    save("smoke_day.png", w, h, &px);
    let m = mean_luma(&px);
    println!("day mean luminance {m:.3}  stats {:?}", r.stats());
    assert!(m > 0.08 && m < 0.9, "mean luminance {m}");
    // No pure black holes (NaNs end up black after clamping).
    let black = px
        .chunks_exact(4)
        .filter(|p| p[0] == 0 && p[1] == 0 && p[2] == 0)
        .count();
    assert!(black < px.len() / 4 / 50, "{black} black pixels");
}

#[test]
fn night_frame_with_lights_is_sane() {
    let Some(mut r) = renderer(480, 270) else {
        return;
    };
    simple_scene(&mut r);
    r.set_environment(&Environment {
        sun_direction: Environment::sun_from_angles(-20.0, 30.0),
        wetness: 1.0,
        puddles: 0.6,
        rain: 0.8,
        ..Default::default()
    });
    let mut lights = Vec::new();
    for i in 0..64 {
        let x = (i % 8) as f32 * 6.0 - 21.0;
        let z = (i / 8) as f32 * 6.0 - 30.0;
        lights.push(Light::point(
            Vec3::new(x, 4.0, z),
            kelvin_to_rgb(3500.0),
            3000.0,
            15.0,
        ));
    }
    lights.push(Light {
        cast_shadows: true,
        ..Light::spot(
            Vec3::new(-3.0, 1.0, 4.0),
            Vec3::new(0.3, -0.2, -1.0),
            Vec3::ONE,
            20000.0,
            40.0,
            0.3,
            0.5,
        )
    });
    r.set_lights(&lights);
    r.set_particles(&[
        Particle {
            position: Vec3::new(1.0, 0.5, 2.0),
            size: 0.6,
            ..Default::default()
        },
        Particle {
            position: Vec3::new(-1.0, 0.5, 2.0),
            kind: ParticleKind::Spark,
            emissive: 5000.0,
            color: Vec4::new(1.0, 0.6, 0.2, 1.0),
            velocity: Vec3::new(3.0, 2.0, 0.0),
            size: 0.03,
            ..Default::default()
        },
    ]);
    r.add_decal(&Decal::road(
        Vec3::new(0.0, 0.0, 3.0),
        Vec3::Z,
        0.15,
        6.0,
        DecalKind::Dashed {
            dash: 3.0,
            gap: 6.0,
        },
        Vec4::new(0.8, 0.8, 0.8, 1.0),
    ));
    r.debug_line(Vec3::ZERO, Vec3::Y * 3.0, Vec4::new(1.0, 0.0, 0.0, 1.0));
    let cam = Camera::look_at(Vec3::new(0.0, 1.7, 8.0), Vec3::new(0.0, 1.0, 0.0), 60.0);
    let (w, h, px) = r.capture_rgba8(&cam).expect("capture");
    save("smoke_night.png", w, h, &px);
    let m = mean_luma(&px);
    println!("night mean luminance {m:.3}  stats {:?}", r.stats());
    assert!(m > 0.02 && m < 0.8, "mean luminance {m}");
}
