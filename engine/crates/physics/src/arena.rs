//! Generational arena: stable handles that detect use-after-free.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Handle {
    pub index: u32,
    pub generation: u32,
}

impl Handle {
    pub const INVALID: Handle = Handle { index: u32::MAX, generation: 0 };
}

#[derive(Clone, Debug)]
enum Slot<T> {
    Occupied(u32, T),
    Free(u32, u32),
}

#[derive(Clone, Debug)]
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free_head: u32,
    len: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self { slots: Vec::new(), free_head: u32::MAX, len: 0 }
    }
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Upper bound of slot indices (for side tables indexed by `Handle::index`).
    pub fn capacity_index(&self) -> usize {
        self.slots.len()
    }

    pub fn insert(&mut self, value: T) -> Handle {
        self.len += 1;
        if self.free_head != u32::MAX {
            let index = self.free_head;
            if let Slot::Free(generation, next) = self.slots[index as usize] {
                self.free_head = next;
                let generation = generation.wrapping_add(1);
                self.slots[index as usize] = Slot::Occupied(generation, value);
                return Handle { index, generation };
            }
            unreachable!()
        }
        self.slots.push(Slot::Occupied(0, value));
        Handle { index: (self.slots.len() - 1) as u32, generation: 0 }
    }

    pub fn remove(&mut self, h: Handle) -> Option<T> {
        let slot = self.slots.get_mut(h.index as usize)?;
        match slot {
            Slot::Occupied(g, _) if *g == h.generation => {
                let old = std::mem::replace(slot, Slot::Free(h.generation, self.free_head));
                self.free_head = h.index;
                self.len -= 1;
                match old {
                    Slot::Occupied(_, v) => Some(v),
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    #[inline]
    pub fn get(&self, h: Handle) -> Option<&T> {
        match self.slots.get(h.index as usize) {
            Some(Slot::Occupied(g, v)) if *g == h.generation => Some(v),
            _ => None,
        }
    }
    #[inline]
    pub fn get_mut(&mut self, h: Handle) -> Option<&mut T> {
        match self.slots.get_mut(h.index as usize) {
            Some(Slot::Occupied(g, v)) if *g == h.generation => Some(v),
            _ => None,
        }
    }
    /// Access by raw index (ignores generation).
    #[inline]
    pub fn get_index(&self, index: u32) -> Option<(Handle, &T)> {
        match self.slots.get(index as usize) {
            Some(Slot::Occupied(g, v)) => Some((Handle { index, generation: *g }, v)),
            _ => None,
        }
    }
    #[inline]
    pub fn contains(&self, h: Handle) -> bool {
        self.get(h).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| match s {
            Slot::Occupied(g, v) => Some((Handle { index: i as u32, generation: *g }, v)),
            _ => None,
        })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle, &mut T)> {
        self.slots.iter_mut().enumerate().filter_map(|(i, s)| match s {
            Slot::Occupied(g, v) => Some((Handle { index: i as u32, generation: *g }, v)),
            _ => None,
        })
    }
    pub fn handles(&self) -> Vec<Handle> {
        self.iter().map(|(h, _)| h).collect()
    }
}

impl<T> std::ops::Index<Handle> for Arena<T> {
    type Output = T;
    fn index(&self, h: Handle) -> &T {
        self.get(h).expect("stale handle")
    }
}
impl<T> std::ops::IndexMut<Handle> for Arena<T> {
    fn index_mut(&mut self, h: Handle) -> &mut T {
        self.get_mut(h).expect("stale handle")
    }
}
