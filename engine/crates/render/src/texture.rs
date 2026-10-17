//! CPU mip generation and texture upload.

use crate::types::{ImageData, ImageFormat};

fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5) as u8
}

/// 2x2 box downsample of one level (gamma-correct for sRGB colour channels).
pub(crate) fn downsample_level(
    src: &[u8],
    w: u32,
    h: u32,
    format: ImageFormat,
    srgb: bool,
) -> Vec<u8> {
    let nw = (w / 2).max(1);
    let nh = (h / 2).max(1);
    let bpp = format.bytes_per_pixel();
    let mut out = vec![0u8; (nw * nh) as usize * bpp];
    let lut: Vec<f32> = (0..256).map(|i| srgb_to_linear(i as u8)).collect();
    for y in 0..nh {
        for x in 0..nw {
            let xs = [(2 * x).min(w - 1), (2 * x + 1).min(w - 1)];
            let ys = [(2 * y).min(h - 1), (2 * y + 1).min(h - 1)];
            let di = (y * nw + x) as usize * bpp;
            match format {
                ImageFormat::Rgba16F => {
                    for c in 0..4 {
                        let mut s = 0.0f32;
                        for &yy in &ys {
                            for &xx in &xs {
                                let i = (yy * w + xx) as usize * 8 + c * 2;
                                s += half::f16::from_le_bytes([src[i], src[i + 1]]).to_f32();
                            }
                        }
                        let b = half::f16::from_f32(s * 0.25).to_le_bytes();
                        out[di + c * 2] = b[0];
                        out[di + c * 2 + 1] = b[1];
                    }
                }
                _ => {
                    for c in 0..bpp {
                        let is_color = srgb && format == ImageFormat::Rgba8 && c < 3;
                        let mut s = 0.0f32;
                        for &yy in &ys {
                            for &xx in &xs {
                                let v = src[(yy * w + xx) as usize * bpp + c];
                                s += if is_color { lut[v as usize] } else { v as f32 };
                            }
                        }
                        s *= 0.25;
                        out[di + c] = if is_color {
                            linear_to_srgb(s)
                        } else {
                            (s + 0.5) as u8
                        };
                    }
                }
            }
        }
    }
    out
}

pub(crate) fn mip_count(w: u32, h: u32) -> u32 {
    32 - w.max(h).max(1).leading_zeros()
}

/// Returns the full mip chain (generating missing levels).
pub(crate) fn full_chain(img: &ImageData) -> Vec<Vec<u8>> {
    let total = mip_count(img.width, img.height) as usize;
    let mut mips = img.mips.clone();
    mips.truncate(total);
    let mut w = img.width;
    let mut h = img.height;
    for _ in 1..mips.len() {
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    while mips.len() < total {
        let next = downsample_level(mips.last().unwrap(), w, h, img.format, img.srgb);
        mips.push(next);
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    mips
}

pub(crate) fn gpu_format(img: &ImageData) -> wgpu::TextureFormat {
    match img.format {
        ImageFormat::R8 => wgpu::TextureFormat::R8Unorm,
        ImageFormat::Rg8 => wgpu::TextureFormat::Rg8Unorm,
        ImageFormat::Rgba8 => {
            if img.srgb {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            }
        }
        ImageFormat::Rgba16F => wgpu::TextureFormat::Rgba16Float,
    }
}

pub(crate) fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    img: &ImageData,
) -> wgpu::Texture {
    let mips = full_chain(img);
    let format = gpu_format(img);
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: img.width,
            height: img.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: mips.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let bpp = img.format.bytes_per_pixel() as u32;
    let mut w = img.width;
    let mut h = img.height;
    for (level, data) in mips.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * bpp),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    tex
}

/// Converts an equirect HDR image to RGBA16F, box-filtering it down to at most `max_w` wide.
pub(crate) fn hdr_to_rgba16(
    rgb: &[f32],
    mut w: u32,
    mut h: u32,
    max_w: u32,
) -> (Vec<u8>, u32, u32) {
    let mut data: Vec<[f32; 3]> = rgb.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
    while w > max_w && w > 1 && h > 1 {
        let nw = w / 2;
        let nh = h / 2;
        let mut nd = vec![[0.0f32; 3]; (nw * nh) as usize];
        for y in 0..nh {
            for x in 0..nw {
                let mut s = [0.0f32; 3];
                for dy in 0..2 {
                    for dx in 0..2 {
                        let p = data[((2 * y + dy) * w + 2 * x + dx) as usize];
                        for c in 0..3 {
                            s[c] += if p[c].is_finite() { p[c] } else { 0.0 };
                        }
                    }
                }
                nd[(y * nw + x) as usize] = [s[0] * 0.25, s[1] * 0.25, s[2] * 0.25];
            }
        }
        data = nd;
        w = nw;
        h = nh;
    }
    let mut out = Vec::with_capacity(data.len() * 8);
    for p in data {
        for c in p {
            let v = if c.is_finite() {
                c.clamp(0.0, 60000.0)
            } else {
                0.0
            };
            out.extend_from_slice(&half::f16::from_f32(v).to_le_bytes());
        }
        out.extend_from_slice(&half::f16::from_f32(1.0).to_le_bytes());
    }
    (out, w, h)
}
