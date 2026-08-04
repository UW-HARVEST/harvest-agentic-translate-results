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
        // Avoid zero-byte allocation by using at least 1
        let total = size.checked_mul(el).unwrap_or(0).max(1);
        let layout = Layout::from_size_align(total, mem::align_of::<u8>())
            .expect("invalid layout");
        let ptr = unsafe { alloc(layout) };
        let b = NonNull::new(ptr).expect("allocation failed");
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
        let total = self.size.checked_mul(self.el).unwrap_or(0).max(1);
        let layout = Layout::from_size_align(total, mem::align_of::<u8>())
            .expect("invalid layout");
        unsafe {
            dealloc(self.b.as_ptr(), layout);
        }
    }

    pub fn resize(&mut self, size: usize) {
        // resize is only for growing array and new size must be a power of two
        assert!(size > self.size && (size & (size.wrapping_sub(1))) == 0);

        let old_total = self.size.checked_mul(self.el).unwrap_or(0).max(1);
        let new_total = size.checked_mul(self.el).expect("overflow");
        let layout = Layout::from_size_align(old_total, mem::align_of::<u8>())
            .expect("invalid layout");
        let new_ptr = unsafe { realloc(self.b.as_ptr(), layout, new_total) };
        let new_b = NonNull::new(new_ptr).expect("realloc failed");
        self.b = new_b;

        if self.start + self.n > self.size {
            // nend is the num items at the end of the buffer, nbeg is at the beginning
            let nend = self.size - self.start;
            let nbeg = (self.start + self.n) & self.mask;
            unsafe {
                let base = self.b.as_ptr();
                if nend < nbeg {
                    // memmove(b + size - nend, b + l->size - nend, el*nend)
                    let src = base.add((self.size - nend) * self.el);
                    let dst = base.add((size - nend) * self.el);
                    ptr::copy(src, dst, nend * self.el);
                } else {
                    // memmove(b + l->size, b, el * nbeg)
                    let src = base;
                    let dst = base.add(self.size * self.el);
                    ptr::copy(src, dst, nbeg * self.el);
                }
            }
        }

        self.size = size;
        self.mask = size - 1;
    }

    pub fn capacity(&mut self, size: usize) {
        if size > self.size {
            let rounded = roundup64(size as u64) as usize;
            self.resize(rounded);
        }
    }

    fn pos(&self, idx: usize) -> usize {
        (self.start + idx) & self.mask
    }

    fn get(&mut self, idx: usize) -> &mut [u8] {
        let pos = self.pos(idx);
        unsafe {
            let p = self.b.as_ptr().add(pos * self.el);
            slice::from_raw_parts_mut(p, self.el)
        }
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
        let slot = self.get(0);
        for byte in slot.iter_mut() {
            *byte = 0;
        }
        slot
    }

    pub fn pop(&mut self) -> &mut [u8] {
        assert!(self.n > 0);
        let old = self.start;
        self.start = (self.start + 1) & self.mask;
        self.n -= 1;
        unsafe {
            let p = self.b.as_ptr().add(old * self.el);
            slice::from_raw_parts_mut(p, self.el)
        }
    }

    pub fn unshift(&mut self) -> &mut [u8] {
        if self.n == self.size {
            self.resize(self.size * 2);
        }
        let n = self.n;
        let slot = self.get(n);
        for byte in slot.iter_mut() {
            *byte = 0;
        }
        self.n += 1;
        // re-borrow: we need to return the slot. Since `slot` was borrowed
        // before n+=1, we need to recompute.
        let n = self.n - 1;
        self.get(n)
    }

    pub fn shift(&mut self) -> &mut [u8] {
        assert!(self.n > 0);
        let n = self.n;
        // C code: returns circa_get(l, l->n) BEFORE decrement
        // Translate exactly: we need pointer at position (start+n) & mask
        let _ = self.get(n);
        self.n -= 1;
        // recompute pointer at the same position as before (start + (n+1)) & mask
        let pos = (self.start + n) & self.mask;
        unsafe {
            let p = self.b.as_ptr().add(pos * self.el);
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
                    // memmove(b + newstart, b + start, el * nright)
                    let src = base.add(self.start * self.el);
                    let dst = base.add(newstart * self.el);
                    ptr::copy(src, dst, nright * self.el);
                    // memcpy(b + newstart + nright, b, el * nleft)
                    let src2 = base;
                    let dst2 = base.add((newstart + nright) * self.el);
                    ptr::copy_nonoverlapping(src2, dst2, nleft * self.el);
                } else {
                    // gca_cycle_left(b, size, el, start - newstart)
                    let total = self.size * self.el;
                    let buf = slice::from_raw_parts_mut(base, total);
                    gca_cycle_left(buf, self.size, self.el, self.start - newstart);
                }
            }
            self.start = newstart;
        }
    }
}

impl Drop for CircBuf {
    fn drop(&mut self) {
        self.dealloc();
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
        shift += 1;
        a >>= 1;
        b >>= 1;
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
