//! # gridlock-render
//!
//! Physically based, deferred + clustered renderer for the Gridlock driving
//! engine, built on `wgpu` (Vulkan / DX12 / Metal). See `README.md` for the
//! frame graph, formats and quality presets.
//!
//! The API is renderer-agnostic of game logic: upload meshes, textures and
//! materials, place instances, set the environment, lights and particles each
//! frame, then call [`Renderer::render`] (windowed) or
//! [`Renderer::capture_png`] (headless).

mod frame;
mod gpu;
mod pipelines;
pub mod procedural;
mod renderer;
mod settings;
mod shadow;
mod sky;
mod targets;
mod texture;
mod timing;
mod types;

pub use renderer::Renderer;
pub use settings::{AntiAliasing, DebugView, Quality, RenderSettings, Tonemapper};
pub use types::*;

/// Re-export so the app uses the exact same wgpu version.
pub use wgpu;
