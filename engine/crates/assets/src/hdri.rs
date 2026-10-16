//! Equirectangular HDR environment maps (Radiance `.hdr` and OpenEXR).
//!
//! Pixels are linear scene-referred radiance (Rec.709 primaries, as published
//! by Poly Haven), stored as RGBA f32 with A = 1.
//!
//! Direction convention used by [`HdrImage::direction_of`] and
//! [`HdrImage::estimate_sun`]: right-handed, **+Y up**. Texture `u` runs from
//! 0 to 1 left to right, `v` from 0 (zenith) to 1 (nadir). The centre of the
//! image (`u = 0.5`) looks down **-Z**, `u = 0.75` looks down **+X**
//! (i.e. `dir = (sin(phi), cos(theta), -cos(phi))` with `phi = (u - 0.5) * 2pi`,
//! `theta = v * pi`). Sample with `u = 0.5 + atan2(d.x, -d.z) / 2pi`, `v = acos(d.y) / pi`.

use std::path::Path;

use half::f16;

use crate::mips;
use crate::AssetError;

/// A decoded equirectangular HDR image.
#[derive(Debug, Clone, PartialEq)]
pub struct HdrImage {
    pub width: u32,
    pub height: u32,
    /// RGBA f32, row-major, top row first.
    pub data: Vec<f32>,
}

/// Brightest-region estimate (the sun, usually) of an HDRI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunEstimate {
    /// Unit direction *towards* the light, in the convention described in the module docs.
    pub direction: [f32; 3],
    /// Average radiance of the selected texels (linear RGB).
    pub radiance: [f32; 3],
    /// Solid angle covered by the selected texels, steradians.
    pub solid_angle: f32,
    /// Irradiance contribution `radiance * solid_angle` (approximate, linear RGB).
    pub irradiance: [f32; 3],
}

impl HdrImage {
    /// Load an `.hdr` (Radiance RGBE) or `.exr` file.
    pub fn load(path: &Path) -> Result<Self, AssetError> {
        let img = crate::texture::open_image(path)?;
        let rgba = img.into_rgba32f();
        let (width, height) = rgba.dimensions();
        let mut data = rgba.into_raw();
        // EXR alpha may be absent/zero; environment maps are opaque.
        for px in data.chunks_exact_mut(4) {
            px[3] = 1.0;
        }
        Ok(HdrImage {
            width,
            height,
            data,
        })
    }

    /// Texel at (x, y), RGB.
    pub fn texel(&self, x: u32, y: u32) -> [f32; 3] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        [self.data[i], self.data[i + 1], self.data[i + 2]]
    }

    /// Convert to RGBA16F (e.g. for `Rgba16Float` GPU textures). Values above
    /// the f16 range (65504) are clamped rather than becoming infinity.
    pub fn to_rgba16f(&self) -> Vec<f16> {
        self.data
            .iter()
            .map(|v| f16::from_f32(v.clamp(-65504.0, 65504.0)))
            .collect()
    }

    /// Same as [`HdrImage::to_rgba16f`] but as raw native-endian bytes.
    pub fn to_rgba16f_bytes(&self) -> Vec<u8> {
        self.to_rgba16f()
            .iter()
            .flat_map(|h| h.to_bits().to_ne_bytes())
            .collect()
    }

    /// Box-filtered mip chain (level 0 = self), RGBA f32 per level.
    pub fn mip_chain(&self) -> Vec<HdrImage> {
        let n = mips::full_mip_count(self.width, self.height) as usize;
        let mut v = vec![self.clone()];
        while v.len() < n {
            let p = v.last().expect("non-empty");
            let (data, width, height) = mips::downsample_f32(&p.data, p.width, p.height, 4);
            v.push(HdrImage {
                width,
                height,
                data,
            });
        }
        v
    }

    /// Direction through the centre of texel (x, y).
    pub fn direction_of(&self, x: u32, y: u32) -> [f32; 3] {
        let u = (x as f32 + 0.5) / self.width as f32;
        let v = (y as f32 + 0.5) / self.height as f32;
        let phi = (u - 0.5) * std::f32::consts::TAU;
        let theta = v * std::f32::consts::PI;
        [
            theta.sin() * phi.sin(),
            theta.cos(),
            -theta.sin() * phi.cos(),
        ]
    }

    /// Average luminance weighted by solid angle (a rough "exposure" hint).
    pub fn average_luminance(&self) -> f32 {
        let (mut sum, mut wsum) = (0.0f64, 0.0f64);
        for y in 0..self.height {
            let w = (((y as f32 + 0.5) / self.height as f32) * std::f32::consts::PI).sin() as f64;
            for x in 0..self.width {
                let c = self.texel(x, y);
                sum += luminance(c) as f64 * w;
                wsum += w;
            }
        }
        if wsum > 0.0 {
            (sum / wsum) as f32
        } else {
            0.0
        }
    }

    /// Estimate the dominant light: the luminance-weighted centroid of the
    /// brightest 0.05 % of the sphere (by solid angle). Returns `None` for an
    /// image without a distinct peak (overcast skies: peak < 8x the average).
    pub fn estimate_sun(&self) -> Option<SunEstimate> {
        let (w, h) = (self.width, self.height);
        if w == 0 || h == 0 {
            return None;
        }
        let texel_sa = |y: u32| {
            let theta = ((y as f32 + 0.5) / h as f32) * std::f32::consts::PI;
            (std::f32::consts::TAU / w as f32) * (std::f32::consts::PI / h as f32) * theta.sin()
        };
        let mut lum: Vec<(f32, u32, u32)> = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                lum.push((luminance(self.texel(x, y)), x, y));
            }
        }
        lum.sort_by(|a, b| b.0.total_cmp(&a.0));
        let avg = self.average_luminance();
        if lum[0].0 < avg * 8.0 {
            return None;
        }
        let budget = 4.0 * std::f32::consts::PI * 0.0005;
        let (mut sa, mut dir, mut rad) = (0.0f32, [0.0f32; 3], [0.0f32; 3]);
        for &(l, x, y) in &lum {
            if sa >= budget {
                break;
            }
            let s = texel_sa(y);
            let d = self.direction_of(x, y);
            let c = self.texel(x, y);
            for i in 0..3 {
                dir[i] += d[i] * l * s;
                rad[i] += c[i] * s;
            }
            sa += s;
        }
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if len <= 0.0 || sa <= 0.0 {
            return None;
        }
        let radiance = [rad[0] / sa, rad[1] / sa, rad[2] / sa];
        Some(SunEstimate {
            direction: [dir[0] / len, dir[1] / len, dir[2] / len],
            radiance,
            solid_angle: sa,
            irradiance: [rad[0], rad[1], rad[2]],
        })
    }
}

/// Rec.709 luminance.
pub fn luminance(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_direction_convention() {
        let (w, h) = (64u32, 32u32);
        let mut data = vec![0.1f32; (w * h * 4) as usize];
        // Put a hot spot at u = 0.75 (+X), v = 0.25 (45 degrees above horizon).
        let (sx, sy) = (48u32, 8u32);
        let i = ((sy * w + sx) * 4) as usize;
        data[i..i + 3].copy_from_slice(&[1000.0, 1000.0, 1000.0]);
        let img = HdrImage {
            width: w,
            height: h,
            data,
        };
        let s = img.estimate_sun().unwrap();
        assert!(s.direction[0] > 0.6 && s.direction[1] > 0.6 && s.direction[2].abs() < 0.1);
        let chain = img.mip_chain();
        assert_eq!(chain.len(), 7);
        assert_eq!((chain[6].width, chain[6].height), (1, 1));
        assert_eq!(img.to_rgba16f().len(), (w * h * 4) as usize);
    }
}
