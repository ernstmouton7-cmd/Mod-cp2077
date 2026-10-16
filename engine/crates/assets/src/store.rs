//! [`AssetStore`]: resolves manifest ids to files in the local cache and
//! loads them.
//!
//! Cache layout: `<assets_dir>/cache/<id>/<res>/<file>` (exactly the `path`
//! fields of the manifest, below `cache/<id>/`).

use std::path::{Path, PathBuf};

use crate::hdri::HdrImage;
use crate::manifest::{AssetEntry, AssetKind, Defaults, Manifest, NormalConvention, Res, Role};
use crate::mips::NormalVariance;
use crate::model::Model;
use crate::texture::{self, PixelFormat, TextureData, TextureUsage};
use crate::AssetError;

/// Environment variable that overrides the assets directory (the one holding `manifest.toml`).
pub const ENV_ASSETS_DIR: &str = "GRIDLOCK_ASSETS_DIR";
/// Environment variable that overrides the cache directory.
pub const ENV_CACHE_DIR: &str = "GRIDLOCK_ASSET_CACHE";

/// One local file of a resolved asset.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFile {
    pub role: Role,
    pub path: PathBuf,
    /// Source convention for normal maps.
    pub normal: Option<NormalConvention>,
}

/// An asset whose files are all present locally at `res`.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAsset {
    pub id: String,
    pub kind: AssetKind,
    pub res: Res,
    pub files: Vec<ResolvedFile>,
}

impl ResolvedAsset {
    /// First file with this role.
    pub fn file(&self, role: Role) -> Option<&ResolvedFile> {
        self.files.iter().find(|f| f.role == role)
    }

    pub fn path(&self, role: Role) -> Option<&Path> {
        self.file(role).map(|f| f.path.as_path())
    }
}

/// A tiling PBR material decoded for upload.
#[derive(Debug, Clone)]
pub struct MaterialImages {
    pub id: String,
    pub res: Res,
    /// `Rgba8UnormSrgb`; alpha = opacity map if the material has one, else 255.
    pub base_color: TextureData,
    /// `Rgba8Unorm`, OpenGL convention, renormalised per mip.
    pub normal: Option<TextureData>,
    /// `Rgba8Unorm`: R = AO, G = roughness (Toksvig-widened mips), B = metalness, A = 255.
    pub orm: TextureData,
    /// `R8Unorm`/`R16Unorm` height (Poly Haven: 0.5 = reference plane).
    pub height: Option<TextureData>,
    /// `Rgba8UnormSrgb` emissive colour (e.g. lit windows at night).
    pub emission: Option<TextureData>,
    /// Real-world size of one tile in metres, when published.
    pub tile_size_m: Option<[f32; 2]>,
    /// Number of roughness mip levels adjusted by Toksvig (0 if no normal map).
    pub toksvig_levels: usize,
    /// Which convention the normal map was stored in before conversion to GL.
    pub normal_source_convention: Option<NormalConvention>,
}

/// Resolves ids from the manifest to cached files and loads them.
#[derive(Debug, Clone)]
pub struct AssetStore {
    assets_dir: PathBuf,
    cache_dir: PathBuf,
    manifest: Manifest,
    preferred: Defaults,
    toksvig: bool,
}

impl AssetStore {
    /// Open the assets directory (the one containing `manifest.toml`).
    /// The cache is `<dir>/cache` unless `GRIDLOCK_ASSET_CACHE` is set.
    pub fn open(assets_dir: impl AsRef<Path>) -> Result<Self, AssetError> {
        let assets_dir = assets_dir.as_ref().to_path_buf();
        let manifest = Manifest::load(&assets_dir.join("manifest.toml"))?;
        let cache_dir = std::env::var_os(ENV_CACHE_DIR)
            .map(PathBuf::from)
            .unwrap_or_else(|| assets_dir.join("cache"));
        let preferred = manifest.defaults.clone();
        Ok(AssetStore {
            assets_dir,
            cache_dir,
            manifest,
            preferred,
            toksvig: true,
        })
    }

    /// Find the assets directory: `$GRIDLOCK_ASSETS_DIR`, else the nearest
    /// `assets/manifest.toml` or `engine/assets/manifest.toml` walking up from
    /// the current directory and from the executable, else the source tree
    /// this crate was built from.
    pub fn discover() -> Result<Self, AssetError> {
        Self::open(Self::find_assets_dir())
    }

    /// See [`AssetStore::discover`].
    pub fn find_assets_dir() -> PathBuf {
        if let Some(d) = std::env::var_os(ENV_ASSETS_DIR) {
            return PathBuf::from(d);
        }
        let mut starts = Vec::new();
        if let Ok(c) = std::env::current_dir() {
            starts.push(c);
        }
        if let Some(p) = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf))
        {
            starts.push(p);
        }
        for s in starts {
            for dir in s.ancestors() {
                for cand in [dir.join("assets"), dir.join("engine").join("assets")] {
                    if cand.join("manifest.toml").is_file() {
                        return cand;
                    }
                }
            }
        }
        Self::source_assets_dir()
    }

    /// `engine/assets` of the source tree this crate was compiled from.
    pub fn source_assets_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
    }

    /// Use a different cache directory.
    pub fn with_cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = dir.into();
        self
    }

    /// Enable/disable Toksvig roughness widening in [`AssetStore::load_material`] (default on).
    pub fn with_toksvig(mut self, on: bool) -> Self {
        self.toksvig = on;
        self
    }

    /// Preferred resolution for a kind when several are cached (defaults from the manifest).
    pub fn set_preferred_res(&mut self, kind: AssetKind, res: Res) {
        match kind {
            AssetKind::Texture => self.preferred.texture_res = res,
            AssetKind::Hdri => self.preferred.hdri_res = res,
            AssetKind::Model => self.preferred.model_res = res,
        }
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn assets_dir(&self) -> &Path {
        &self.assets_dir
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Manifest entry for an id.
    pub fn asset(&self, id: &str) -> Result<&AssetEntry, AssetError> {
        self.manifest
            .get(id)
            .ok_or_else(|| AssetError::UnknownAsset(id.to_string()))
    }

    /// Ids of all assets of a kind, in manifest order.
    pub fn ids(&self, kind: AssetKind) -> Vec<&str> {
        self.manifest
            .assets
            .iter()
            .filter(|a| a.kind == kind)
            .map(|a| a.id.as_str())
            .collect()
    }

    /// `<cache>/<id>`.
    pub fn asset_dir(&self, id: &str) -> PathBuf {
        self.cache_dir.join(id)
    }

    fn local_files(&self, a: &AssetEntry, res: Res) -> Option<Vec<ResolvedFile>> {
        let dir = self.asset_dir(&a.id);
        let mut out = Vec::new();
        let mut any = false;
        for f in a.files_at(res) {
            any = true;
            if f.role == Role::Archive {
                for m in &f.members {
                    let p = dir.join(&m.path);
                    if !p.is_file() {
                        return None;
                    }
                    out.push(ResolvedFile {
                        role: m.role,
                        path: p,
                        normal: m.normal,
                    });
                }
            } else {
                let p = dir.join(&f.path);
                match std::fs::metadata(&p) {
                    Ok(md) if md.is_file() && md.len() == f.size => {}
                    _ => return None,
                }
                out.push(ResolvedFile {
                    role: f.role,
                    path: p,
                    normal: f.normal,
                });
            }
        }
        any.then_some(out)
    }

    /// Best locally available resolution: the preferred one, else the next
    /// lower, else the next higher.
    pub fn local_res(&self, id: &str) -> Option<Res> {
        let a = self.manifest.get(id)?;
        let pref = self.preferred.for_kind(a.kind);
        let mut order: Vec<Res> = Res::ALL
            .iter()
            .rev()
            .filter(|r| **r <= pref)
            .copied()
            .collect();
        order.extend(Res::ALL.iter().filter(|r| **r > pref));
        order
            .into_iter()
            .find(|r| self.local_files(a, *r).is_some())
    }

    /// Whether the asset is fully present in the cache at some resolution.
    pub fn is_cached(&self, id: &str) -> bool {
        self.local_res(id).is_some()
    }

    /// Resolve to local file paths (best available resolution).
    pub fn resolve(&self, id: &str) -> Result<ResolvedAsset, AssetError> {
        let a = self.asset(id)?;
        let res = self.local_res(id).ok_or_else(|| AssetError::NotCached {
            id: id.to_string(),
            hint: format!(
                "run `cargo run -p gridlock-assets --bin fetch-assets --release -- --only {id}`; cache dir {}",
                self.cache_dir.display()
            ),
        })?;
        let files = self.local_files(a, res).expect("local_res checked");
        Ok(ResolvedAsset {
            id: id.to_string(),
            kind: a.kind,
            res,
            files,
        })
    }

    fn resolve_kind(&self, id: &str, kind: AssetKind) -> Result<ResolvedAsset, AssetError> {
        let a = self.asset(id)?;
        if a.kind != kind {
            return Err(AssetError::WrongKind {
                id: id.to_string(),
                expected: kind,
                found: a.kind,
            });
        }
        self.resolve(id)
    }

    /// Load one map of a texture asset with the decoding its role implies.
    pub fn load_map(&self, id: &str, role: Role) -> Result<TextureData, AssetError> {
        let r = self.resolve_kind(id, AssetKind::Texture)?;
        let f = r.file(role).ok_or(AssetError::MissingMap {
            id: id.to_string(),
            role,
        })?;
        let usage = match role {
            Role::Albedo | Role::Emission => TextureUsage::Color,
            Role::Normal => TextureUsage::Normal(f.normal.unwrap_or_default()),
            Role::Arm => TextureUsage::Data,
            _ => TextureUsage::Mask,
        };
        texture::load_texture(&f.path, usage)
    }

    /// Load a complete tiling material (see [`MaterialImages`]). Maps are
    /// decoded in parallel.
    pub fn load_material(&self, id: &str) -> Result<MaterialImages, AssetError> {
        let entry = self.asset(id)?;
        let r = self.resolve_kind(id, AssetKind::Texture)?;
        let albedo = r.file(Role::Albedo).ok_or(AssetError::MissingMap {
            id: id.to_string(),
            role: Role::Albedo,
        })?;
        let normal = r.file(Role::Normal);
        let default_metal = entry.metalness.unwrap_or(0.0);

        type Res2<T> = Result<T, AssetError>;
        let (base, nrm, orm, height, emission) = std::thread::scope(|s| {
            let base = s.spawn(|| -> Res2<TextureData> {
                let img = texture::open_image(&albedo.path)?;
                let mut rgba = img.into_rgba8();
                if let Some(op) = r.file(Role::Opacity) {
                    let o = texture::open_image(&op.path)?;
                    if o.width() == rgba.width() && o.height() == rgba.height() {
                        let a = texture::first_channel_u8(&o);
                        for (px, a) in rgba.pixels_mut().zip(a) {
                            px.0[3] = a;
                        }
                    }
                }
                let (w, h) = rgba.dimensions();
                Ok(TextureData::from_rgba8(w, h, rgba.into_raw(), true))
            });
            let nrm = s.spawn(|| -> Res2<Option<(TextureData, NormalVariance)>> {
                match normal {
                    Some(n) => Ok(Some(texture::load_normal_map(
                        &n.path,
                        n.normal.unwrap_or_default(),
                    )?)),
                    None => Ok(None),
                }
            });
            let orm = s.spawn(|| -> Res2<TextureData> { load_orm(&r, default_metal) });
            let height = s.spawn(|| -> Res2<Option<TextureData>> {
                r.file(Role::Height)
                    .map(|f| texture::load_texture(&f.path, TextureUsage::Mask))
                    .transpose()
            });
            let emission = s.spawn(|| -> Res2<Option<TextureData>> {
                r.file(Role::Emission)
                    .map(|f| texture::load_texture(&f.path, TextureUsage::Color))
                    .transpose()
            });
            (
                base.join().expect("albedo thread"),
                nrm.join().expect("normal thread"),
                orm.join().expect("orm thread"),
                height.join().expect("height thread"),
                emission.join().expect("emission thread"),
            )
        });
        let base_color = base?;
        let nrm = nrm?;
        let mut orm = orm?;
        let mut toksvig_levels = 0;
        if let (true, Some((_, var))) = (self.toksvig, &nrm) {
            toksvig_levels = texture::apply_toksvig(&mut orm, 1, var);
        }
        Ok(MaterialImages {
            id: id.to_string(),
            res: r.res,
            base_color,
            normal: nrm.map(|(t, _)| t),
            orm,
            height: height?,
            emission: emission?,
            tile_size_m: entry.tile_size_m,
            toksvig_levels,
            normal_source_convention: normal.map(|n| n.normal.unwrap_or_default()),
        })
    }

    /// Load an equirectangular HDRI.
    pub fn load_hdri(&self, id: &str) -> Result<HdrImage, AssetError> {
        let r = self.resolve_kind(id, AssetKind::Hdri)?;
        let f = r.file(Role::Hdri).ok_or(AssetError::MissingMap {
            id: id.to_string(),
            role: Role::Hdri,
        })?;
        HdrImage::load(&f.path)
    }

    /// Load a glTF model (geometry + material references; textures are
    /// decoded on demand with [`Model::load_texture`]).
    pub fn load_model(&self, id: &str) -> Result<Model, AssetError> {
        let r = self.resolve_kind(id, AssetKind::Model)?;
        let f = r.file(Role::Gltf).ok_or(AssetError::MissingMap {
            id: id.to_string(),
            role: Role::Gltf,
        })?;
        Model::load(&f.path)
    }
}

/// Build the ORM texture from either a packed ARM map or separate AO /
/// roughness / metalness maps.
fn load_orm(r: &ResolvedAsset, default_metal: f32) -> Result<TextureData, AssetError> {
    if let Some(arm) = r.file(Role::Arm) {
        return texture::load_texture(&arm.path, TextureUsage::Data);
    }
    let load = |role| -> Result<Option<(u32, u32, Vec<u8>)>, AssetError> {
        match r.file(role) {
            Some(f) => {
                let img = texture::open_image(&f.path)?;
                Ok(Some((
                    img.width(),
                    img.height(),
                    texture::first_channel_u8(&img),
                )))
            }
            None => Ok(None),
        }
    };
    let rough = load(Role::Roughness)?;
    let ao = load(Role::Ao)?;
    let metal = load(Role::Metalness)?;
    let (w, h) = rough
        .as_ref()
        .or(ao.as_ref())
        .or(metal.as_ref())
        .map(|(w, h, _)| (*w, *h))
        .unwrap_or((4, 4));
    let sample = |m: &Option<(u32, u32, Vec<u8>)>, x: u32, y: u32, default: u8| -> u8 {
        match m {
            Some((mw, mh, d)) => {
                let sx = (x as u64 * *mw as u64 / w as u64) as usize;
                let sy = (y as u64 * *mh as u64 / h as u64) as usize;
                d[sy * *mw as usize + sx]
            }
            None => default,
        }
    };
    let dm = (default_metal.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    let mut data = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h {
        for x in 0..w {
            data.extend_from_slice(&[
                sample(&ao, x, y, 255),
                sample(&rough, x, y, 128),
                sample(&metal, x, y, dm),
                255,
            ]);
        }
    }
    let t = TextureData::from_rgba8(w, h, data, false);
    debug_assert_eq!(t.format, PixelFormat::Rgba8Unorm);
    Ok(t)
}
