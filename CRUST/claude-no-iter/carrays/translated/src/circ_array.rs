use std::alloc::{alloc, dealloc, realloc, Layout};
use std::ptr::{self, NonNull};
use std::slice;
use std::mem;

pub struct CircBuf {
    el: usize,
    start: usize,
    n: usize,
    size: usize,
    mask: usize,
    b: NonNull<u8>,
}

impl CircBuf {
    pub fn new(el: usize, size: usize) -> Self {
        let size = roundup64(size as u64) as usize;
        let bytes = size.checked_mul(el).expect("circa_alloc: size overflow");
        let layout = Layout::from_size_align(bytes.max(1), mem::align_of::<u64>())
            .expect("invalid layout");
        let raw = unsafe { alloc(layout) };
        let b = NonNull::new(raw).expect("allocation failed");
        CircBuf {
            el,
            start: 0,
            n: 0,
            size,
            mask: size.wrapping_sub(1),
            b,
        }
    }

    pub fn dealloc(&mut self) {
        if self.size != 0 {
            let bytes = self.size * self.el;
            let layout = Layout::from_size_align(bytes.max(1), mem::align_of::<u64>())
                .expect("invalid layout");
            unsafe {
                dealloc(self.b.as_ptr(), layout);
            }
            // Set to dangling so we don't double-free
            self.b = NonNull::dangling();
            self.size = 0;
            self.n = 0;
            self.start = 0;
            self.mask = 0;
        }
    }

    pub fn resize(&mut self, size: usize) {
        // resize is only for growing array and new size must be a power of two
        assert!(size > self.size && (size & (size - 1)) == 0);

        let old_bytes = self.size * self.el;
        let new_bytes = size * self.el;
        let layout = Layout::from_size_align(old_bytes.max(1), mem::align_of::<u64>())
            .expect("invalid layout");
        let new_ptr = unsafe { realloc(self.b.as_ptr(), layout, new_bytes.max(1)) };
        let new_nn = NonNull::new(new_ptr).expect("realloc failed");
        self.b = new_nn;

        if self.start + self.n > self.size {
            // nend is the num items at the end of b, nbeg is at the beginning
            let nend = self.size - self.start;
            let nbeg = (self.start + self.n) & self.mask;
            unsafe {
                let base = self.b.as_ptr();
                if nend < nbeg {
                    // memmove(b + size - nend, b + old_size - nend, el*nend)
                    let dst = base.add((size - nend) * self.el);
                    let src = base.add((self.size - nend) * self.el);
                    ptr::copy(src, dst, nend * self.el);
                } else {
                    // memmove(b + old_size, b, el*nbeg)
                    let dst = base.add(self.size * self.el);
                    let src = base;
                    ptr::copy(src, dst, nbeg * self.el);
                }
            }
        }
        self.size = size;
        self.mask = size - 1;
    }

    pub fn capacity(&mut self, size: usize) {
        if size > self.size {
            self.resize(roundup64(size as u64) as usize);
        }
    }

    fn pos(&self, idx: usize) -> usize {
        (self.start + idx) & self.mask
    }

    fn get_ptr(&self, idx: usize) -> *mut u8 {
        let p = self.pos(idx);
        unsafe { self.b.as_ptr().add(p * self.el) }
    }

    pub fn push(&mut self) -> &mut [u8] {
        if self.n == self.size {
            self.resize(self.size * 2);
        }
        self.start = if self.start != 0 {
            self.start - 1
        } else {
            self.size - 1
        };
        self.n += 1;
        let ptr = self.get_ptr(0);
        unsafe {
            ptr::write_bytes(ptr, 0, self.el);
            slice::from_raw_parts_mut(ptr, self.el)
        }
    }

    pub fn pop(&mut self) -> &mut [u8] {
        assert!(self.n > 0);
        let old = self.start;
        self.start = (self.start + 1) & self.mask;
        self.n -= 1;
        unsafe {
            let ptr = self.b.as_ptr().add(old * self.el);
            slice::from_raw_parts_mut(ptr, self.el)
        }
    }

    pub fn unshift(&mut self) -> &mut [u8] {
        if self.n == self.size {
            self.resize(self.size * 2);
        }
        let ptr = self.get_ptr(self.n);
        unsafe {
            ptr::write_bytes(ptr, 0, self.el);
        }
        self.n += 1;
        unsafe { slice::from_raw_parts_mut(ptr, self.el) }
    }

    pub fn shift(&mut self) -> &mut [u8] {
        assert!(self.n > 0);
        // Match C semantics: get pointer at index n, then decrement n
        let ptr = self.get_ptr(self.n);
        self.n -= 1;
        unsafe { slice::from_raw_parts_mut(ptr, self.el) }
    }

    pub fn norm(&mut self) {
        if self.start + self.n > self.size {
            let newstart = (self.size - self.n) / 2;
            let nleft = self.start + self.n - self.size;
            let nright = self.size - self.start;
            unsafe {
                let base = self.b.as_ptr();
                if nleft <= newstart {
                    let dst1 = base.add(newstart * self.el);
                    let src1 = base.add(self.start * self.el);
                    ptr::copy(src1, dst1, self.el * nright);
                    let dst2 = base.add((newstart + nright) * self.el);
                    let src2 = base;
                    ptr::copy_nonoverlapping(src2, dst2, self.el * nleft);
                } else {
                    let total = self.size * self.el;
                    let buf = slice::from_raw_parts_mut(base, total);
                    gca_cycle_left(buf, self.size, self.el, self.start - newstart);
                }
            }
            self.start = newstart;
        }
    }

    fn get(&mut self, idx: usize) -> &mut [u8] {
        let ptr = self.get_ptr(idx);
        unsafe { slice::from_raw_parts_mut(ptr, self.el) }
    }
}

impl Drop for CircBuf {
    fn drop(&mut self) {
        if self.size != 0 {
            let bytes = self.size * self.el;
            let layout = Layout::from_size_align(bytes.max(1), mem::align_of::<u64>())
                .expect("invalid layout");
            unsafe {
                dealloc(self.b.as_ptr(), layout);
            }
            self.size = 0;
        }
    }
}

unsafe impl Send for CircBuf {}
unsafe impl Sync for CircBuf {}

fn roundup64(mut x: u64) -> u64 {
    x = x.wrapping_sub(1);
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x |= x >> 32;
    x.wrapping_add(1)
}

fn gca_cycle_left(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
    if n <= 1 || shift == 0 {
        return;
    }
    let shift = shift % n;
    if shift == 0 {
        return;
    }
    let total = n * es;
    if total > ptr.len() {
        return;
    }
    ptr[..total].rotate_left(shift * es);
}

fn gca_calc_gcd(mut a: u32, mut b: u32) -> u32 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }
    let mut shift: u32 = 0;
    while ((a | b) & 1) == 0 {
        a >>= 1;
        b >>= 1;
        shift += 1;
    }
    while (a & 1) == 0 {
        a >>= 1;
    }
    loop {
        while (b & 1) == 0 {
            b >>= 1;
        }
        if a > b {
            mem::swap(&mut a, &mut b);
        }
        b = b - a;
        if b == 0 {
            break;
        }
    }
    a << shift
}
