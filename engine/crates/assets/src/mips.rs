//! CPU mip-chain generation.
//!
//! All filters are area-weighted box filters that also handle odd and
//! non-power-of-two sizes (each destination texel averages the source texels
//! its footprint touches). Each level is produced from the previous one.
//!
//! * Colour (sRGB) data is decoded to linear light before averaging and
//!   re-encoded afterwards; alpha is averaged linearly.
//! * Normal maps are averaged as vectors and renormalised per level. The
//!   *unnormalised* average length is kept in a [`NormalVariance`] pyramid so
//!   roughness mips can be widened (Toksvig / vMF specular anti-aliasing).
//! * Float data (HDRIs) is averaged as-is.

use std::sync::OnceLock;

/// One mip level of a texture: tightly packed rows, no padding.
#[derive(Debug, Clone, PartialEq)]
pub struct MipLevel {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Number of levels in a full chain down to 1x1.
pub fn full_mip_count(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

/// Size of the next (smaller) level.
pub fn next_size(width: u32, height: u32) -> (u32, u32) {
    ((width / 2).max(1), (height / 2).max(1))
}

#[inline]
fn footprint(d: u32, dst: u32, src: u32) -> (u32, u32) {
    let a = (d as u64 * src as u64 / dst as u64) as u32;
    let b = ((d as u64 + 1) * src as u64).div_ceil(dst as u64) as u32;
    (a.min(src - 1), b.clamp(a + 1, src))
}

/// Generic box downsample of an interleaved f32 image with `ch` channels.
pub fn downsample_f32(src: &[f32], w: u32, h: u32, ch: usize) -> (Vec<f32>, u32, u32) {
    let (dw, dh) = next_size(w, h);
    let mut out = vec![0.0f32; dw as usize * dh as usize * ch];
    let mut acc = [0.0f32; 4];
    for dy in 0..dh {
        let (y0, y1) = footprint(dy, dh, h);
        for dx in 0..dw {
            let (x0, x1) = footprint(dx, dw, w);
            acc[..ch].fill(0.0);
            for y in y0..y1 {
                let row = (y as usize * w as usize) * ch;
                for x in x0..x1 {
                    let p = row + x as usize * ch;
                    for c in 0..ch {
                        acc[c] += src[p + c];
                    }
                }
            }
            let inv = 1.0 / ((y1 - y0) * (x1 - x0)) as f32;
            let o = (dy as usize * dw as usize + dx as usize) * ch;
            for c in 0..ch {
                out[o + c] = acc[c] * inv;
            }
        }
    }
    (out, dw, dh)
}

fn srgb_to_linear_lut() -> &'static [f32; 256] {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut t = [0.0f32; 256];
        for (i, v) in t.iter_mut().enumerate() {
            *v = srgb_to_linear(i as f32 / 255.0);
        }
        t
    })
}

/// sRGB transfer function, decode.
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// sRGB transfer function, encode.
pub fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

const ENC_LUT_SIZE: usize = 16384;

fn linear_to_srgb8_lut() -> &'static [u8] {
    static LUT: OnceLock<Vec<u8>> = OnceLock::new();
    LUT.get_or_init(|| {
        (0..ENC_LUT_SIZE)
            .map(|i| {
                let l = i as f32 / (ENC_LUT_SIZE - 1) as f32;
                (linear_to_srgb(l) * 255.0 + 0.5) as u8
            })
            .collect()
    })
}

#[inline]
fn enc_srgb8(lut: &[u8], l: f32) -> u8 {
    lut[(l.clamp(0.0, 1.0) * (ENC_LUT_SIZE - 1) as f32 + 0.5) as usize]
}

#[inline]
fn unorm8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Downsample one 8-bit level with `ch` channels. When `srgb` is set the
/// first three channels are filtered in linear light (alpha stays linear).
pub fn downsample_u8(src: &[u8], w: u32, h: u32, ch: usize, srgb: bool) -> MipLevel {
    let (dw, dh) = next_size(w, h);
    let dec = srgb_to_linear_lut();
    let enc = linear_to_srgb8_lut();
    let mut out = vec![0u8; dw as usize * dh as usize * ch];
    let mut acc = [0.0f32; 4];
    for dy in 0..dh {
        let (y0, y1) = footprint(dy, dh, h);
        for dx in 0..dw {
            let (x0, x1) = footprint(dx, dw, w);
            acc[..ch].fill(0.0);
            for y in y0..y1 {
                let row = (y as usize * w as usize) * ch;
                for x in x0..x1 {
                    let p = row + x as usize * ch;
                    for c in 0..ch {
                        let v = src[p + c];
                        acc[c] += if srgb && c < 3 {
                            dec[v as usize]
                        } else {
                            v as f32 * (1.0 / 255.0)
                        };
                    }
                }
            }
            let inv = 1.0 / ((y1 - y0) * (x1 - x0)) as f32;
            let o = (dy as usize * dw as usize + dx as usize) * ch;
            for c in 0..ch {
                let v = acc[c] * inv;
                out[o + c] = if srgb && c < 3 {
                    enc_srgb8(enc, v)
                } else {
                    unorm8(v)
                };
            }
        }
    }
    MipLevel {
        width: dw,
        height: dh,
        data: out,
    }
}

/// Downsample one 16-bit single/multi-channel level (native-endian u16 in bytes).
pub fn downsample_u16(src: &[u8], w: u32, h: u32, ch: usize) -> MipLevel {
    let vals: Vec<f32> = src
        .chunks_exact(2)
        .map(|b| u16::from_ne_bytes([b[0], b[1]]) as f32 / 65535.0)
        .collect();
    let (d, dw, dh) = downsample_f32(&vals, w, h, ch);
    let mut out = Vec::with_capacity(d.len() * 2);
    for v in d {
        out.extend_from_slice(&((v.clamp(0.0, 1.0) * 65535.0 + 0.5) as u16).to_ne_bytes());
    }
    MipLevel {
        width: dw,
        height: dh,
        data: out,
    }
}

/// Build the full chain (level 0 included) for 8-bit data.
pub fn build_chain_u8(level0: MipLevel, ch: usize, srgb: bool) -> Vec<MipLevel> {
    let n = full_mip_count(level0.width, level0.height) as usize;
    let mut v = Vec::with_capacity(n);
    v.push(level0);
    while v.len() < n {
        let p = v.last().expect("non-empty");
        let next = downsample_u8(&p.data, p.width, p.height, ch, srgb);
        v.push(next);
    }
    v
}

/// Build the full chain (level 0 included) for 16-bit unorm data.
pub fn build_chain_u16(level0: MipLevel, ch: usize) -> Vec<MipLevel> {
    let n = full_mip_count(level0.width, level0.height) as usize;
    let mut v = Vec::with_capacity(n);
    v.push(level0);
    while v.len() < n {
        let p = v.last().expect("non-empty");
        let next = downsample_u16(&p.data, p.width, p.height, ch);
        v.push(next);
    }
    v
}

/// Per-level average-normal lengths (1.0 = flat/coherent, < 1 = the normals
/// under that texel diverge). Level 0 is all ones and is not stored.
#[derive(Debug, Clone, Default)]
pub struct NormalVariance {
    /// `levels[i]` corresponds to mip `i + 1`.
    pub levels: Vec<(u32, u32, Vec<f32>)>,
}

impl NormalVariance {
    /// Lengths for mip `level` if its size is `w x h`.
    pub fn lengths(&self, level: usize, w: u32, h: u32) -> Option<&[f32]> {
        if level == 0 {
            return None;
        }
        self.levels
            .get(level - 1)
            .filter(|(lw, lh, _)| *lw == w && *lh == h)
            .map(|(_, _, v)| v.as_slice())
    }
}

#[inline]
fn encode_normal(n: [f32; 3]) -> [u8; 3] {
    [
        unorm8(n[0] * 0.5 + 0.5),
        unorm8(n[1] * 0.5 + 0.5),
        unorm8(n[2] * 0.5 + 0.5),
    ]
}

#[inline]
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 1e-8 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Build a normal-map chain from unit vectors (`level0` is xyz interleaved,
/// already in the OpenGL convention). Output levels are RGBA8 with A = 255.
/// Returns the chain plus the average-length pyramid for Toksvig.
pub fn build_chain_normal(
    level0: &[f32],
    width: u32,
    height: u32,
) -> (Vec<MipLevel>, NormalVariance) {
    let n = full_mip_count(width, height) as usize;
    let mut levels = Vec::with_capacity(n);
    let mut var = NormalVariance::default();
    let enc_level = |vecs: &[f32], w: u32, h: u32| -> MipLevel {
        let mut data = Vec::with_capacity(w as usize * h as usize * 4);
        for v in vecs.chunks_exact(3) {
            let e = encode_normal(normalize3([v[0], v[1], v[2]]));
            data.extend_from_slice(&[e[0], e[1], e[2], 255]);
        }
        MipLevel {
            width: w,
            height: h,
            data,
        }
    };
    levels.push(enc_level(level0, width, height));
    // `cur` holds unnormalised averages of the level-0 unit vectors, so its
    // length is the coherence of the footprint.
    let mut cur: Vec<f32> = level0.to_vec();
    let (mut w, mut h) = (width, height);
    while levels.len() < n {
        let (next, nw, nh) = downsample_f32(&cur, w, h, 3);
        let lens: Vec<f32> = next
            .chunks_exact(3)
            .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().min(1.0))
            .collect();
        levels.push(enc_level(&next, nw, nh));
        var.levels.push((nw, nh, lens));
        cur = next;
        w = nw;
        h = nh;
    }
    (levels, var)
}

/// Widen a perceptual roughness value given the average normal length `len`
/// of the footprint (vMF fit of the normal distribution, as in Toksvig /
/// "Crafting a Next-Gen Material Pipeline", Neubelt & Pettineo 2013):
/// `kappa = (3r - r^3) / (1 - r^2)`, `alpha'^2 = alpha^2 + 2 / kappa`.
pub fn toksvig_roughness(roughness: f32, len: f32) -> f32 {
    let r = len.clamp(0.0, 1.0);
    if r >= 0.9999 {
        return roughness;
    }
    let alpha = roughness * roughness;
    let kappa = if r <= 1e-4 {
        0.0
    } else {
        (3.0 * r - r * r * r) / (1.0 - r * r)
    };
    let alpha2 = if kappa <= 1e-6 {
        1.0
    } else {
        (alpha * alpha + 2.0 / kappa).min(1.0)
    };
    alpha2.sqrt().sqrt().max(roughness)
}

/// Apply [`toksvig_roughness`] in place to channel `channel` of an 8-bit
/// chain with `ch` channels, using the normal map's variance pyramid. Levels
/// whose size does not match the normal pyramid are left untouched.
/// Returns the number of levels adjusted.
pub fn apply_toksvig_u8(
    chain: &mut [MipLevel],
    ch: usize,
    channel: usize,
    var: &NormalVariance,
) -> usize {
    let mut n = 0;
    for (i, lvl) in chain.iter_mut().enumerate().skip(1) {
        let Some(lens) = var.lengths(i, lvl.width, lvl.height) else {
            continue;
        };
        for (px, len) in lvl.data.chunks_exact_mut(ch).zip(lens) {
            let r = px[channel] as f32 / 255.0;
            px[channel] = unorm8(toksvig_roughness(r, *len));
        }
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts() {
        assert_eq!(full_mip_count(2048, 2048), 12);
        assert_eq!(full_mip_count(1, 1), 1);
        assert_eq!(full_mip_count(300, 7), 9);
        assert_eq!(next_size(3, 1), (1, 1));
    }

    #[test]
    fn odd_sizes_average_everything() {
        // 3x1 -> 1x1 must average all three texels.
        let src = [0u8, 30, 60];
        let m = downsample_u8(&src, 3, 1, 1, false);
        assert_eq!((m.width, m.height), (1, 1));
        assert_eq!(m.data[0], 30);
    }

    #[test]
    fn srgb_mip_is_linear_average() {
        // black + white in sRGB averages to ~188 (linear 0.5), not 128.
        let src = [0u8, 0, 0, 255, 255, 255, 255, 255];
        let m = downsample_u8(&src, 2, 1, 4, true);
        assert!((186..=190).contains(&m.data[0]), "{}", m.data[0]);
        assert_eq!(m.data[3], 255);
    }

    #[test]
    fn normal_chain_renormalises_and_records_variance() {
        // Two opposing tilted normals average to a short vector.
        let a = normalize3([0.8, 0.0, 0.6]);
        let b = normalize3([-0.8, 0.0, 0.6]);
        let lvl0 = [a, b].concat();
        let (chain, var) = build_chain_normal(&lvl0, 2, 1);
        assert_eq!(chain.len(), 2);
        let px = &chain[1].data;
        // Renormalised to straight up.
        assert!(px[0].abs_diff(128) <= 1 && px[2] == 255, "{px:?}");
        let len = var.lengths(1, 1, 1).unwrap()[0];
        assert!((len - 0.6).abs() < 1e-3);
        assert!(toksvig_roughness(0.2, len) > 0.5);
        assert_eq!(toksvig_roughness(0.3, 1.0), 0.3);
    }
}
