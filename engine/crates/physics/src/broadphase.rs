//! Dynamic AABB tree (incremental, SAH insertion with AVL-style rotations), after Box2D's b2DynamicTree.

use crate::aabb::Aabb;
use glam::Vec3;

const NULL: u32 = u32::MAX;

#[derive(Clone, Copy, Debug)]
struct Node {
    aabb: Aabb,
    parent: u32,
    child1: u32,
    child2: u32,
    height: i32,
    data: u32,
}

impl Node {
    #[inline]
    fn is_leaf(&self) -> bool {
        self.child1 == NULL
    }
}

#[derive(Clone, Debug, Default)]
pub struct DynamicTree {
    nodes: Vec<Node>,
    root: u32,
    free: Vec<u32>,
    count: usize,
}

impl DynamicTree {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), root: NULL, free: Vec::new(), count: 0 }
    }

    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn alloc(&mut self) -> u32 {
        let n = Node { aabb: Aabb::EMPTY, parent: NULL, child1: NULL, child2: NULL, height: 0, data: 0 };
        if let Some(i) = self.free.pop() {
            self.nodes[i as usize] = n;
            i
        } else {
            self.nodes.push(n);
            (self.nodes.len() - 1) as u32
        }
    }

    /// Insert a proxy with the given (already fattened) bounds. Returns its id.
    pub fn insert(&mut self, aabb: Aabb, data: u32) -> u32 {
        let leaf = self.alloc();
        self.nodes[leaf as usize].aabb = aabb;
        self.nodes[leaf as usize].data = data;
        self.insert_leaf(leaf);
        self.count += 1;
        leaf
    }

    pub fn remove(&mut self, proxy: u32) {
        self.remove_leaf(proxy);
        self.free.push(proxy);
        self.count -= 1;
    }

    #[inline]
    pub fn fat_aabb(&self, proxy: u32) -> Aabb {
        self.nodes[proxy as usize].aabb
    }
    #[inline]
    pub fn data(&self, proxy: u32) -> u32 {
        self.nodes[proxy as usize].data
    }

    /// Re-insert with new bounds.
    pub fn update(&mut self, proxy: u32, aabb: Aabb) {
        self.remove_leaf(proxy);
        self.nodes[proxy as usize].aabb = aabb;
        self.insert_leaf(proxy);
    }

    pub fn query(&self, aabb: &Aabb, mut f: impl FnMut(u32) -> bool) {
        if self.root == NULL {
            return;
        }
        let mut stack: smallvec::SmallVec<[u32; 64]> = smallvec::smallvec![self.root];
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i as usize];
            if !n.aabb.overlaps(aabb) {
                continue;
            }
            if n.is_leaf() {
                if !f(i) {
                    return;
                }
            } else {
                stack.push(n.child1);
                stack.push(n.child2);
            }
        }
    }

    /// `f(proxy, t_max) -> new t_max` (return a negative value to stop).
    pub fn ray_cast(&self, origin: Vec3, dir: Vec3, max_t: f32, mut f: impl FnMut(u32, f32) -> f32) {
        if self.root == NULL {
            return;
        }
        let inv = Vec3::ONE / dir;
        let mut t_max = max_t;
        let mut stack: smallvec::SmallVec<[u32; 64]> = smallvec::smallvec![self.root];
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i as usize];
            if n.aabb.ray_hit(origin, inv, t_max).is_none() {
                continue;
            }
            if n.is_leaf() {
                let t = f(i, t_max);
                if t < 0.0 {
                    return;
                }
                t_max = t;
            } else {
                stack.push(n.child1);
                stack.push(n.child2);
            }
        }
    }

    fn insert_leaf(&mut self, leaf: u32) {
        if self.root == NULL {
            self.root = leaf;
            self.nodes[leaf as usize].parent = NULL;
            return;
        }
        // Find the best sibling (branch and bound on SAH cost, greedy descent).
        let leaf_aabb = self.nodes[leaf as usize].aabb;
        let mut index = self.root;
        while !self.nodes[index as usize].is_leaf() {
            let n = self.nodes[index as usize];
            let area = n.aabb.area();
            let combined = n.aabb.union(&leaf_aabb).area();
            let cost = 2.0 * combined;
            let inherit = 2.0 * (combined - area);
            let child_cost = |c: u32| {
                let ca = self.nodes[c as usize].aabb;
                let u = ca.union(&leaf_aabb).area();
                if self.nodes[c as usize].is_leaf() {
                    u + inherit
                } else {
                    u - ca.area() + inherit
                }
            };
            let c1 = child_cost(n.child1);
            let c2 = child_cost(n.child2);
            if cost < c1 && cost < c2 {
                break;
            }
            index = if c1 < c2 { n.child1 } else { n.child2 };
        }
        let sibling = index;
        let old_parent = self.nodes[sibling as usize].parent;
        let new_parent = self.alloc();
        {
            let sib_aabb = self.nodes[sibling as usize].aabb;
            let h = self.nodes[sibling as usize].height;
            let np = &mut self.nodes[new_parent as usize];
            np.parent = old_parent;
            np.aabb = leaf_aabb.union(&sib_aabb);
            np.height = h + 1;
            np.child1 = sibling;
            np.child2 = leaf;
        }
        if old_parent != NULL {
            let op = &mut self.nodes[old_parent as usize];
            if op.child1 == sibling {
                op.child1 = new_parent;
            } else {
                op.child2 = new_parent;
            }
        } else {
            self.root = new_parent;
        }
        self.nodes[sibling as usize].parent = new_parent;
        self.nodes[leaf as usize].parent = new_parent;
        self.refit_from(self.nodes[leaf as usize].parent);
    }

    fn remove_leaf(&mut self, leaf: u32) {
        if leaf == self.root {
            self.root = NULL;
            return;
        }
        let parent = self.nodes[leaf as usize].parent;
        let grand = self.nodes[parent as usize].parent;
        let sibling = if self.nodes[parent as usize].child1 == leaf {
            self.nodes[parent as usize].child2
        } else {
            self.nodes[parent as usize].child1
        };
        if grand != NULL {
            let g = &mut self.nodes[grand as usize];
            if g.child1 == parent {
                g.child1 = sibling;
            } else {
                g.child2 = sibling;
            }
            self.nodes[sibling as usize].parent = grand;
            self.free.push(parent);
            self.refit_from(grand);
        } else {
            self.root = sibling;
            self.nodes[sibling as usize].parent = NULL;
            self.free.push(parent);
        }
    }

    fn refit_from(&mut self, mut index: u32) {
        while index != NULL {
            index = self.balance(index);
            let n = self.nodes[index as usize];
            let (a, b) = (self.nodes[n.child1 as usize], self.nodes[n.child2 as usize]);
            let node = &mut self.nodes[index as usize];
            node.height = 1 + a.height.max(b.height);
            node.aabb = a.aabb.union(&b.aabb);
            index = node.parent;
        }
    }

    /// AVL rotation if the subtree at `a` is imbalanced. Returns the new subtree root.
    fn balance(&mut self, a: u32) -> u32 {
        let na = self.nodes[a as usize];
        if na.is_leaf() || na.height < 2 {
            return a;
        }
        let (b, c) = (na.child1, na.child2);
        let balance = self.nodes[c as usize].height - self.nodes[b as usize].height;
        if balance > 1 {
            self.rotate(a, c, b)
        } else if balance < -1 {
            self.rotate(a, b, c)
        } else {
            a
        }
    }

    /// Promote `up` (a child of `a`) above `a`; `other` is a's other child.
    fn rotate(&mut self, a: u32, up: u32, other: u32) -> u32 {
        let nu = self.nodes[up as usize];
        let (f, g) = (nu.child1, nu.child2);
        // up takes a's place.
        let a_parent = self.nodes[a as usize].parent;
        self.nodes[up as usize].child1 = a;
        self.nodes[up as usize].parent = a_parent;
        self.nodes[a as usize].parent = up;
        if a_parent != NULL {
            let p = &mut self.nodes[a_parent as usize];
            if p.child1 == a {
                p.child1 = up;
            } else {
                p.child2 = up;
            }
        } else {
            self.root = up;
        }
        // Keep the taller grandchild under `up`, give the shorter to `a`.
        let (keep, give) = if self.nodes[f as usize].height > self.nodes[g as usize].height { (f, g) } else { (g, f) };
        self.nodes[up as usize].child2 = keep;
        let a_is_child1 = self.nodes[a as usize].child1 == up;
        if a_is_child1 {
            self.nodes[a as usize].child1 = give;
        } else {
            self.nodes[a as usize].child2 = give;
        }
        self.nodes[give as usize].parent = a;
        let (o, gi) = (self.nodes[other as usize], self.nodes[give as usize]);
        {
            let na = &mut self.nodes[a as usize];
            na.aabb = o.aabb.union(&gi.aabb);
            na.height = 1 + o.height.max(gi.height);
        }
        let (na, nk) = (self.nodes[a as usize], self.nodes[keep as usize]);
        let nu = &mut self.nodes[up as usize];
        nu.aabb = na.aabb.union(&nk.aabb);
        nu.height = 1 + na.height.max(nk.height);
        up
    }

    /// Debug check of structural invariants.
    pub fn validate(&self) -> usize {
        fn walk(t: &DynamicTree, i: u32, parent: u32) -> (usize, i32) {
            let n = &t.nodes[i as usize];
            assert_eq!(n.parent, parent);
            if n.is_leaf() {
                return (1, 0);
            }
            let (l1, h1) = walk(t, n.child1, i);
            let (l2, h2) = walk(t, n.child2, i);
            assert_eq!(n.height, 1 + h1.max(h2));
            let u = t.nodes[n.child1 as usize].aabb.union(&t.nodes[n.child2 as usize].aabb);
            assert!(n.aabb.contains(&u));
            (l1 + l2, n.height)
        }
        if self.root == NULL {
            return 0;
        }
        walk(self, self.root, NULL).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_insert_remove_query() {
        let mut t = DynamicTree::new();
        let mut ids = Vec::new();
        for i in 0..200 {
            let x = (i % 20) as f32 * 2.0;
            let z = (i / 20) as f32 * 2.0;
            ids.push(t.insert(Aabb::from_center_half(Vec3::new(x, 0.0, z), Vec3::splat(0.6)), i));
        }
        assert_eq!(t.validate(), 200);
        let mut hits = 0;
        t.query(&Aabb::from_center_half(Vec3::new(4.0, 0.0, 4.0), Vec3::splat(0.1)), |_| {
            hits += 1;
            true
        });
        assert_eq!(hits, 1);
        for (k, id) in ids.iter().enumerate() {
            if k % 3 == 0 {
                t.remove(*id);
            } else {
                t.update(*id, Aabb::from_center_half(Vec3::new(k as f32, 5.0, 0.0), Vec3::splat(0.4)));
            }
        }
        assert_eq!(t.validate(), 200 - 67);
    }
}
