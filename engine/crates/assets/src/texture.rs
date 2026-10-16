//! Texture decoding into renderer-agnostic CPU images with full mip chains.
//!
//! Colour space rules (match these when creating GPU textures):
//!
//! | usage                  | output format            | notes                                   |
//! |------------------------|--------------------------|-----------------------------------------|
//! | [`TextureUsage::Color`]  | [`PixelFormat::Rgba8UnormSrgb`] | albedo / emission; mips filtered in linear light |
//! | [`TextureUsage::Normal`] | [`PixelFormat::Rgba8Unorm`]     | always OpenGL (+Y) convention, renormalised per mip |
//! | [`TextureUsage::Data`]   | [`PixelFormat::Rgba8Unorm`]     | packed linear data (ORM/ARM)            |
//! | [`TextureUsage::Mask`]   | [`PixelFormat::R8Unorm`] / [`PixelFormat::R16Unorm`] | single linear channel (height, roughness, ...) |

use std::path::Path;

use image::DynamicImage;

use crate::manifest::NormalConvention;
use crate::mips::{self, MipLevel, NormalVariance};
use crate::AssetError;

/// Pixel layout of every level of a [`TextureData`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// 8-bit RGBA, colour channels sRGB-encoded (sample with an `*Srgb` GPU format).
    Rgba8UnormSrgb,
    /// 8-bit RGBA, linear.
    Rgba8Unorm,
    /// 8-bit single channel, linear.
    R8Unorm,
    /// 16-bit single channel, linear, native-endian.
    R16Unorm,
    /// 16-bit float RGBA (`half::f16`, native-endian).
    Rgba16Float,
    /// 32-bit float RGBA, native-endian.
    Rgba32Float,
}

impl PixelFormat {
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            PixelFormat::Rgba8UnormSrgb | PixelFormat::Rgba8Unorm => 4,
            PixelFormat::R8Unorm => 1,
            PixelFormat::R16Unorm => 2,
            PixelFormat::Rgba16Float => 8,
            PixelFormat::Rgba32Float => 16,
        }
    }

    pub fn is_srgb(self) -> bool {
        matches!(self, PixelFormat::Rgba8UnormSrgb)
    }
}

/// How a texture file is going to be sampled; decides decoding and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureUsage {
    /// sRGB colour (albedo, emission). Alpha (if any) is kept, linear.
    Color,
    /// Tangent-space normal map stored in the given convention; output is OpenGL.
    Normal(NormalConvention),
    /// Linear multi-channel data (e.g. packed AO/roughness/metalness).
    Data,
    /// Linear single-channel data (height, roughness, metalness, AO, opacity).
    Mask,
}

/// A decoded texture with its full mip chain (level 0 first).
#[derive(Debug, Clone, PartialEq)]
pub struct TextureData {
    pub format: PixelFormat,
    pub mips: Vec<MipLevel>,
}

impl TextureData {
    pub fn width(&self) -> u32 {
        self.mips.first().map_or(0, |m| m.width)
    }

    pub fn height(&self) -> u32 {
        self.mips.first().map_or(0, |m| m.height)
    }

    pub fn mip_count(&self) -> u32 {
        self.mips.len() as u32
    }

    /// Total bytes of all levels.
    pub fn total_bytes(&self) -> usize {
        self.mips.iter().map(|m| m.data.len()).sum()
    }

    /// Bytes per row of level `level` (tightly packed).
    pub fn row_bytes(&self, level: usize) -> usize {
        self.mips[level].width as usize * self.format.bytes_per_pixel()
    }

    /// Checks that the chain is complete and every level has the right size.
    pub fn validate(&self) -> Result<(), String> {
        let (w, h) = (self.width(), self.height());
        if w == 0 || h == 0 {
            return Err("empty texture".into());
        }
        let want = mips::full_mip_count(w, h);
        if self.mip_count() != want {
            return Err(format!(
                "{} mips for {w}x{h}, expected {want}",
                self.mip_count()
            ));
        }
        let (mut lw, mut lh) = (w, h);
        for (i, m) in self.mips.iter().enumerate() {
            if (m.width, m.height) != (lw, lh) {
                return Err(format!(
                    "mip {i} is {}x{}, expected {lw}x{lh}",
                    m.width, m.height
                ));
            }
            let bytes = lw as usize * lh as usize * self.format.bytes_per_pixel();
            if m.data.len() != bytes {
                return Err(format!(
                    "mip {i} has {} bytes, expected {bytes}",
                    m.data.len()
                ));
            }
            (lw, lh) = mips::next_size(lw, lh);
        }
        Ok(())
    }

    /// Wrap an RGBA8 level 0 and generate its chain.
    pub fn from_rgba8(width: u32, height: u32, data: Vec<u8>, srgb: bool) -> Self {
        let lvl = MipLevel {
            width,
            height,
            data,
        };
        TextureData {
            format: if srgb {
                PixelFormat::Rgba8UnormSrgb
            } else {
                PixelFormat::Rgba8Unorm
            },
            mips: mips::build_chain_u8(lvl, 4, srgb),
        }
    }
}

/// Open an image file with the `image` crate (format from content/extension).
pub fn open_image(path: &Path) -> Result<DynamicImage, AssetError> {
    image::ImageReader::open(path)
        .map_err(|e| AssetError::io(path, e))?
        .with_guessed_format()
        .map_err(|e| AssetError::io(path, e))?
        .decode()
        .map_err(|e| AssetError::Decode {
            path: path.to_path_buf(),
            msg: e.to_string(),
        })
}

fn decode_bytes(bytes: &[u8], what: &Path) -> Result<DynamicImage, AssetError> {
    image::load_from_memory(bytes).map_err(|e| AssetError::Decode {
        path: what.to_path_buf(),
        msg: e.to_string(),
    })
}

/// Decode and mip a texture file.
pub fn load_texture(path: &Path, usage: TextureUsage) -> Result<TextureData, AssetError> {
    Ok(texture_from_image(open_image(path)?, usage))
}

/// Decode and mip an in-memory encoded image (PNG/JPEG/...), e.g. embedded in a `.glb`.
pub fn decode_texture(bytes: &[u8], usage: TextureUsage) -> Result<TextureData, AssetError> {
    Ok(texture_from_image(
        decode_bytes(bytes, Path::new("<memory>"))?,
        usage,
    ))
}

/// Load a normal map, convert it to OpenGL convention and return the
/// variance pyramid used for Toksvig roughness widening.
pub fn load_normal_map(
    path: &Path,
    convention: NormalConvention,
) -> Result<(TextureData, NormalVariance), AssetError> {
    Ok(normal_from_image(&open_image(path)?, convention))
}

/// Convert an already-decoded image.
pub fn texture_from_image(img: DynamicImage, usage: TextureUsage) -> TextureData {
    match usage {
        TextureUsage::Color | TextureUsage::Data => {
            let srgb = usage == TextureUsage::Color;
            let rgba = img.into_rgba8();
            let (w, h) = rgba.dimensions();
            TextureData::from_rgba8(w, h, rgba.into_raw(), srgb)
        }
        TextureUsage::Normal(conv) => normal_from_image(&img, conv).0,
        TextureUsage::Mask => mask_from_image(&img),
    }
}

/// Single linear channel; 16-bit sources stay 16-bit.
pub fn mask_from_image(img: &DynamicImage) -> TextureData {
    let sixteen = matches!(
        img,
        DynamicImage::ImageLuma16(_)
            | DynamicImage::ImageLumaA16(_)
            | DynamicImage::ImageRgb16(_)
            | DynamicImage::ImageRgba16(_)
    );
    if sixteen {
        let l = first_channel_u16(img);
        let (w, h) = (img.width(), img.height());
        let data: Vec<u8> = l.iter().flat_map(|v| v.to_ne_bytes()).collect();
        TextureData {
            format: PixelFormat::R16Unorm,
            mips: mips::build_chain_u16(
                MipLevel {
                    width: w,
                    height: h,
                    data,
                },
                1,
            ),
        }
    } else {
        let data = first_channel_u8(img);
        let (w, h) = (img.width(), img.height());
        TextureData {
            format: PixelFormat::R8Unorm,
            mips: mips::build_chain_u8(
                MipLevel {
                    width: w,
                    height: h,
                    data,
                },
                1,
                false,
            ),
        }
    }
}

/// The red (or luma) channel as 8-bit, without any colour weighting.
pub fn first_channel_u8(img: &DynamicImage) -> Vec<u8> {
    match img {
        DynamicImage::ImageLuma8(b) => b.as_raw().clone(),
        DynamicImage::ImageLumaA8(b) => b.as_raw().chunks_exact(2).map(|p| p[0]).collect(),
        DynamicImage::ImageRgb8(b) => b.as_raw().chunks_exact(3).map(|p| p[0]).collect(),
        DynamicImage::ImageRgba8(b) => b.as_raw().chunks_exact(4).map(|p| p[0]).collect(),
        other => other
            .to_rgba8()
            .as_raw()
            .chunks_exact(4)
            .map(|p| p[0])
            .collect(),
    }
}

fn first_channel_u16(img: &DynamicImage) -> Vec<u16> {
    match img {
        DynamicImage::ImageLuma16(b) => b.as_raw().clone(),
        other => other
            .to_rgba16()
            .as_raw()
            .chunks_exact(4)
            .map(|p| p[0])
            .collect(),
    }
}

/// Decode a normal map into unit OpenGL-convention vectors (xyz interleaved).
/// DirectX sources get their green channel flipped.
pub fn decode_normals(img: &DynamicImage, convention: NormalConvention) -> Vec<f32> {
    let flip_y = convention == NormalConvention::Dx;
    let mut out = Vec::with_capacity(img.width() as usize * img.height() as usize * 3);
    let mut push = |r: f32, g: f32, b: f32| {
        let x = r * 2.0 - 1.0;
        let mut y = g * 2.0 - 1.0;
        if flip_y {
            y = -y;
        }
        let z = b * 2.0 - 1.0;
        let l = (x * x + y * y + z * z).sqrt();
        if l > 1e-6 {
            out.extend_from_slice(&[x / l, y / l, z / l]);
        } else {
            out.extend_from_slice(&[0.0, 0.0, 1.0]);
        }
    };
    match img {
        DynamicImage::ImageRgb8(b) => {
            for p in b.as_raw().chunks_exact(3) {
                push(
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                );
            }
        }
        DynamicImage::ImageRgba8(b) => {
            for p in b.as_raw().chunks_exact(4) {
                push(
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                );
            }
        }
        other => {
            for p in other.to_rgb32f().as_raw().chunks_exact(3) {
                push(p[0], p[1], p[2]);
            }
        }
    }
    out
}

/// Normal map image -> GL-convention RGBA8 chain + variance pyramid.
pub fn normal_from_image(
    img: &DynamicImage,
    convention: NormalConvention,
) -> (TextureData, NormalVariance) {
    let v = decode_normals(img, convention);
    let (chain, var) = mips::build_chain_normal(&v, img.width(), img.height());
    (
        TextureData {
            format: PixelFormat::Rgba8Unorm,
            mips: chain,
        },
        var,
    )
}

/// Widen the roughness stored in channel `channel` of an RGBA8 (or R8, then
/// `channel` must be 0) texture according to a normal map's variance pyramid.
/// Returns how many mip levels were adjusted (0 if sizes do not match).
pub fn apply_toksvig(roughness: &mut TextureData, channel: usize, var: &NormalVariance) -> usize {
    let ch = match roughness.format {
        PixelFormat::Rgba8Unorm | PixelFormat::Rgba8UnormSrgb => 4,
        PixelFormat::R8Unorm => 1,
        _ => return 0,
    };
    if channel >= ch {
        return 0;
    }
    mips::apply_toksvig_u8(&mut roughness.mips, ch, channel, var)
}

/// Average normal of level 0 (GL convention, as decoded from the stored
/// RGBA8). Useful for sanity checks: tiling normal maps average to ~+Z.
pub fn average_normal(tex: &TextureData) -> [f32; 3] {
    let lvl = tex.mips.last().expect("texture has mips");
    let p = &lvl.data;
    [
        p[0] as f32 / 127.5 - 1.0,
        p[1] as f32 / 127.5 - 1.0,
        p[2] as f32 / 127.5 - 1.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn dx_normals_flip_green() {
        let mut img = RgbImage::new(2, 2);
        for p in img.pixels_mut() {
            *p = Rgb([128, 200, 230]); // +Y in the stored convention
        }
        let dynimg = DynamicImage::ImageRgb8(img);
        let gl = decode_normals(&dynimg, NormalConvention::Gl);
        let dx = decode_normals(&dynimg, NormalConvention::Dx);
        assert!(gl[1] > 0.3);
        assert!((dx[1] + gl[1]).abs() < 1e-6);
        let (tex, _) = normal_from_image(&dynimg, NormalConvention::Dx);
        assert!(tex.mips[0].data[1] < 128);
        tex.validate().unwrap();
    }

    #[test]
    fn color_chain_validates() {
        let t = TextureData::from_rgba8(5, 3, vec![200; 5 * 3 * 4], true);
        t.validate().unwrap();
        assert_eq!(t.mip_count(), 3);
        // constant colour stays constant through sRGB-correct filtering
        assert!(t
            .mips
            .iter()
            .all(|m| m.data.iter().all(|v| v.abs_diff(200) <= 1)));
    }
}
