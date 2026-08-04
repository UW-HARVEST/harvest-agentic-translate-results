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
    let bytes = (size * el).max(1);
    let layout = Layout::from_size_align(bytes, 1).unwrap();
    let raw = unsafe { alloc(layout) };
    let b = NonNull::new(raw).expect("allocation failed");
    Self {
        el,
        start: 0,
        n: 0,
        size,
        mask: size.wrapping_sub(1),
        b,
    }
}
pub fn dealloc(&mut self) {
    let bytes = (self.size * self.el).max(1);
    let layout = Layout::from_size_align(bytes, 1).unwrap();
    unsafe { dealloc(self.b.as_ptr(), layout) };
}
pub fn resize(&mut self, size: usize) {
    // resize is only for growing array and new size must be a power of two
    assert!(size > self.size && (size & (size - 1)) == 0);
    let old_bytes = (self.size * self.el).max(1);
    let new_bytes = (size * self.el).max(1);
    let old_layout = Layout::from_size_align(old_bytes, 1).unwrap();
    let new_ptr = unsafe { realloc(self.b.as_ptr(), old_layout, new_bytes) };
    let new_b = NonNull::new(new_ptr).expect("realloc failed");
    self.b = new_b;

    if self.start + self.n > self.size {
        // nend is the num items at the end of the b, nbeg is at the beginning
        let nend = self.size - self.start;
        let nbeg = (self.start + self.n) & self.mask;
        unsafe {
            let base = self.b.as_ptr();
            if nend < nbeg {
                // memmove(b + size - nend, b + old_size - nend, el * nend)
                ptr::copy(
                    base.add((self.size - nend) * self.el),
                    base.add((size - nend) * self.el),
                    nend * self.el,
                );
            } else {
                // memmove(b + old_size, b, el * nbeg)
                ptr::copy(
                    base,
                    base.add(self.size * self.el),
                    nbeg * self.el,
                );
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
pub fn push(&mut self) -> &mut [u8] {
    if self.n == self.size {
        self.resize(self.size * 2);
    }
    self.start = if self.start != 0 { self.start - 1 } else { self.size - 1 };
    self.n += 1;
    let pos = self.start & self.mask;
    let offset = self.el * pos;
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
    let offset = self.el * old;
    unsafe {
        slice::from_raw_parts_mut(self.b.as_ptr().add(offset), self.el)
    }
}
pub fn unshift(&mut self) -> &mut [u8] {
    if self.n == self.size {
        self.resize(self.size * 2);
    }
    let pos = (self.start + self.n) & self.mask;
    let offset = self.el * pos;
    self.n += 1;
    unsafe {
        let p = self.b.as_ptr().add(offset);
        ptr::write_bytes(p, 0, self.el);
        slice::from_raw_parts_mut(p, self.el)
    }
}
pub fn shift(&mut self) -> &mut [u8] {
    assert!(self.n > 0);
    // Match C: returns get(l, l->n) before decrement
    let pos = (self.start + self.n) & self.mask;
    let offset = self.el * pos;
    self.n -= 1;
    unsafe {
        slice::from_raw_parts_mut(self.b.as_ptr().add(offset), self.el)
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
                ptr::copy(
                    base.add(self.start * self.el),
                    base.add(newstart * self.el),
                    nright * self.el,
                );
                // memcpy(b + newstart + nright, b, el * nleft)
                ptr::copy_nonoverlapping(
                    base,
                    base.add((newstart + nright) * self.el),
                    nleft * self.el,
                );
            } else {
                let total_bytes = self.size * self.el;
                let buf = slice::from_raw_parts_mut(base, total_bytes);
                gca_cycle_left(buf, self.size, self.el, self.start - newstart);
            }
        }
        self.start = newstart;
    }
}
fn get(&mut self, idx: usize) -> &mut [u8] {
    let pos = (self.start + idx) & self.mask;
    let offset = self.el * pos;
    unsafe {
        slice::from_raw_parts_mut(self.b.as_ptr().add(offset), self.el)
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
        b -= a;
        if b == 0 {
            break;
        }
    }

    a << shift
}
