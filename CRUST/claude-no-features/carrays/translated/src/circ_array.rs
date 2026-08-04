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
        let bytes = size.checked_mul(el).expect("size * el overflow");
        // Allocate with alignment of 1 byte (u8 buffer)
        let layout = Layout::from_size_align(bytes.max(1), 1).expect("invalid layout");
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
        if self.size == 0 {
            return;
        }
        let bytes = self.size.checked_mul(self.el).expect("overflow");
        let layout = Layout::from_size_align(bytes.max(1), 1).expect("invalid layout");
        unsafe {
            dealloc(self.b.as_ptr(), layout);
        }
        self.size = 0;
        self.n = 0;
        self.start = 0;
        self.mask = 0;
        // Use a dangling pointer
        self.b = NonNull::dangling();
    }

    pub fn resize(&mut self, size: usize) {
        // resize is only for growing array and new size must be a power of two
        assert!(size > self.size && (size & size.wrapping_sub(1)) == 0);

        let old_size = self.size;
        let old_bytes = old_size.checked_mul(self.el).expect("overflow");
        let new_bytes = size.checked_mul(self.el).expect("overflow");
        let old_layout = Layout::from_size_align(old_bytes.max(1), 1).expect("invalid layout");
        let new_ptr = unsafe { realloc(self.b.as_ptr(), old_layout, new_bytes.max(1)) };
        let new_b = NonNull::new(new_ptr).expect("realloc failed");
        self.b = new_b;

        if self.start + self.n > old_size {
            // nend is the num items at the end of the b, nbeg is at the beginning
            let nend = old_size - self.start;
            let nbeg = (self.start + self.n) & self.mask;
            unsafe {
                let base = self.b.as_ptr();
                if nend < nbeg {
                    // memmove(b+size-nend, b+old_size-nend, el*nend)
                    let dst = base.add((size - nend) * self.el);
                    let src = base.add((old_size - nend) * self.el);
                    ptr::copy(src, dst, self.el * nend);
                } else {
                    // memmove(b+old_size, b, el*nbeg)
                    let dst = base.add(old_size * self.el);
                    let src = base;
                    ptr::copy(src, dst, self.el * nbeg);
                }
            }
        }
        self.size = size;
        self.mask = size - 1;
    }

    pub fn capacity(&mut self, size: usize) {
        if size > self.size {
            let new_size = roundup64(size as u64) as usize;
            self.resize(new_size);
        }
    }

    pub fn push(&mut self) -> &mut [u8] {
        if self.n == self.size {
            self.resize(self.size * 2);
        }
        self.start = if self.start != 0 { self.start - 1 } else { self.size - 1 };
        self.n += 1;
        let pos = (self.start) & self.mask;
        let offset = pos * self.el;
        unsafe {
            let p = self.b.as_ptr().add(offset);
            ptr::write_bytes(p, 0, self.el);
            slice::from_raw_parts_mut(p, self.el)
        }
    }

    pub fn pop(&mut self) -> &mut [u8] {
        assert!(self.n > 0);
        let old = self.start;
        self.start = (self.start + 1) & self.mask;
        self.n -= 1;
        let offset = old * self.el;
        unsafe {
            let p = self.b.as_ptr().add(offset);
            slice::from_raw_parts_mut(p, self.el)
        }
    }

    pub fn unshift(&mut self) -> &mut [u8] {
        if self.n == self.size {
            self.resize(self.size * 2);
        }
        let pos = (self.start + self.n) & self.mask;
        let offset = pos * self.el;
        self.n += 1;
        unsafe {
            let p = self.b.as_ptr().add(offset);
            ptr::write_bytes(p, 0, self.el);
            slice::from_raw_parts_mut(p, self.el)
        }
    }

    pub fn shift(&mut self) -> &mut [u8] {
        assert!(self.n > 0);
        // Note: this matches the C implementation which uses circa_get(l, l->n)
        // before decrementing n. After decrement, the item at index n-1 (now n) is
        // returned.
        let pos = (self.start + self.n) & self.mask;
        let offset = pos * self.el;
        self.n -= 1;
        unsafe {
            let p = self.b.as_ptr().add(offset);
            slice::from_raw_parts_mut(p, self.el)
        }
    }

    pub fn norm(&mut self) {
        if self.start + self.n > self.size {
            let newstart = (self.size - self.n) / 2;
            let nleft = self.start + self.n - self.size;
            let nright = self.size - self.start;
            unsafe {
                let base = self.b.as_ptr();
                if nleft <= newstart {
                    // memmove(b+newstart, b+start, el*nright)
                    let dst = base.add(newstart * self.el);
                    let src = base.add(self.start * self.el);
                    ptr::copy(src, dst, self.el * nright);
                    // memcpy(b+newstart+nright, b, el*nleft)
                    let dst2 = base.add((newstart + nright) * self.el);
                    let src2 = base;
                    ptr::copy_nonoverlapping(src2, dst2, self.el * nleft);
                } else {
                    let buf = slice::from_raw_parts_mut(base, self.size * self.el);
                    gca_cycle_left(buf, self.size, self.el, self.start - newstart);
                }
            }
            self.start = newstart;
        }
    }

    fn get(&mut self, idx: usize) -> &mut [u8] {
        let pos = (self.start + idx) & self.mask;
        let offset = pos * self.el;
        unsafe {
            let p = self.b.as_ptr().add(offset);
            slice::from_raw_parts_mut(p, self.el)
        }
    }
}

impl Drop for CircBuf {
    fn drop(&mut self) {
        if self.size > 0 {
            let bytes = self.size.checked_mul(self.el).unwrap_or(0);
            if bytes > 0 {
                let layout = Layout::from_size_align(bytes, 1).expect("invalid layout");
                unsafe {
                    dealloc(self.b.as_ptr(), layout);
                }
                self.size = 0;
            }
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

fn gca_cycle_left(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
    if n <= 1 || shift == 0 {
        return;
    }
    let shift = shift % n;
    if shift == 0 {
        return;
    }

    let gcd = gca_calc_gcd(n as u32, shift as u32) as usize;
    let mut tmp = vec![0u8; es];

    for i in 0..gcd {
        tmp.copy_from_slice(&ptr[es * i..es * (i + 1)]);
        let mut j = i;
        loop {
            let mut k = j + shift;
            if k >= n {
                k -= n;
            }
            if k == i {
                break;
            }
            ptr.copy_within(es * k..es * (k + 1), es * j);
            j = k;
        }
        ptr[es * j..es * (j + 1)].copy_from_slice(&tmp);
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
