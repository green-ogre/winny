use std::{
    alloc::Layout,
    ptr::{self, NonNull},
};

use crate::IntoStorageError;

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
    item_layout: Option<Layout>,
    data: NonNull<u8>,
    drop: Option<DumbDrop>,
}

impl DumbVec {
    pub fn new<T>() -> Self {
        let drop = new_dumb_drop::<T>();
        let capacity = if std::mem::size_of::<T>() == 0 { usize::MAX } else { 0 };
        let item_layout = if std::mem::size_of::<T>() == 0 { None } else { std::alloc::Layout::new::<T>() };

        Self {
            len: 0,
            data: NonNull::dangling(),
            item_layout,
            capacity,
            drop,
        }
    }

    pub fn to_new_with_capacity(&self, capacity: usize) -> DumbVec {
        if self.capacity == usize::MAX {
        Self {
            len: 0,
            data: NonNull::dangling(),
            item_layout: None,
            capacity: usize::MAX,
            drop: None,
        }
        } else {

        let alloc_layout = unsafe {
            Layout::from_size_align_unchecked(
                self.item_layout.unwrap().size() * capacity,
                self.item_layout.unwrap().align(),
            )
        };
        let ptr = unsafe { std::alloc::alloc(alloc_layout) };
        let Some(data) = NonNull::new(ptr) else {
            panic!("Failed to allocate memory of capaity {}!", capacity);
        };

        Self {
            capacity,
            data,
            len: 0,
            item_layout: self.item_layout.unwrap().clone(),
            drop: self.drop.clone(),
        }
        }
    }

    pub fn remove_and_push_other(&mut self, other: &mut DumbVec, src: usize) {
        if src >= self.len {
            panic!("removal index exceedes bounds");
        }

        if other.len >= other.capacity || other.capacity == 0 {
            other.reserve(1);
        }

        let size = self.item_layout.size();

        unsafe {
            {
                let ptr = self.get_ptr().add(src * size);
                ptr::copy(ptr, other.get_ptr().add(other.len * size), size);
                ptr::copy(ptr.add(1 * size), ptr, (self.len - src - 1) * size);
            }
            self.len -= 1;
            other.len += 1;
        }
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
        let size = self.item_layout.unwrap().size();

        unsafe { NonNull::new_unchecked(self.get_ptr().byte_add(size * index)) }
    }

    pub fn get_mut_unchecked(&self, index: usize) -> NonNull<u8> {
        let size = self.item_layout.unwrap().size();

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
                self.item_layout.unwrap().size() * exact_count,
                self.item_layout.unwrap().align(),
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

    pub fn push<T>(&mut self, val: T) -> Result<(), IntoStorageError> {

        if self.item_layout.is_none() {

        self.len += 1;
            return Ok(());
        }

        if std::alloc::Layout::new::<T>() != self.item_layout.unwrap() {
            return Err(IntoStorageError::LayoutMisMatch);
        }

        if self.len >= self.capacity || self.capacity == 0 {
            self.reserve(1);
        }

        unsafe { ptr::write(self.get_unchecked(self.len).cast::<T>().as_mut(), val) };
        self.len += 1;

        Ok(())
    }

    pub fn push_unchecked<T>(&mut self, val: T) -> Result<(), ()> {
        if self.item_layout.is_none() {

        self.len += 1;
            return Ok(());
        }

        if self.len >= self.capacity || self.capacity == 0 {
            self.reserve(1);
        }

        unsafe { ptr::write(self.get_unchecked(self.len).cast::<T>().as_mut(), val) };
        self.len += 1;

        Ok(())
    }

    pub fn push_erased_unchecked(&mut self, val: *const u8) {
        if self.len >= self.capacity || self.capacity == 0 {
            self.reserve(1);
        }

        let size = self.item_layout.unwrap().size();

        unsafe { self.get_ptr().add(self.len * size).copy_from(val, size) }
    }

    pub fn pop<T>(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(unsafe { ptr::read(self.get_unchecked(self.len).cast::<T>().as_ref()) })
        }
    }

    pub fn pop_unchecked<T>(&mut self) -> T {
        self.len -= 1;
        unsafe { ptr::read(self.get_unchecked(self.len).cast::<T>().as_ref()) }
    }

    pub fn push_dyn<T>(&mut self, val: T) -> Result<(), ()> {
        if std::alloc::Layout::new::<T>() != self.item_layout.unwrap() {
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
            std::ptr::copy_nonoverlapping(last.as_ptr(), remove.as_ptr(), self.item_layout.unwrap().size())
        }

        self.len -= 1;
    }

    pub fn swap_remove_drop_unchecked(&mut self, index: usize) {
        self.swap_remove_unchecked(index);

        if let Some(drop) = self.drop {
            unsafe { drop(self.get_mut_unchecked(index).as_ptr()) };
        }
    }

    pub fn as_slice<T>(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.get_ptr() as *const T, self.len) }
    }

    pub fn into_vec<T>(&mut self) -> Vec<T> {
        let mut new_vec = Vec::with_capacity(self.len);

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
                    ptr = ptr.byte_add(self.item_layout.unwrap().size());
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
                    ptr = ptr.byte_add(self.item_layout.unwrap().size());
                }
            }
        }
    }
}
