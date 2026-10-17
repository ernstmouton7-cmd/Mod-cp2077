//! Quality presets and tunable render settings.

/// Coarse quality level; see [`RenderSettings::preset`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Low,
    Medium,
    High,
    Ultra,
}

/// Final tone curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tonemapper {
    /// AgX base with a gentle "punchy" look (default; best hue preservation).
    AgX,
    /// ACES fitted RRT+ODT (Hill).
    Aces,
    /// Khronos PBR Neutral.
    Neutral,
}

/// Anti-aliasing mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AntiAliasing {
    None,
    Fxaa,
    /// Temporal AA (also performs upsampling when `render_scale < 1`).
    Taa,
}

/// Debug visualisation of intermediate buffers (shown instead of the lit image).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugView {
    None = 0,
    Albedo = 1,
    Normals = 2,
    Roughness = 3,
    AmbientOcclusion = 4,
    SkyVisibility = 5,
    DiffuseLighting = 6,
    SpecularLighting = 7,
    Ssr = 8,
    Metallic = 9,
}

/// Everything that can be tuned at runtime. Changing it via
/// [`crate::Renderer::set_settings`] recreates only what is needed.
#[derive(Clone, Debug)]
pub struct RenderSettings {
    pub quality: Quality,
    /// Internal resolution multiplier, 0.5..2.0.
    pub render_scale: f32,
    pub vsync: bool,
    pub anti_aliasing: AntiAliasing,
    /// Shadow map resolution per cascade.
    pub shadow_resolution: u32,
    /// 1..=4 cascades.
    pub shadow_cascades: u32,
    /// Shadowed distance for the sun (m).
    pub shadow_distance: f32,
    /// Percentage-closer soft shadows (blocker search) instead of fixed PCF.
    pub soft_shadows: bool,
    /// Number of shadowed spot lights (e.g. player headlights).
    pub spot_shadows: u32,
    pub spot_shadow_resolution: u32,
    pub ssao: bool,
    /// Ambient occlusion at half resolution.
    pub ssao_half_res: bool,
    pub ssr: bool,
    /// Screen space reflections at half resolution.
    pub ssr_half_res: bool,
    pub ssr_steps: u32,
    pub bloom: bool,
    pub bloom_intensity: f32,
    pub auto_exposure: bool,
    pub min_ev100: f32,
    pub max_ev100: f32,
    /// Exposure adaptation speed (1/s).
    pub exposure_speed: f32,
    pub tonemapper: Tonemapper,
    pub vignette: f32,
    pub film_grain: f32,
    pub chromatic_aberration: f32,
    pub motion_blur: f32,
    /// Sharpening after TAA (0..1).
    pub sharpen: f32,
    /// Maximum local lights considered per frame.
    pub max_lights: u32,
    /// Maximum lights stored per cluster.
    pub max_lights_per_cluster: u32,
    /// Lights that scatter into fog (halos and beams).
    pub volumetric_lights: u32,
    /// Max rain streak particles at rain = 1.
    pub rain_streaks: u32,
    /// Specular IBL cube resolution.
    pub ibl_resolution: u32,
    /// Height-field sky occlusion + bounce light (street canyons).
    pub sky_occlusion: bool,
    pub wireframe: bool,
    pub debug_view: DebugView,
    /// Frames rendered before a headless capture is read back (TAA/exposure convergence).
    pub capture_warmup_frames: u32,
    /// Collect GPU timestamps when supported.
    pub gpu_timing: bool,
}

impl RenderSettings {
    /// Settings for a given quality level.
    pub fn preset(q: Quality) -> Self {
        let base = Self {
            quality: q,
            render_scale: 1.0,
            vsync: true,
            anti_aliasing: AntiAliasing::Taa,
            shadow_resolution: 2048,
            shadow_cascades: 4,
            shadow_distance: 160.0,
            soft_shadows: false,
            spot_shadows: 2,
            spot_shadow_resolution: 1024,
            ssao: true,
            ssao_half_res: false,
            ssr: true,
            ssr_half_res: true,
            ssr_steps: 48,
            bloom: true,
            bloom_intensity: 0.05,
            auto_exposure: true,
            min_ev100: -4.0,
            max_ev100: 16.0,
            exposure_speed: 1.5,
            tonemapper: Tonemapper::AgX,
            vignette: 0.25,
            film_grain: 0.02,
            chromatic_aberration: 0.0,
            motion_blur: 0.0,
            sharpen: 0.15,
            max_lights: 1024,
            max_lights_per_cluster: 96,
            volumetric_lights: 16,
            rain_streaks: 12000,
            ibl_resolution: 128,
            sky_occlusion: true,
            wireframe: false,
            debug_view: DebugView::None,
            capture_warmup_frames: 10,
            gpu_timing: true,
        };
        match q {
            Quality::Low => Self {
                shadow_resolution: 1024,
                shadow_cascades: 2,
                shadow_distance: 80.0,
                spot_shadows: 0,
                ssao: false,
                ssr: false,
                anti_aliasing: AntiAliasing::Fxaa,
                bloom_intensity: 0.04,
                max_lights: 256,
                max_lights_per_cluster: 32,
                volumetric_lights: 0,
                rain_streaks: 3000,
                ibl_resolution: 64,
                sharpen: 0.0,
                film_grain: 0.0,
                ..base
            },
            Quality::Medium => Self {
                shadow_resolution: 1536,
                shadow_cascades: 3,
                shadow_distance: 120.0,
                spot_shadows: 1,
                ssao_half_res: true,
                ssr_steps: 32,
                max_lights: 512,
                max_lights_per_cluster: 64,
                volumetric_lights: 8,
                rain_streaks: 6000,
                ..base
            },
            Quality::High => base,
            Quality::Ultra => Self {
                shadow_resolution: 4096,
                shadow_distance: 250.0,
                soft_shadows: true,
                spot_shadows: 4,
                spot_shadow_resolution: 2048,
                ssr_half_res: false,
                ssr_steps: 64,
                max_lights: 2048,
                max_lights_per_cluster: 128,
                volumetric_lights: 32,
                rain_streaks: 24000,
                ibl_resolution: 256,
                ..base
            },
        }
    }

    /// Lightweight settings for the software rasteriser (tests, CI screenshots).
    pub fn software() -> Self {
        Self {
            shadow_resolution: 2048,
            shadow_cascades: 4,
            spot_shadows: 1,
            ssr_half_res: true,
            ssao_half_res: true,
            ibl_resolution: 64,
            rain_streaks: 6000,
            volumetric_lights: 12,
            capture_warmup_frames: 8,
            ..Self::preset(Quality::High)
        }
    }
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self::preset(Quality::High)
    }
}
