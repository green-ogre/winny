use std::{alloc::Layout, cell::UnsafeCell, ptr::NonNull};

use crate::ComponentDescription;

pub type DumbDrop = unsafe fn(ptr: *mut u8, len: usize);

pub fn new_dumb_drop<T>() -> Option<DumbDrop> {
    match std::mem::needs_drop::<T>() {
        true => Some(|mut ptr: *mut u8, len: usize| {
            for _ in 0..len {
                unsafe {
                    std::ptr::drop_in_place(ptr as *mut T);
                    ptr = ptr.byte_add(std::mem::size_of::<T>());
                }
            }
        }),
        false => None,
    }
}

#[derive(Debug)]
pub struct DumbVec {
    capacity: usize,
    len: usize,
    item_layout: Layout,
    data: NonNull<u8>,
    drop: Option<DumbDrop>,
}

impl DumbVec {
    pub fn new(layout: Layout, capacity: usize, drop: Option<DumbDrop>) -> Self {
        let alloc_layout =
            unsafe { Layout::from_size_align_unchecked(layout.size() * capacity, layout.align()) };
        let ptr = unsafe { std::alloc::alloc(alloc_layout) };
        let Some(data) = NonNull::new(ptr) else {
            panic!("Failed to allocate memory of capaity {}!", capacity);
        };

        Self {
            len: 0,
            item_layout: layout,
            capacity,
            data,
            drop,
        }
    }

    pub fn from_description(desc: ComponentDescription) -> Self {
        DumbVec::new(desc.layout, 0, desc.drop)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn get_ptr(&self) -> *mut u8 {
        self.data.as_ptr()
    }

    pub fn get_unchecked(&self, index: usize) -> NonNull<u8> {
        debug_assert!(index < self.len);
        let size = self.item_layout.size();

        unsafe { NonNull::new_unchecked(self.get_ptr().byte_add(size * index)) }
    }

    pub fn get_mut_unchecked(&self, index: usize) -> NonNull<u8> {
        debug_assert!(index < self.len);
        let size = self.item_layout.size();

        unsafe { NonNull::new_unchecked(self.get_ptr().byte_add(size * index)) }
    }

    fn reserve(&mut self, additional: usize) {
        if self.len + additional > self.capacity {
            self.resize_exact(self.capacity + additional);
        }
    }

    fn resize_exact(&mut self, exact_count: usize) {
        let new_layout = unsafe {
            Layout::from_size_align_unchecked(
                self.item_layout.size() * exact_count,
                self.item_layout.align(),
            )
        };

        let data = if self.capacity == 0 {
            unsafe { std::alloc::alloc(new_layout) }
        } else {
            unsafe { std::alloc::realloc(self.data.as_ptr(), new_layout, new_layout.size()) }
        };

        self.capacity = exact_count;
        self.data = unsafe { NonNull::new_unchecked(data) };
    }

    pub fn push<T>(&mut self, val: T) -> Result<(), ()> {
        if std::alloc::Layout::new::<T>() != self.item_layout {
            return Err(());
        }

        self.len += 1;

        if self.len > self.capacity {
            self.reserve(1);
        }

        if self.capacity == 0 {
            self.reserve(1);
        }

        let ptr = self.get_unchecked(self.len - 1);
        let mut temp = std::mem::ManuallyDrop::new(val);
        let val = NonNull::from(&mut *temp).cast();

        unsafe {
            std::ptr::copy_nonoverlapping::<u8>(val.as_ptr(), ptr.as_ptr(), self.item_layout.size())
        };

        Ok(())
    }

    pub fn push_dyn<T>(&mut self, val: T) {
        self.len += 1;

        if self.len > self.capacity {
            self.reserve(self.capacity);
        }

        if self.capacity == 0 {
            self.reserve(1);
        }

        let ptr = self.get_unchecked(self.len - 1);
        let mut temp = std::mem::ManuallyDrop::new(val);
        let val = NonNull::from(&mut *temp).cast();

        unsafe {
            std::ptr::copy_nonoverlapping::<u8>(val.as_ptr(), ptr.as_ptr(), self.item_layout.size())
        };
    }

    // The 'removed' value is overwritten by the last element. There will be two instances of an
    // element, however, the old element is forgotten and will either be dropped when the DumbVec leaves scope, or
    // overwritten by another element.
    pub fn swap_remove(&mut self, index: usize) {
        let last = self.get_mut_unchecked(self.len - 1);
        let remove = self.get_mut_unchecked(index);

        unsafe {
            std::ptr::copy_nonoverlapping(last.as_ptr(), remove.as_ptr(), self.item_layout.size())
        }

        self.len -= 1;
    }

    pub fn as_slice_unchecked<T>(&self) -> &[UnsafeCell<T>] {
        unsafe { std::slice::from_raw_parts(self.get_ptr() as *const UnsafeCell<T>, self.len) }
    }
}

impl Drop for DumbVec {
    fn drop(&mut self) {
        if let Some(drop) = self.drop {
            unsafe { drop(self.data.as_ptr(), self.len) };
        }
    }
}
