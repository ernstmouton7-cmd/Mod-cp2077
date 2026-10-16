//! Gridlock asset pipeline: manifest, local cache resolution and CPU-side
//! loaders for the free CC0 assets (Poly Haven, ambientCG) used by the city.
//!
//! The crate is renderer-agnostic: it produces plain byte/float buffers with
//! explicit [`PixelFormat`]s and flat [`Mesh`]es; the renderer decides how to
//! upload them.
//!
//! ```no_run
//! use gridlock_assets::AssetStore;
//! let store = AssetStore::discover()?;
//! let asphalt = store.load_material("tex.asphalt")?; // sRGB albedo, GL normal, ORM, height
//! let sky = store.load_hdri("hdri.day_clear")?;      // RGBA f32 equirect
//! let hydrant = store.load_model("model.fire_hydrant")?;
//! # Ok::<(), gridlock_assets::AssetError>(())
//! ```
//!
//! Conventions:
//! * Albedo/emission: `Rgba8UnormSrgb`, mips filtered in linear light.
//! * Normal maps: `Rgba8Unorm`, **always OpenGL convention** (+Y/green up,
//!   bitangent = cross(N, T) * w); DirectX sources are flipped on load.
//!   Renormalised per mip.
//! * ORM: `Rgba8Unorm`, R = AO, G = perceptual roughness, B = metalness
//!   (glTF layout). Roughness mips are widened from the normal map's
//!   variance (Toksvig/vMF) so distant bumpy surfaces don't sparkle.
//! * Height: `R8Unorm` or `R16Unorm`, linear.
//! * HDRIs: linear RGBA f32 equirect, +Y up (see [`hdri`]).

pub mod hdri;
pub mod manifest;
pub mod mips;
pub mod model;
pub mod store;
pub mod texture;

#[cfg(feature = "fetch")]
pub mod fetch;

use std::fmt;
use std::path::{Path, PathBuf};

pub use hdri::{HdrImage, SunEstimate};
pub use manifest::{
    AssetEntry, AssetKind, FileEntry, Manifest, NormalConvention, Res, Role, Source,
};
pub use mips::{MipLevel, NormalVariance};
pub use model::{AlphaMode, Material, Mesh, Model, TextureRef, TextureSlot};
pub use store::{AssetStore, MaterialImages, ResolvedAsset, ResolvedFile};
pub use texture::{PixelFormat, TextureData, TextureUsage};

/// Errors from manifest parsing, cache lookup and decoding.
#[derive(Debug)]
pub enum AssetError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Manifest(String),
    UnknownAsset(String),
    /// The asset is in the manifest but not in the local cache (run `fetch-assets`).
    NotCached {
        id: String,
        hint: String,
    },
    WrongKind {
        id: String,
        expected: AssetKind,
        found: AssetKind,
    },
    MissingMap {
        id: String,
        role: Role,
    },
    Decode {
        path: PathBuf,
        msg: String,
    },
    Gltf {
        path: PathBuf,
        msg: String,
    },
    Fetch(String),
}

impl AssetError {
    pub(crate) fn io(path: &Path, source: std::io::Error) -> Self {
        AssetError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Io { path, source } => write!(f, "{}: {source}", path.display()),
            AssetError::Manifest(m) => write!(f, "manifest: {m}"),
            AssetError::UnknownAsset(id) => write!(f, "unknown asset id '{id}'"),
            AssetError::NotCached { id, hint } => write!(f, "asset '{id}' is not cached ({hint})"),
            AssetError::WrongKind {
                id,
                expected,
                found,
            } => {
                write!(f, "asset '{id}' is a {found}, expected a {expected}")
            }
            AssetError::MissingMap { id, role } => write!(f, "asset '{id}' has no {role:?} map"),
            AssetError::Decode { path, msg } => write!(f, "decoding {}: {msg}", path.display()),
            AssetError::Gltf { path, msg } => write!(f, "glTF {}: {msg}", path.display()),
            AssetError::Fetch(m) => write!(f, "fetch: {m}"),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AssetError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
