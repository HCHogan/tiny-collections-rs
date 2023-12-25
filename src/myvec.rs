use std::{
    alloc::{self, Layout},
    array::IntoIter,
    marker::PhantomData,
    mem,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    slice,
};

pub struct MyVec<T> {
    // Covariant over T
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
    // Tell the compiler to do drop check on inner type.
    _t: PhantomData<T>,
}

// As is
unsafe impl<T: Send> Send for MyVec<T> {}
unsafe impl<T: Sync> Sync for MyVec<T> {}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        assert!(std::mem::size_of::<T>() != 0, "ZST is not supported");
        MyVec {
            // mem::align_of::<T>() in short
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
            _t: PhantomData,
        }
    }

    pub fn push(&mut self, elem: T) {
        if self.len == self.cap {
            self.grow();
        }

        unsafe {
            ptr::write(self.ptr.as_ptr().add(self.len), elem);
        }

        // This can't fail, we'll OOM first.
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            // Copies out the bits from the target address and interpret it as a value of type T.
            unsafe { Some(ptr::read(self.ptr.as_ptr().add(self.len))) }
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len, "index out of bounds");
        if self.cap == self.len {
            self.grow();
        }

        unsafe {
            ptr::copy(
                self.ptr.as_ptr().add(index),
                self.ptr.as_ptr().add(index + 1),
                self.len - index,
            );
            ptr::write(self.ptr.as_ptr().add(index), elem);
            self.len += 1;
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        // Note: '<' because it's *not* valid to remove after everything
        assert!(index < self.len, "index out of bounds");
        unsafe {
            self.len -= 1;
            let result = ptr::read(self.ptr.as_ptr().add(index));
            ptr::copy(
                self.ptr.as_ptr().add(index + 1),
                self.ptr.as_ptr().add(index),
                self.len - index,
            );
            result
        }
    }

    // We index into arrays with unsigned integers, but GEP(ptr::offset) takes a signed integer
    // which means that half of the seemingly valid indices into an array will overflow GEP and
    // actually go in the wrong direction! As such we must limit all allocations to isize::MAX
    // However, On all 64-bit targets that Rust currently supports we're limited to significantly
    // less than all 64 bits(for example x64 uses 48bits), so we can rely on just running out of
    // memory first. But on on 32-bit targets, particularly those with extensions to use more of
    // the address space (PAE x86 or x32), it's theoretically possible to successfully allocate
    // more than isize::MAX bytes of memory.
    fn grow(&mut self) {
        let (new_cap, new_layout) = if self.cap == 0 {
            // If self.cap is 0, we allocate 1 element.
            (1, Layout::array::<T>(1).unwrap())
        } else {
            // Can't overflow since self.cap <= isize::MAX.
            let new_cap = 2 * self.cap;
            // 'Layout::array' checks that the number of bytes is <= usize::MAX,
            let new_layout = Layout::array::<T>(new_cap).unwrap();
            (new_cap, new_layout)
        };

        // However since this is a tutorial, we're not going to be particularly optimal here, and
        // just unconditionally check, rather than use clever platform-specific cfgs.
        assert!(
            new_layout.size() <= isize::MAX as usize,
            "Allocation to large"
        );

        let new_ptr = if self.cap == 0 {
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // If allocation failes, 'new_ptr' will be null, in which case we abort.
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            // platform-specific OOM handler
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }
}

// Implemet Deref and DerefMut, so we can have len, first, last, indexing, slicing, sorting,
// iter, iter_mut, and all other sorts of bells and whistles provided by slice. Sweet!
// All we need is slice::from_raw_parts. It will correctly handle empty slices for us. '
// Later once we set up zero-sized type support it will also Just Work for those too.
impl<T> Deref for MyVec<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for MyVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

pub struct MyVecIntoIter<T> {
    buf: NonNull<T>,
    cap: usize,
    start: *const T,
    end: *const T,
}

impl<T> Iterator for MyVecIntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                let result = ptr::read(self.start);
                self.start = self.start.offset(1);
                Some(result)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.end as usize - self.start as usize) / mem::size_of::<T>();
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for MyVecIntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                self.end = self.end.offset(-1);
                Some(ptr::read(self.end))
            }
        }
    }
}

impl<T> Drop for MyVecIntoIter<T> {
    fn drop(&mut self) {
        // destroy the remaining elements
        for _ in &mut *self {}
        let layout = Layout::array::<T>(self.cap).unwrap();
        unsafe {
            alloc::dealloc(self.buf.as_ptr() as *mut u8, layout);
        }
    }
}

impl<T> IntoIterator for MyVec<T> {
    type Item = T;
    type IntoIter = MyVecIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        let vec = ManuallyDrop::new(self);

        // Can't destructure Vec since it's Drop
        let ptr = vec.ptr;
        let cap = vec.cap;
        let len = vec.len;

        unsafe {
            MyVecIntoIter {
                buf: ptr,
                cap,
                start: ptr.as_ptr(),
                end: if cap == 0 {
                    // can't offset this pointer, it's not allocated
                    ptr.as_ptr()
                } else {
                    ptr.as_ptr().add(len)
                },
            }
        }
    }
}

impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            // call 'destructors' for all elements in the vector
            #[allow(clippy::redundant_pattern_matching)]
            while let Some(_) = self.pop() {}
            let layout = Layout::array::<T>(self.cap).unwrap();
            unsafe {
                alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

impl<T> Default for MyVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RawMyVec<T> {
    ptr: NonNull<T>,
    cap: usize,
}

impl<T> RawMyVec<T> {
    fn new() -> Self {
        assert!(std::mem::size_of::<T>() != 0, "ZST is not supported");
        RawMyVec {
            ptr: NonNull::dangling(),
            cap: 0,
        }
    }

    fn grow(&mut self) {
        let (new_cap, new_layout) = if self.cap == 0 {
            (1, Layout::array::<T>(1).unwrap())
        } else {
            let new_cap = 2 * self.cap;
            (new_cap, Layout::array::<T>(new_cap).unwrap())
        };

        if new_layout.size() > isize::MAX as usize {
            panic!("Allocation too large");
        }

        let new_ptr = if self.cap == 0 {
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }
}
