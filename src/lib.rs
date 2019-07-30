use hibitset::BitSetLike;
use specs::storage::{DistinctStorage, UnprotectedStorage};
use specs::world::Index;

const SPARSE_RATIO: usize = 4;

struct InterleavedGroup<T> {
    redirects: [u16; SPARSE_RATIO],
    data: Option<T>,
}

impl<T> InterleavedGroup<T> {
    const fn blank() -> Self {
        Self {
            redirects: [0; SPARSE_RATIO],
            data: None,
        }
    }
}

pub struct IDVStorage<T> {
    inner: Vec<InterleavedGroup<T>>,
    next_free_slot: usize,
}

impl<T> Default for IDVStorage<T> {
    fn default() -> Self {
        IDVStorage {
            inner: Vec::new(),
            next_free_slot: 0,
        }
    }
}

impl<T> IDVStorage<T> {
    #[inline]
    unsafe fn resolve_to_internal(&self, idx: usize) -> u16 {
        let group_idx = idx / SPARSE_RATIO;
        let group_sub = idx % SPARSE_RATIO;
        debug_assert!(self.inner.len() > group_idx);
        *self
            .inner
            .get_unchecked(group_idx)
            .redirects
            .get_unchecked(group_sub)
    }

    #[inline]
    unsafe fn check_prefill(&mut self, idx_cap: usize) {
        let additional = idx_cap.saturating_sub(self.inner.len() / SPARSE_RATIO);
        debug_assert!(additional < std::isize::MAX as usize);
        self.inner.reserve(additional);
        while self.inner.len() / SPARSE_RATIO < idx_cap {
            self.inner.push(InterleavedGroup::blank());
        }
    }

    #[inline]
    unsafe fn find_free(&mut self) -> usize {
        let start = self.next_free_slot - 1;
        let mut i = start;

        // Loop around once, searching for an open slot.
        while i != start + 1 {
            if i == self.inner.len() - 1 {
                i = 0;
            }

            let e = self.inner.get_unchecked(i);
            if e.data.is_none() {
                self.next_free_slot = i;
                return i;
            }

            i += 1;
        }

        // Did not find a open slot. Expanding.
        self.inner.push(InterleavedGroup::blank());
        self.inner.push(InterleavedGroup::blank());
        self.next_free_slot = self.inner.len() - 1;
        self.next_free_slot
    }

    #[inline]
    unsafe fn c_insert(&mut self, idx: usize, v: T) {
        self.check_prefill(idx);
        let group_idx = idx / SPARSE_RATIO;
        let group_sub = idx % SPARSE_RATIO;
        let internal_point = self.find_free();
        debug_assert!(internal_point < self.inner.len());
        debug_assert!(group_idx < self.inner.len());
        *self
            .inner
            .get_unchecked_mut(group_idx)
            .redirects
            .get_unchecked_mut(group_sub) = internal_point as u16;
        self.inner.get_unchecked_mut(internal_point).data = Some(v);
    }

    #[inline]
    unsafe fn c_get(&self, idx: usize) -> Option<&T> {
        let internal = self.resolve_to_internal(idx);
        debug_assert!((internal as usize) < self.inner.len());
        self.inner.get_unchecked(internal as usize).data.as_ref()
    }

    #[inline]
    unsafe fn c_get_mut(&mut self, idx: usize) -> Option<&mut T> {
        let internal = self.resolve_to_internal(idx);
        debug_assert!((internal as usize) < self.inner.len());
        self.inner
            .get_unchecked_mut(internal as usize)
            .data
            .as_mut()
    }

    #[inline]
    unsafe fn c_remove(&mut self, idx: usize) -> Option<T> {
        let internal = self.resolve_to_internal(idx);
        debug_assert!((internal as usize) < self.inner.len());
        self.inner.get_unchecked_mut(internal as usize).data.take()
    }

    #[inline]
    unsafe fn c_clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        let mut garbage = Vec::new();

        for (i, e) in self.inner.iter_mut().enumerate() {
            for j in 0..SPARSE_RATIO {
                if has.contains((i * j) as u32) {
                    let real = e.redirects[j];
                    garbage.push(real);
                }
            }
        }

        for idx in garbage {
            self.inner[idx as usize].data = None;
        }
    }
}

impl<T> UnprotectedStorage<T> for IDVStorage<T> {
    #[inline]
    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        self.c_clean(has);
    }

    #[inline]
    unsafe fn get(&self, idx: Index) -> &T {
        self.c_get(idx as usize).unwrap()
    }

    #[inline]
    unsafe fn get_mut(&mut self, idx: Index) -> &mut T {
        self.c_get_mut(idx as usize).unwrap()
    }

    #[inline]
    unsafe fn insert(&mut self, idx: Index, v: T) {
        self.c_insert(idx as usize, v);
    }

    #[inline]
    unsafe fn remove(&mut self, idx: Index) -> T {
        self.c_remove(idx as usize).unwrap()
    }
}

unsafe impl<T> DistinctStorage for IDVStorage<T> {}
