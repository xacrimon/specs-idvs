use hibitset::BitSetLike;
use std::ptr;

struct InterleavedGroup<T> {
    redirects: [u16; 4],
    data: Option<T>,
}

impl<T> InterleavedGroup<T> {
    fn blank() -> Self {
        Self {
            redirects: [0; 4],
            data: None,
        }
    }
}

#[derive(Default)]
pub struct IDVStorage<T> {
    inner: Vec<InterleavedGroup<T>>,
}

impl<T> IDVStorage<T> {
    fn resolve_to_internal(&self, idx: usize) -> u16 {
        let group_idx = idx / 4;
        let group_sub = idx % 4;
        self.inner[group_idx].redirects[group_sub]
    }

    fn check_prefill(&mut self, idx_cap: usize) {
        while self.inner.len() / 4 < idx_cap {
            self.inner.push(InterleavedGroup::blank());
        }
    }

    fn find_free(&mut self) -> usize {
        for (i, e) in self.inner.iter().enumerate() {
            if let None = e.data {
                return i;
            }
        }
        self.inner.push(InterleavedGroup::blank());
        self.inner.len() - 1
    }

    fn insert(&mut self, idx: usize, v: T) {
        self.check_prefill(idx);
        let group_idx = idx / 4;
        let group_sub = idx % 4;
        let internal_point = self.find_free();
        self.inner[group_idx].redirects[group_sub] = internal_point as u16;
        self.inner[internal_point].data = Some(v);
    }

    fn get(&self, idx: usize) -> Option<&T> {
        let internal = self.resolve_to_internal(idx);
        self.inner[idx].data.as_ref()
    }

    fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        let internal = self.resolve_to_internal(idx);
        self.inner[idx].data.as_mut()
    }

    fn remove(&mut self, idx: usize) -> Option<T> {
        let internal = self.resolve_to_internal(idx);
        self.inner[idx].data.take()
    }

    fn drop(&mut self, idx: usize) {
        self.remove(idx);
    }

    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike
    {
        let mut garbage = Vec::new();

        for (i, e) in self.inner.iter_mut().enumerate() {
            for j in (0..4) {
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
