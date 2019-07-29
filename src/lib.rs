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
}

impl<T> Default for IDVStorage<T> {
    fn default() -> Self {
        IDVStorage { inner: Vec::new() }
    }
}

impl<T> IDVStorage<T> {
    #[inline]
    fn resolve_to_internal(&self, idx: usize) -> u16 {
        let group_idx = idx / SPARSE_RATIO;
        let group_sub = idx % SPARSE_RATIO;
        self.inner[group_idx].redirects[group_sub]
    }

    #[inline]
    fn check_prefill(&mut self, idx_cap: usize) {
        while self.inner.len() / SPARSE_RATIO < idx_cap {
            self.inner.push(InterleavedGroup::blank());
        }
    }

    #[inline]
    fn find_free(&mut self) -> usize {
        for (i, e) in self.inner.iter().enumerate() {
            if let None = e.data {
                return i;
            }
        }
        self.inner.push(InterleavedGroup::blank());
        self.inner.len() - 1
    }

    #[inline]
    fn c_insert(&mut self, idx: usize, v: T) {
        self.check_prefill(idx);
        let group_idx = idx / SPARSE_RATIO;
        let group_sub = idx % SPARSE_RATIO;
        let internal_point = self.find_free();
        self.inner[group_idx].redirects[group_sub] = internal_point as u16;
        self.inner[internal_point].data = Some(v);
    }

    #[inline]
    fn c_get(&self, idx: usize) -> Option<&T> {
        let internal = self.resolve_to_internal(idx);
        self.inner[internal as usize].data.as_ref()
    }

    #[inline]
    fn c_get_mut(&mut self, idx: usize) -> Option<&mut T> {
        let internal = self.resolve_to_internal(idx);
        self.inner[internal as usize].data.as_mut()
    }

    #[inline]
    fn c_remove(&mut self, idx: usize) -> Option<T> {
        let internal = self.resolve_to_internal(idx);
        self.inner[internal as usize].data.take()
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
