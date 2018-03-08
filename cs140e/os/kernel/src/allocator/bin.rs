use std::fmt;
use std::cmp::{min, max};
use alloc::heap::{AllocErr, Layout};

use allocator::util::*;
use allocator::linked_list::LinkedList;

/// A simple allocator that allocates based on size classes.
#[derive(Debug)]
pub struct Allocator {
    bins: [LinkedList; 32],
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
        let mut bins = [LinkedList::new(); 32];
        let mut start = start;

        while start < end {
            let sz = min(1 << start.trailing_zeros(),
                        (end - start).next_power_of_two() << 1);
            if sz >= 1 << 3 {
                unsafe {
                    bins[sz.trailing_zeros() as usize - 3]
                        .push(start as *mut usize);
                }
            }
            start += sz;
        }

        Allocator { bins: bins }
    }

    /// Allocates memory. Returns a pointer meeting the size and alignment
    /// properties of `layout.size()` and `layout.align()`.
    ///
    /// If this method returns an `Ok(addr)`, `addr` will be non-null address
    /// pointing to a block of storage suitable for holding an instance of
    /// `layout`. In particular, the block will be at least `layout.size()`
    /// bytes large and will be aligned to `layout.align()`. The returned block
    /// of storage may or may not have its contents initialized or zeroed.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure that `layout.size() > 0` and that
    /// `layout.align()` is a power of two. Parameters not meeting these
    /// conditions may result in undefined behavior.
    ///
    /// # Errors
    ///
    /// Returning `Err` indicates that either memory is exhausted
    /// (`AllocError::Exhausted`) or `layout` does not meet this allocator's
    /// size or alignment constraints (`AllocError::Unsupported`).
    pub fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
		if !layout.align().is_power_of_two() {
			return Err(AllocErr::Unsupported {details: "Requested layout is not a power of two"} );
		} else if layout.align() <= 0 {
			return Err(AllocErr::Unsupported {details: "Requested layout is too small"} );
		}

        let sz_bits = layout.size().next_power_of_two().trailing_zeros();
		Self::_alloc(self, sz_bits as usize, layout.align(), layout)
    }
	
    fn _alloc(&mut self, sz: usize, align: usize, layout: Layout) -> Result<*mut u8, AllocErr> {
        let bin_index = sz.saturating_sub(3);
        if bin_index >= 32 {
            return Err(AllocErr::Exhausted{
                request: layout
            })
        }
        for node in self.bins[bin_index].iter_mut() {
            let addr = node.value() as usize;
            if (addr / align) * align == addr {
                return Ok(node.pop() as *mut u8);
            }
        }
        let new_node = Self::_alloc(self, sz + 1, align, layout)?; 
        unsafe {
            self.bins[bin_index]
                .push(new_node.add(1 << sz) as *mut usize);
        }
        Ok(new_node)
	}

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure the following:
    ///
    ///   * `ptr` must denote a block of memory currently allocated via this
    ///     allocator
    ///   * `layout` must properly represent the original layout used in the
    ///     allocation call that returned `ptr`
    ///
    /// Parameters not meeting these conditions may result in undefined
    /// behavior.
    pub fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let sz_bits = layout.size().next_power_of_two().trailing_zeros();

        Self::_dealloc(self, ptr, sz_bits as usize)
    }

    fn _dealloc(&mut self, ptr: *mut u8, sz: usize) {
        let my_addr = ptr as usize;
        let mut buddy : Option<usize> = None;
        let buddy_addr = my_addr ^ (1 << sz);
        // For sz < 3, use bin[0]
        let bin_index = sz.saturating_sub(3);
        if bin_index >= 32 {
            return;
        }

        for node in self.bins[bin_index].iter_mut() {
            let node_addr = node.value() as usize;
            if node_addr == buddy_addr {
                node.pop();
                buddy = Some(node_addr);
                break;
            }
        }

        match buddy {
            Some(node_addr) => {
                let new_addr = min(my_addr, node_addr);
                Self::_dealloc(self, new_addr as *mut u8, sz + 1);
            }
            None => {
                unsafe {
                    self.bins[sz - 3].push(ptr as *mut usize);
                }
            }
        }
	}
}
//
// FIXME: Implement `Debug` for `Allocator`.
