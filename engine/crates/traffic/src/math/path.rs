//! Arc-length parameterised polylines and geometry helpers.

/// A 2D point or vector (metres; x east, y north).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }
}

/// Position and heading (radians, CCW from +x) at an arc length.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PathSample {
    pub x: f64,
    pub y: f64,
    pub heading: f64,
}

/// Projection of a point onto a path.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Projection {
    /// Arc length of the closest point.
    pub s: f64,
    /// Signed lateral offset (positive = left of the travel direction).
    pub lateral: f64,
    /// Squared distance to the closest point.
    pub d2: f64,
}

/// A polyline parameterised by arc length. All lane and connector geometry is
/// stored this way: sampling is a binary search plus a lerp.
#[derive(Clone, Debug, Default)]
pub struct Path {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    /// Cumulative length at each vertex.
    pub cum: Vec<f64>,
    pub length: f64,
}

impl Path {
    pub fn new(points: &[Vec2]) -> Self {
        let mut xs: Vec<f64> = Vec::with_capacity(points.len());
        let mut ys: Vec<f64> = Vec::with_capacity(points.len());
        let mut cum = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                let dx = p.x - xs[xs.len() - 1];
                let dy = p.y - ys[ys.len() - 1];
                let d = (dx * dx + dy * dy).sqrt();
                if d < 1e-6 {
                    continue;
                }
                total += d;
            }
            xs.push(p.x);
            ys.push(p.y);
            cum.push(total);
        }
        if xs.len() == 1 {
            // Degenerate path: duplicate the point so sampling never breaks.
            xs.push(xs[0] + 1e-3);
            ys.push(ys[0]);
            cum.push(1e-3);
            total = 1e-3;
        }
        Path {
            xs,
            ys,
            cum,
            length: total,
        }
    }

    /// Index i such that cum[i] <= s < cum[i+1].
    fn seg(&self, s: f64) -> usize {
        let cum = &self.cum;
        let mut lo = 0usize;
        let mut hi = cum.len() - 2;
        if s <= 0.0 {
            return 0;
        }
        if s >= self.length {
            return hi;
        }
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if cum[mid] <= s {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo
    }

    /// Point and heading at arc length `s` (extrapolates before the start).
    pub fn sample(&self, s: f64) -> PathSample {
        let i = self.seg(s);
        let x0 = self.xs[i];
        let y0 = self.ys[i];
        let x1 = self.xs[i + 1];
        let y1 = self.ys[i + 1];
        let seg_len = self.cum[i + 1] - self.cum[i];
        let mut t = if seg_len > 0.0 {
            (s - self.cum[i]) / seg_len
        } else {
            0.0
        };
        if i == 0 && t < 0.0 {
            t = (s - self.cum[0]) / seg_len;
        }
        PathSample {
            x: x0 + (x1 - x0) * t,
            y: y0 + (y1 - y0) * t,
            heading: (y1 - y0).atan2(x1 - x0),
        }
    }

    pub fn heading_at(&self, s: f64) -> f64 {
        let i = self.seg(s);
        (self.ys[i + 1] - self.ys[i]).atan2(self.xs[i + 1] - self.xs[i])
    }

    /// Curvature estimate (1/m) around s, from heading change over a window.
    pub fn curvature_at(&self, s: f64, window: f64) -> f64 {
        let a = self.heading_at(s - window);
        let b = self.heading_at(s + window);
        let mut d = b - a;
        while d > std::f64::consts::PI {
            d -= std::f64::consts::PI * 2.0;
        }
        while d < -std::f64::consts::PI {
            d += std::f64::consts::PI * 2.0;
        }
        d / (2.0 * window)
    }

    /// Project a point onto the path.
    pub fn project(&self, px: f64, py: f64) -> Projection {
        let mut best = f64::INFINITY;
        let mut best_s = 0.0;
        let mut best_lat = 0.0;
        let n = self.xs.len();
        for i in 0..n - 1 {
            let x0 = self.xs[i];
            let y0 = self.ys[i];
            let dx = self.xs[i + 1] - x0;
            let dy = self.ys[i + 1] - y0;
            let l2 = dx * dx + dy * dy;
            let mut t = if l2 > 0.0 {
                ((px - x0) * dx + (py - y0) * dy) / l2
            } else {
                0.0
            };
            t = t.clamp(0.0, 1.0);
            let cx = x0 + dx * t;
            let cy = y0 + dy * t;
            let ex = px - cx;
            let ey = py - cy;
            let d2 = ex * ex + ey * ey;
            if d2 < best {
                best = d2;
                best_s = self.cum[i] + t * (self.cum[i + 1] - self.cum[i]);
                let l = l2.sqrt();
                best_lat = if l > 0.0 {
                    (dx * ey - dy * ex) / l
                } else {
                    0.0
                };
            }
        }
        Projection {
            s: best_s,
            lateral: best_lat,
            d2: best,
        }
    }

    pub fn points(&self) -> Vec<Vec2> {
        self.xs
            .iter()
            .zip(&self.ys)
            .map(|(&x, &y)| Vec2::new(x, y))
            .collect()
    }

    /// Points of the sub-range [s0, s1] (for trimmed geometry and rendering).
    pub fn slice(&self, s0: f64, s1: f64) -> Vec<Vec2> {
        let mut r = Vec::new();
        let a = self.sample(s0);
        r.push(Vec2::new(a.x, a.y));
        for i in 0..self.cum.len() {
            if self.cum[i] > s0 && self.cum[i] < s1 {
                r.push(Vec2::new(self.xs[i], self.ys[i]));
            }
        }
        let b = self.sample(s1);
        r.push(Vec2::new(b.x, b.y));
        r
    }
}

/// Offset a polyline sideways by `offset` metres (positive = left), with
/// miter correction so parallel offsets keep a constant width at corners.
pub fn offset_polyline(points: &[Vec2], offset: f64) -> Vec<Vec2> {
    let n = points.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut nx = 0.0;
        let mut ny = 0.0;
        if i > 0 {
            let dx = points[i].x - points[i - 1].x;
            let dy = points[i].y - points[i - 1].y;
            let l = (dx * dx + dy * dy).sqrt();
            if l > 0.0 {
                nx += -dy / l;
                ny += dx / l;
            }
        }
        if i < n - 1 {
            let dx = points[i + 1].x - points[i].x;
            let dy = points[i + 1].y - points[i].y;
            let l = (dx * dx + dy * dy).sqrt();
            if l > 0.0 {
                nx += -dy / l;
                ny += dx / l;
            }
        }
        let nl = (nx * nx + ny * ny).sqrt();
        if nl > 0.0 {
            nx /= nl;
            ny /= nl;
        }
        let mut k = 1.0;
        if i > 0 && i < n - 1 {
            let dx = points[i + 1].x - points[i].x;
            let dy = points[i + 1].y - points[i].y;
            let l = (dx * dx + dy * dy).sqrt();
            if l > 0.0 {
                let c = (nx * -dy + ny * dx) / l;
                if c > 0.3 {
                    k = 1.0 / c;
                }
            }
        }
        out.push(Vec2::new(
            points[i].x + nx * offset * k,
            points[i].y + ny * offset * k,
        ));
    }
    out
}

/// Cubic Bezier flattened to `steps + 1` points.
pub fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, steps: usize) -> Vec<Vec2> {
    (0..=steps)
        .map(|i| {
            let t = i as f64 / steps as f64;
            let u = 1.0 - t;
            let a = u * u * u;
            let b = 3.0 * u * u * t;
            let c = 3.0 * u * t * t;
            let d = t * t * t;
            Vec2::new(
                a * p0.x + b * p1.x + c * p2.x + d * p3.x,
                a * p0.y + b * p1.y + c * p2.y + d * p3.y,
            )
        })
        .collect()
}

/// Intersection of segments p1-p2 and p3-p4 as parameters `(t, u)` on each.
#[allow(clippy::too_many_arguments)]
pub fn segment_intersect(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    x4: f64,
    y4: f64,
) -> Option<(f64, f64)> {
    let d = (x2 - x1) * (y4 - y3) - (y2 - y1) * (x4 - x3);
    if d.abs() < 1e-12 {
        return None;
    }
    let t = ((x3 - x1) * (y4 - y3) - (y3 - y1) * (x4 - x3)) / d;
    let u = ((x3 - x1) * (y2 - y1) - (y3 - y1) * (x2 - x1)) / d;
    if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
        return None;
    }
    Some((t, u))
}

/// First crossing between two paths, as arc lengths `(sa, sb)`.
pub fn path_crossing(a: &Path, b: &Path) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None;
    for i in 0..a.xs.len() - 1 {
        for j in 0..b.xs.len() - 1 {
            if let Some((t, u)) = segment_intersect(
                a.xs[i],
                a.ys[i],
                a.xs[i + 1],
                a.ys[i + 1],
                b.xs[j],
                b.ys[j],
                b.xs[j + 1],
                b.ys[j + 1],
            ) {
                let sa = a.cum[i] + t * (a.cum[i + 1] - a.cum[i]);
                let sb = b.cum[j] + u * (b.cum[j + 1] - b.cum[j]);
                if best.is_none_or(|bst| sa < bst.0) {
                    best = Some((sa, sb));
                }
            }
        }
    }
    best
}

/// Minimum distance between two paths (sampled every `step` metres on `a`):
/// `(sa, sb, d)`.
pub fn path_closest(a: &Path, b: &Path, step: f64) -> (f64, f64, f64) {
    let mut best = (0.0, 0.0, f64::INFINITY);
    let n = ((a.length / step).ceil() as usize).max(2);
    for i in 0..=n {
        let s = (a.length * i as f64) / n as f64;
        let ps = a.sample(s);
        let pr = b.project(ps.x, ps.y);
        let d = pr.d2.sqrt();
        if d < best.2 {
            best = (s, pr.s, d);
        }
    }
    best
}
