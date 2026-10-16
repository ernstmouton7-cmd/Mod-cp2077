//! Static bounding volume hierarchy (used for triangle meshes).

use crate::aabb::Aabb;
use glam::Vec3;

#[derive(Clone, Copy, Debug)]
struct Node {
    aabb: Aabb,
    /// Leaf: first primitive index into `indices`; inner: index of the left child (right = left + 1).
    first: u32,
    /// Leaf primitive count, 0 for inner nodes.
    count: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Bvh {
    nodes: Vec<Node>,
    indices: Vec<u32>,
}

impl Bvh {
    pub fn build(boxes: &[Aabb]) -> Self {
        let mut bvh = Bvh {
            nodes: Vec::with_capacity(boxes.len() * 2),
            indices: (0..boxes.len() as u32).collect(),
        };
        if boxes.is_empty() {
            bvh.nodes.push(Node {
                aabb: Aabb::new(Vec3::ZERO, Vec3::ZERO),
                first: 0,
                count: 0,
            });
            return bvh;
        }
        let centers: Vec<Vec3> = boxes.iter().map(|b| b.center()).collect();
        bvh.nodes.push(Node {
            aabb: Aabb::EMPTY,
            first: 0,
            count: 0,
        });
        bvh.split(0, 0, boxes.len(), boxes, &centers);
        bvh
    }

    fn split(&mut self, node: usize, start: usize, end: usize, boxes: &[Aabb], centers: &[Vec3]) {
        let mut aabb = Aabb::EMPTY;
        let mut cb = Aabb::EMPTY;
        for &i in &self.indices[start..end] {
            aabb = aabb.union(&boxes[i as usize]);
            cb.grow(centers[i as usize]);
        }
        self.nodes[node].aabb = aabb;
        let n = end - start;
        if n <= 4 {
            self.nodes[node].first = start as u32;
            self.nodes[node].count = n as u32;
            return;
        }
        let ext = cb.max - cb.min;
        let axis = if ext.x >= ext.y && ext.x >= ext.z {
            0
        } else if ext.y >= ext.z {
            1
        } else {
            2
        };
        let mid = start + n / 2;
        self.indices[start..end].select_nth_unstable_by(n / 2, |a, b| {
            centers[*a as usize][axis].total_cmp(&centers[*b as usize][axis])
        });
        let left = self.nodes.len();
        self.nodes.push(Node {
            aabb: Aabb::EMPTY,
            first: 0,
            count: 0,
        });
        self.nodes.push(Node {
            aabb: Aabb::EMPTY,
            first: 0,
            count: 0,
        });
        self.nodes[node].first = left as u32;
        self.nodes[node].count = 0;
        self.split(left, start, mid, boxes, centers);
        self.split(left + 1, mid, end, boxes, centers);
    }

    pub fn root_aabb(&self) -> Aabb {
        self.nodes[0].aabb
    }

    /// Visit every primitive whose box overlaps `aabb`.
    pub fn query_aabb(&self, aabb: &Aabb, mut f: impl FnMut(usize)) {
        if self.indices.is_empty() {
            return;
        }
        let mut stack = [0u32; 64];
        let mut sp = 1;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            if !node.aabb.overlaps(aabb) {
                continue;
            }
            if node.count > 0 {
                for &i in &self.indices[node.first as usize..(node.first + node.count) as usize] {
                    f(i as usize);
                }
            } else {
                stack[sp] = node.first;
                stack[sp + 1] = node.first + 1;
                sp += 2;
            }
        }
    }

    /// Ray traversal; `f(prim, t_max)` returns the (possibly shortened) max distance.
    pub fn ray_cast(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_t: f32,
        f: &mut dyn FnMut(usize, f32) -> f32,
    ) {
        if self.indices.is_empty() {
            return;
        }
        let inv = Vec3::ONE / dir;
        let mut t_max = max_t;
        let mut stack = [0u32; 64];
        let mut sp = 1;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            if node.aabb.ray_hit(origin, inv, t_max).is_none() {
                continue;
            }
            if node.count > 0 {
                for &i in &self.indices[node.first as usize..(node.first + node.count) as usize] {
                    t_max = f(i as usize, t_max);
                }
            } else {
                stack[sp] = node.first;
                stack[sp + 1] = node.first + 1;
                sp += 2;
            }
        }
    }
}
