use std::alloc::{AllocError, Allocator, Global, Layout};
use std::ptr::NonNull;

use crate::nnue::{NNUE, Network};

const HUGE_PAGE: usize = 2 * 1024 * 1024;

fn advise(ptr: *const u8, len: usize) {
    #[cfg(target_os = "linux")]
    unsafe {
        libc::madvise(ptr.cast_mut().cast(), len, libc::MADV_HUGEPAGE);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (ptr, len);
}

pub fn nnue_advise_huge_pages() {
    let len = size_of::<Network>().next_multiple_of(HUGE_PAGE);
    advise((&raw const *NNUE).cast(), len);
}

#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct LargePages;

impl LargePages {
    #[inline]
    fn huge_layout(layout: Layout) -> Option<Layout> {
        if !cfg!(target_os = "linux") || layout.size() < HUGE_PAGE {
            return None;
        }
        let align = layout.align().max(HUGE_PAGE);
        let size = layout.size().next_multiple_of(HUGE_PAGE);
        Layout::from_size_align(size, align).ok()
    }
}

unsafe impl Allocator for LargePages {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let Some(huge) = Self::huge_layout(layout) else {
            return Global.allocate(layout);
        };
        let block = Global.allocate(huge)?;
        advise(block.cast().as_ptr(), block.len());
        Ok(block)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let layout = Self::huge_layout(layout).unwrap_or(layout);
        unsafe { Global.deallocate(ptr, layout) }
    }
}

