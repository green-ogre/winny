use std::{
    alloc::Layout,
    cell::UnsafeCell,
    ptr::{self, NonNull},
};

use crate::ComponentDescription;

pub type DumbDrop = unsafe fn(ptr: *mut u8);

pub fn new_dumb_drop<T>() -> Option<DumbDrop> {
    match std::mem::needs_drop::<T>() {
        true => Some(|ptr: *mut u8| unsafe {
            std::ptr::drop_in_place(ptr as *mut T);
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
        let size = self.item_layout.size();

        unsafe { NonNull::new_unchecked(self.get_ptr().byte_add(size * index)) }
    }

    pub fn get_mut_unchecked(&self, index: usize) -> NonNull<u8> {
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

        if self.len >= self.capacity || self.capacity == 0 {
            self.reserve(1);
        }

        unsafe { ptr::write(self.get_unchecked(self.len).cast::<T>().as_mut(), val) };
        self.len += 1;

        Ok(())
    }

    pub fn pop<T>(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(unsafe { ptr::read(self.get_unchecked(self.len).cast::<T>().as_ref()) })
        }
    }

    pub fn push_dyn<T>(&mut self, val: T) -> Result<(), ()> {
        if std::alloc::Layout::new::<T>() != self.item_layout {
            return Err(());
        }

        if self.len > self.capacity || self.capacity == 0 {
            self.reserve(self.len);
        } else {
            unsafe { ptr::write(self.get_unchecked(self.len).cast::<T>().as_mut(), val) };
            self.len += 1;
        }

        Ok(())
    }

    // The 'removed' value is overwritten by the last element. There will be two instances of an
    // element, however, the old element is forgotten and will either be dropped when the DumbVec leaves scope, or
    // overwritten by another element.
    fn swap_remove_unchecked(&mut self, index: usize) {
        let last = self.get_mut_unchecked(self.len - 1);
        let remove = self.get_mut_unchecked(index);

        unsafe {
            std::ptr::copy_nonoverlapping(last.as_ptr(), remove.as_ptr(), self.item_layout.size())
        }

        self.len -= 1;
    }

    pub fn swap_remove_drop_unchecked(&mut self, index: usize) {
        self.swap_remove_unchecked(index);

        if let Some(drop) = self.drop {
            unsafe { drop(self.get_mut_unchecked(index).as_ptr()) };
        }
    }

    pub fn as_slice_unchecked<T>(&self) -> &[UnsafeCell<T>] {
        unsafe { std::slice::from_raw_parts(self.get_ptr() as *const UnsafeCell<T>, self.len) }
    }

    pub fn into_vec<T>(&mut self) -> Vec<T> {
        let mut new_vec = vec![];

        while let Some(val) = self.pop::<T>() {
            new_vec.push(val);
        }

        new_vec
    }

    pub fn clear_drop(&mut self) {
        if let Some(drop) = self.drop {
            let mut ptr = self.data.as_ptr();

            for _ in 0..self.len {
                unsafe {
                    drop(ptr);
                    ptr = ptr.byte_add(self.item_layout.size());
                }
            }
        }

        self.len = 0;
    }
}

impl Drop for DumbVec {
    fn drop(&mut self) {
        if let Some(drop) = self.drop {
            let mut ptr = self.data.as_ptr();

            for _ in 0..self.len {
                unsafe {
                    drop(ptr);
                    ptr = ptr.byte_add(self.item_layout.size());
                }
            }
        }
    }
}
