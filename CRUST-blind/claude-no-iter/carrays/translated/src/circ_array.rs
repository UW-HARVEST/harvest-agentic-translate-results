use std::alloc::{alloc, dealloc, realloc, Layout};
use std::mem;
use std::ptr::{self, NonNull};
use std::slice;

pub struct CircBuf {
    el: usize,
    start: usize,
    n: usize,
    size: usize,
    mask: usize,
    b: NonNull<u8>,
}

fn make_layout(bytes: usize) -> Layout {
    // Use byte-array layout (1-byte aligned) so we can grow/shrink freely.
    // Rust's Layout::array::<u8>(0) returns Ok with size 0 and align 1.
    Layout::from_size_align(bytes, 1).expect("invalid layout")
}

impl CircBuf {
    pub fn new(el: usize, size: usize) -> Self {
        let size = roundup64(size as u64) as usize;
        let total = size.saturating_mul(el);
        let layout = make_layout(total);
        let raw = if total == 0 {
            // dangling but non-null pointer
            NonNull::<u8>::dangling().as_ptr()
        } else {
            unsafe { alloc(layout) }
        };
        let b = NonNull::new(raw).expect("alloc failed");
        let mask = if size == 0 { 0 } else { size - 1 };
        CircBuf {
            el,
            start: 0,
            n: 0,
            size,
            mask,
            b,
        }
    }

    pub fn dealloc(&mut self) {
        let total = self.size.saturating_mul(self.el);
        if total != 0 {
            let layout = make_layout(total);
            unsafe { dealloc(self.b.as_ptr(), layout) };
        }
        // Replace with a dangling pointer to mark as deallocated
        self.b = NonNull::<u8>::dangling();
        self.size = 0;
        self.n = 0;
        self.start = 0;
        self.mask = 0;
    }

    pub fn resize(&mut self, size: usize) {
        // resize is only for growing array and new size must be a power of two
        assert!(size > self.size && (size & (size.wrapping_sub(1))) == 0);

        let old_total = self.size * self.el;
        let new_total = size * self.el;

        let new_ptr = if old_total == 0 {
            let layout = make_layout(new_total);
            unsafe { alloc(layout) }
        } else {
            let old_layout = make_layout(old_total);
            unsafe { realloc(self.b.as_ptr(), old_layout, new_total) }
        };
        self.b = NonNull::new(new_ptr).expect("realloc failed");

        if self.start + self.n > self.size {
            // nend is the num items at the end of the b, nbeg is at the beginning
            let nend = self.size - self.start;
            let nbeg = (self.start + self.n) & self.mask;
            let bp = self.b.as_ptr();
            unsafe {
                if nend < nbeg {
                    // Move the trailing nend elements to be the last nend
                    // elements of the new (larger) buffer.
                    let src = bp.add(self.el * (self.size - nend));
                    let dst = bp.add(self.el * (size - nend));
                    ptr::copy(src, dst, self.el * nend);
                    self.start = size - nend;
                } else {
                    // Move the leading nbeg elements to position old_size,
                    // making the contents contiguous starting at self.start.
                    let src = bp;
                    let dst = bp.add(self.el * self.size);
                    ptr::copy(src, dst, self.el * nbeg);
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

    // Add to start
    // Returns a pointer to the item added (zero'd)
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
        // get index 0 in circular order
        let pos = (self.start + 0) & self.mask;
        let offset = pos * self.el;
        let el = self.el;
        unsafe {
            let s = slice::from_raw_parts_mut(self.b.as_ptr().add(offset), el);
            for byte in s.iter_mut() {
                *byte = 0;
            }
            s
        }
    }

    // Remove from start
    // Returns a pointer to the item removed
    pub fn pop(&mut self) -> &mut [u8] {
        assert!(self.n > 0);
        let old = self.start;
        self.start = (self.start + 1) & self.mask;
        self.n -= 1;
        let offset = old * self.el;
        let el = self.el;
        unsafe { slice::from_raw_parts_mut(self.b.as_ptr().add(offset), el) }
    }

    // Add to end
    // Returns a pointer to the item added (zero'd)
    pub fn unshift(&mut self) -> &mut [u8] {
        if self.n == self.size {
            self.resize(self.size * 2);
        }
        let pos = (self.start + self.n) & self.mask;
        self.n += 1;
        let offset = pos * self.el;
        let el = self.el;
        unsafe {
            let s = slice::from_raw_parts_mut(self.b.as_ptr().add(offset), el);
            for byte in s.iter_mut() {
                *byte = 0;
            }
            s
        }
    }

    // Remove from end
    // Returns a pointer to the item removed (matches the C behaviour:
    // returns the slot at index `n` BEFORE decrementing)
    pub fn shift(&mut self) -> &mut [u8] {
        assert!(self.n > 0);
        let pos = (self.start + self.n) & self.mask;
        self.n -= 1;
        let offset = pos * self.el;
        let el = self.el;
        unsafe { slice::from_raw_parts_mut(self.b.as_ptr().add(offset), el) }
    }

    // Stop circular array from wrapping around
    pub fn norm(&mut self) {
        if self.start + self.n > self.size {
            let newstart = (self.size - self.n) / 2; // pick a new start
            let nleft = self.start + self.n - self.size;
            let nright = self.size - self.start;
            let bp = self.b.as_ptr();
            unsafe {
                if nleft <= newstart {
                    ptr::copy(
                        bp.add(self.el * self.start),
                        bp.add(self.el * newstart),
                        self.el * nright,
                    );
                    ptr::copy_nonoverlapping(
                        bp,
                        bp.add(self.el * (newstart + nright)),
                        self.el * nleft,
                    );
                } else {
                    let total_bytes = self.size * self.el;
                    let s = slice::from_raw_parts_mut(bp, total_bytes);
                    gca_cycle_left(s, self.size, self.el, self.start - newstart);
                }
            }
            self.start = newstart;
        }
    }

    fn get(&mut self, idx: usize) -> &mut [u8] {
        let pos = (self.start + idx) & self.mask;
        let offset = pos * self.el;
        let el = self.el;
        unsafe { slice::from_raw_parts_mut(self.b.as_ptr().add(offset), el) }
    }
}

impl Drop for CircBuf {
    fn drop(&mut self) {
        let total = self.size.saturating_mul(self.el);
        if total != 0 {
            let layout = make_layout(total);
            unsafe { dealloc(self.b.as_ptr(), layout) };
            // Mark as freed so a subsequent explicit dealloc() is a no-op.
            self.b = NonNull::<u8>::dangling();
            self.size = 0;
            self.n = 0;
            self.start = 0;
            self.mask = 0;
        }
    }
}

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

fn gca_cycle_left(ptr: &mut [u8], n: usize, es: usize, mut shift: usize) {
    if n <= 1 || shift == 0 {
        return;
    }
    shift %= n;

    let gcd = gca_calc_gcd(n as u32, shift as u32) as usize;
    let mut tmp = vec![0u8; es];

    for i in 0..gcd {
        tmp.copy_from_slice(&ptr[i * es..(i + 1) * es]);
        let mut j = i;
        loop {
            let mut k = j + shift;
            if k >= n {
                k -= n;
            }
            if k == i {
                break;
            }
            ptr.copy_within(k * es..(k + 1) * es, j * es);
            j = k;
        }
        ptr[j * es..(j + 1) * es].copy_from_slice(&tmp);
    }
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
        b -= a;
        if b == 0 {
            break;
        }
    }
    a << shift
}
