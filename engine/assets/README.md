# Gridlock assets

Photoreal street-level city assets for Gridlock: tiling PBR materials, HDRI
skies and glTF street furniture. **Everything is CC0** (public domain) from
[Poly Haven](https://polyhaven.com) and [ambientCG](https://ambientcg.com).
The files are not committed to git; they are downloaded into `cache/`.

| file | what |
|---|---|
| `sources.toml` | hand-edited list of assets (id, site, slug, notes) |
| `manifest.toml` | **generated** from `sources.toml`: exact URLs, byte sizes and MD5s for 1k/2k/4k |
| `LICENSES.md` | **generated** credits + licence verification per asset |
| `cache/` | downloaded files (git-ignored), `cache/<id>/<res>/...` |

## Fetching

From `engine/`:

```sh
cargo run -p gridlock-assets --bin fetch-assets --release
```

Default set: textures and models at 2k, HDRIs at 4k, about **520 MB**
(1k: ~135 MB, 4k: ~1.55 GB). Options:

```text
--res 1k|2k|4k        texture + model resolution
--hdri-res 1k|2k|4k   HDRI resolution (default 4k)
--only <id-prefix>    e.g. --only tex.asphalt --only hdri.   (repeatable; also selects optional assets)
--all                 include optional assets (two heavy hero trees, two extra HDRIs; +400 MB)
--list / --dry-run    show what is cached / what would be downloaded
--verify              re-check MD5s of files already present
--jobs N              parallel downloads (default 4)
--curl                use the system curl for every transfer
--no-curl-fallback    never fall back to curl
--keep-archives       keep ambientCG .zip files after extracting the maps
--assets-dir / --cache-dir DIR
```

Re-running is cheap: files already present with the right size are skipped,
interrupted downloads resume from `*.part`, every transfer is retried with
exponential backoff and checked against the manifest size (and MD5 for Poly
Haven). ambientCG ships zips; only the listed maps are extracted and the zip
is deleted.

**Networking.** HTTPS uses `ureq` + rustls with the operating system's root
certificates (Windows certificate store / macOS keychain / `/etc/ssl` or
`SSL_CERT_FILE` on Linux), so a TLS-inspecting proxy whose CA is installed in
the OS works. `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` and `NO_PROXY` are
honoured. `GRIDLOCK_EXTRA_CA=<pem file>` adds roots if needed. If ureq cannot
connect at all, the downloader falls back to `curl` (`curl.exe` ships with
Windows 10 1803+; every Linux has it), which has its own proxy/TLS handling.

Environment overrides used by both the tools and the runtime:
`GRIDLOCK_ASSETS_DIR` (directory with `manifest.toml`), `GRIDLOCK_ASSET_CACHE`
(cache directory).

## Using assets in code

```rust
use gridlock_assets::{AssetStore, Role};
let store = AssetStore::discover()?;                 // finds engine/assets
let road  = store.load_material("tex.asphalt")?;     // albedo sRGB, normal GL, ORM, height, mips
let sky   = store.load_hdri("hdri.day_clear")?;      // RGBA f32 equirect (+ estimate_sun())
let hyd   = store.load_model("model.fire_hydrant")?; // flattened meshes + PBR material refs
```

Conventions: albedo/emission `Rgba8UnormSrgb` (mips filtered in linear light);
normal maps `Rgba8Unorm`, **always OpenGL (+Y) convention** whatever the
source, renormalised per mip; ORM `Rgba8Unorm` R=AO G=roughness B=metalness
with Toksvig-widened roughness mips; height `R8Unorm`/`R16Unorm`; HDRIs linear
f32, +Y up, image centre looking down -Z. Ids are stable: code should only use
ids, never file names. `cargo test -p gridlock-assets --release` loads and
checks everything that is cached (and skips what is not).

Things that are deliberately **procedural** and not in the manifest: road
markings, car paint, wet asphalt/puddles (modulate the dry asphalt),
window glass, modern cobra-head street lights and traffic-signal poles (use
`tex.metal_powder_coat`), cars (use the glTF loader for user models).

## Adding an asset

1. Find a CC0 asset on Poly Haven (`https://polyhaven.com/a/<slug>`) or
   ambientCG (`https://ambientcg.com/view?id=<AssetId>`, materials only).
2. Add an `[[asset]]` block to `sources.toml` with a new stable `id`
   (`tex.*`, `hdri.*`, `model.*`), `kind`, `source`, `slug` and optionally
   `maps`, `max_res`, `optional`, `metalness`, `notes`.
3. Regenerate the manifest and licences, then fetch:

   ```sh
   cargo run -p gridlock-assets --bin gen-manifest --release
   cargo run -p gridlock-assets --bin fetch-assets --release -- --only <id>
   cargo test  -p gridlock-assets --release
   ```

   `gen-manifest` re-checks the CC0 licence on each asset page and drops
   assets that fail. Keep the default set (non-optional) under ~600 MB at
   2k; mark big things `optional = true`.
4. Commit `sources.toml`, `manifest.toml` and `LICENSES.md` (never `cache/`).

Other CC0 sites can be added by extending `src/bin/gen_manifest.rs`; the
runtime and downloader only read `manifest.toml`, so a hand-written entry
with `url`/`size`/`path` per file also works.

## Licensing

All assets are **CC0 1.0 Universal**: no attribution required, commercial
use and redistribution allowed. We credit authors in `LICENSES.md` anyway.
Poly Haven: <https://polyhaven.com/license>; ambientCG:
<https://docs.ambientcg.com/license/>. Do not add assets under any other
licence (CC-BY, "free for personal use", Sketchfab standard licences, ...):
`Manifest::validate` rejects anything that is not `CC0-1.0`.
