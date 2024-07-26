#![allow(clippy::missing_safety_doc, dead_code)]
use std::{
    alloc::Layout,
    cell::UnsafeCell,
    marker::PhantomData,
    mem::ManuallyDrop,
    ptr::{self, NonNull},
};

#[derive(Debug)]
pub struct DumbVec {
    capacity: usize,
    len: usize,
    item_layout: Layout,
    data: NonNull<u8>,
    drop: Option<DumbDrop>,
}

unsafe impl Sync for DumbVec {}
unsafe impl Send for DumbVec {}

impl DumbVec {
    pub fn new<T>() -> Self {
        let drop = new_dumb_drop::<T>();
        let capacity = if std::mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            0
        };
        let item_layout = std::alloc::Layout::new::<T>();

        Self::new_init(item_layout, capacity, drop)
    }

    pub fn new_from(layout: std::alloc::Layout, capacity: usize, drop: Option<DumbDrop>) -> Self {
        let capacity = if layout.size() == 0 {
            usize::MAX
        } else {
            capacity
        };

        Self::new_init(layout, capacity, drop)
    }

    pub fn with_capacity<T>(cap: usize) -> Self {
        let drop = new_dumb_drop::<T>();
        let item_layout = std::alloc::Layout::new::<T>();
        let capacity = if std::mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            cap
        };

        Self::new_init(item_layout, capacity, drop)
    }

    fn new_init(item_layout: Layout, capacity: usize, drop: Option<DumbDrop>) -> Self {
        let data = if capacity != 0 && capacity != usize::MAX {
            unsafe {
                let layout = Layout::from_size_align_unchecked(
                    item_layout.size() * capacity,
                    item_layout.align(),
                );
                NonNull::new_unchecked(std::alloc::alloc(layout))
            }
        } else {
            NonNull::dangling()
        };

        Self {
            drop,
            capacity,
            item_layout,
            data,
            len: 0,
        }
    }

    pub fn clone_empty(&self) -> Self {
        let item_layout = self.item_layout;
        let capacity = if self.capacity == usize::MAX {
            usize::MAX
        } else {
            0
        };
        let drop = self.drop;

        Self::new_init(item_layout, capacity, drop)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.data.as_ptr()
    }

    // Caller is responsible for bounds
    pub unsafe fn get_unchecked(&self, index: usize) -> NonNull<u8> {
        NonNull::new_unchecked(self.as_ptr().add(self.item_layout.size() * index))
    }

    // Caller is responsible for bounds
    pub unsafe fn get_mut_unchecked(&self, index: usize) -> NonNull<u8> {
        NonNull::new_unchecked(self.as_ptr().add(self.item_layout.size() * index))
    }

    fn reserve(&mut self, new_size: usize) {
        if new_size > self.capacity {
            self.resize_exact(new_size);
        }
    }

    fn resize_exact(&mut self, exact_count: usize) {
        assert!(self.item_layout.size() != 0, "capacity overflow");

        let new_layout = unsafe {
            Layout::from_size_align_unchecked(
                self.item_layout.size() * exact_count,
                self.item_layout.align(),
            )
        };

        assert!(
            new_layout.size() <= isize::MAX as usize,
            "Allocation too large"
        );

        unsafe {
            let ptr = if self.capacity == 0 {
                std::alloc::alloc(new_layout)
            } else {
                let old_layout = Layout::from_size_align_unchecked(
                    self.item_layout.size() * self.capacity,
                    self.item_layout.align(),
                );

                std::alloc::realloc(self.data.as_ptr(), old_layout, new_layout.size())
            };
            self.data = NonNull::new(ptr).unwrap();
        }

        self.capacity = exact_count;
        // Cannot check for null ptr allocations
        // self.data = match NonNull::new(new_ptr as *mut T) {
        //     Some(p) => p,
        //     None => alloc::handle_alloc_error(new_layout),
        // };
    }

    // Caller ensures that DumbVec stores the correct type
    pub unsafe fn push<T>(&mut self, val: T) {
        assert!(Layout::new::<T>() == self.item_layout, "Invalid type");

        if self.len >= self.capacity || self.capacity == 0 {
            self.reserve(self.len * 2 + 1);
        }

        ptr::write(self.get_unchecked(self.len).cast::<T>().as_mut(), val);
        self.len += 1;
    }

    // Caller ensures that DumbVec stores the correct type
    pub unsafe fn pop<T>(&mut self) -> T {
        assert!(Layout::new::<T>() == self.item_layout, "Invalid type");

        self.len -= 1;
        ptr::read(self.get_unchecked(self.len).cast::<T>().as_mut())
    }

    // Caller ensures that DumbVec stores the correct type
    pub unsafe fn push_erased(&mut self, val: OwnedPtr) {
        if self.len >= self.capacity || self.capacity == 0 {
            self.reserve(self.len * 2 + 1);
        }

        let src = val.as_ptr();
        let dst = self.get_mut_unchecked(self.len).as_ptr();

        std::ptr::copy_nonoverlapping(src, dst, self.item_layout.size());
        self.len += 1;
    }

    // Caller ensures that DumbVec stores the correct type
    pub unsafe fn replace_drop<T>(&mut self, val: T, index: usize) {
        assert!(index < self.len);

        let dst = self.get_mut_unchecked(index).cast::<T>().as_ptr();
        let _old_val = std::ptr::read(dst);
        std::ptr::write(dst, val);
    }

    // Caller ensures that DumbVec stores the correct type
    pub unsafe fn replace_erased(&mut self, val: OwnedPtr, index: usize) {
        assert!(index < self.len);

        let src = val.as_ptr();
        let dst = self.get_mut_unchecked(index).as_ptr();

        std::ptr::copy_nonoverlapping(src, dst, self.item_layout.size());
    }

    pub unsafe fn swap_remove_no_drop(&mut self, index: usize) {
        assert!(index < self.len);

        self.len -= 1;
        if self.len == index {
            return;
        }

        std::ptr::copy_nonoverlapping(
            self.get_mut_unchecked(self.len).as_ptr(),
            self.get_mut_unchecked(index).as_ptr(),
            self.item_layout.size(),
        );
    }

    pub unsafe fn swap_remove_drop(&mut self, index: usize) {
        if let Some(drop) = self.drop {
            assert!(index < self.len);

            drop(self.get_mut_unchecked(index).as_ptr());

            self.len -= 1;
            if self.len == index {
                return;
            }

            std::ptr::copy_nonoverlapping(
                self.get_mut_unchecked(self.len).as_ptr(),
                self.get_mut_unchecked(index).as_ptr(),
                self.item_layout.size(),
            );
        } else {
            self.swap_remove_no_drop(index);
        }
    }

    pub unsafe fn as_slice<T>(&self) -> &[UnsafeCell<T>] {
        std::slice::from_raw_parts(self.as_ptr() as *const UnsafeCell<T>, self.len)
    }

    fn as_slice_debug<T>(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.as_ptr().cast::<T>(), self.len) }
    }

    pub fn clear(&mut self) {
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

        unsafe {
            if self.item_layout.size() != 0 {
                let layout = Layout::from_size_align_unchecked(
                    self.item_layout.size() * self.capacity,
                    self.item_layout.align(),
                );
                std::alloc::dealloc(self.as_ptr(), layout);
            }
        }
    }
}

pub type DumbDrop = unsafe fn(ptr: *mut u8);

pub fn new_dumb_drop<T>() -> Option<DumbDrop> {
    match std::mem::needs_drop::<T>() {
        true => Some(|ptr: *mut u8| unsafe {
            std::ptr::drop_in_place(ptr as *mut T);
        }),
        false => None,
    }
}

pub struct OwnedPtr<'d>(NonNull<u8>, PhantomData<&'d u8>);

impl OwnedPtr<'_> {
    pub fn make<T, F>(val: T, f: F)
    where
        F: FnOnce(OwnedPtr),
    {
        let mut tmp = ManuallyDrop::new(val);

        f(Self(NonNull::from(&mut *tmp).cast(), PhantomData));
    }

    pub fn new<T>(mut val: T) -> Self {
        let ptr = OwnedPtr::from(NonNull::from(&mut val));
        std::mem::forget(val);

        ptr
    }

    pub fn from<T>(ptr: NonNull<T>) -> Self {
        Self(ptr.cast::<u8>(), PhantomData)
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }

    pub fn read<T>(self) -> T {
        unsafe { ptr::read(self.0.as_ptr().cast::<T>()) }
    }

    pub fn drop<T>(self) {
        unsafe { self.0.as_ptr().cast::<T>().drop_in_place() };
    }
}

// impl Drop for OwnedPtr {
//     fn drop(&mut self) {
//         // util::tracing::error!("Failed to drop OwnedPtr");
//         println!("dropping OwnedPtr");
//     }
// }

#[derive(Debug)]
pub struct Test(&'static str);

impl Drop for Test {
    fn drop(&mut self) {
        println!("    Dropping: {:?}", self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings() {
        unsafe {
            let val = String::from("hello");
            let val2 = String::from("world");
            let mut vec = DumbVec::new::<String>();
            vec.push(val);
            vec.push(val2);
            vec.push(String::from("goodbye"));

            // let x = Test(2);
            // let val = { OwnedPtr::new(x) };

            // let val = OwnedPtr::new(Test(2));

            {
                let x = String::from("gooch");
                OwnedPtr::make(x, |x_ptr| vec.push_erased(x_ptr));
            }

            println!("{:?}", vec.as_slice_debug::<String>());

            let mut vec2 = DumbVec::new::<String>();
            let val = OwnedPtr::from(vec.get_unchecked(0));
            vec2.push_erased(val);
            vec.swap_remove_no_drop(0);
            vec2.swap_remove_drop(0);

            println!("{:?}", vec.as_slice_debug::<String>());
            println!("{:?}", vec2.as_slice_debug::<String>());
        }
    }

    #[test]
    fn drop() {
        unsafe {
            let val = Test("hello");
            let val2 = Test("world");
            let mut vec = DumbVec::new::<Test>();
            vec.push(val);
            vec.push(val2);
            vec.push(Test("goodbye"));

            // let x = Test(2);
            // let val = { OwnedPtr::new(x) };

            // let val = OwnedPtr::new(Test(2));

            {
                let x = Test("gooch");
                OwnedPtr::make(x, |x_ptr| vec.push_erased(x_ptr));
            }

            println!("{:?}", vec.as_slice_debug::<Test>());

            let mut vec2 = DumbVec::new::<Test>();
            let val = OwnedPtr::from(vec.get_unchecked(0));
            vec2.push_erased(val);
            vec.swap_remove_no_drop(0);

            vec2.swap_remove_drop(0);

            println!("{:?}", vec.as_slice_debug::<Test>());
            println!("{:?}", vec2.as_slice_debug::<Test>());
        }
    }

    struct Zero;

    #[test]
    fn zero_sized_struct() {
        unsafe {
            let mut vec = DumbVec::new::<Zero>();
            vec.push(Zero);
            vec.push(Zero);
            vec.push(Zero);
        }
    }
}
