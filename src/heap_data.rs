use std::alloc::{alloc, dealloc, Layout};

#[derive(Clone)]
pub struct Heap {
    ptr: *mut u8,
    layout: Layout,
}

impl Drop for Heap {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}

impl Heap {
    pub fn new(size: usize) -> anyhow::Result<Self> {
        let layout = Layout::from_size_align(size, 8)?;
        let ptr = unsafe { alloc(layout) };
        Ok(Heap { ptr, layout })
    }

    pub fn get_ptr(&self) -> *mut u8 {
        self.ptr
    }
}

unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}
