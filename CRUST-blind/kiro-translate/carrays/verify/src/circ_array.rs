use std::alloc::{alloc, dealloc, realloc, Layout};
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
impl CircBuf {
pub fn new(el: usize, size: usize) -> Self {
    let size = roundup64(size as u64) as usize;
    let layout = Layout::from_size_align(size * el, 1).unwrap();
    let b = unsafe {
        let ptr = alloc(layout);
        ptr::write_bytes(ptr, 0, size * el);
        NonNull::new(ptr).expect("allocation failed")
    };
    CircBuf { el, start: 0, n: 0, size, mask: size - 1, b }
}
pub fn dealloc(&mut self) {
    let layout = Layout::from_size_align(self.size * self.el, 1).unwrap();
    unsafe { dealloc(self.b.as_ptr(), layout); }
}
pub fn resize(&mut self, size: usize) {
    assert!(size > self.size && (size & (size - 1)) == 0);
    let old_layout = Layout::from_size_align(self.size * self.el, 1).unwrap();
    let new_byte_size = size * self.el;
    let ptr = unsafe { realloc(self.b.as_ptr(), old_layout, new_byte_size) };
    self.b = NonNull::new(ptr).expect("realloc failed");
    if self.start + self.n > self.size {
        let nend = self.size - self.start;
        let nbeg = (self.start + self.n) & self.mask;
        unsafe {
            let b = self.b.as_ptr();
            if nend < nbeg {
                ptr::copy(b.add((self.size - nend) * self.el), b.add((size - nend) * self.el), self.el * nend);
            } else {
                ptr::copy(b, b.add(self.size * self.el), self.el * nbeg);
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
    if self.n == self.size { self.resize(self.size * 2); }
    self.start = if self.start > 0 { self.start - 1 } else { self.size - 1 };
    self.n += 1;
    let pos = (self.start) & self.mask;
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
    unsafe { slice::from_raw_parts_mut(self.b.as_ptr().add(offset), self.el) }
}
pub fn unshift(&mut self) -> &mut [u8] {
    if self.n == self.size { self.resize(self.size * 2); }
    let pos = (self.start + self.n) & self.mask;
    let offset = self.el * pos;
    unsafe {
        let p = self.b.as_ptr().add(offset);
        ptr::write_bytes(p, 0, self.el);
        self.n += 1;
        slice::from_raw_parts_mut(p, self.el)
    }
}
pub fn shift(&mut self) -> &mut [u8] {
    assert!(self.n > 0);
    let pos = (self.start + self.n) & self.mask;
    self.n -= 1;
    let offset = self.el * pos;
    unsafe { slice::from_raw_parts_mut(self.b.as_ptr().add(offset), self.el) }
}
pub fn norm(&mut self) {
    if self.start + self.n > self.size {
        let newstart = (self.size - self.n) / 2;
        let nleft = self.start + self.n - self.size;
        let nright = self.size - self.start;
        unsafe {
            let b = self.b.as_ptr();
            if nleft <= newstart {
                ptr::copy(b.add(self.start * self.el), b.add(newstart * self.el), self.el * nright);
                ptr::copy_nonoverlapping(b, b.add((newstart + nright) * self.el), self.el * nleft);
            } else {
                let buf = slice::from_raw_parts_mut(b, self.size * self.el);
                gca_cycle_left(buf, self.size, self.el, self.start - newstart);
            }
        }
        self.start = newstart;
    }
}
fn get(&mut self, idx: usize) -> &mut [u8] {
    let pos = (self.start + idx) & self.mask;
    let offset = self.el * pos;
    unsafe { slice::from_raw_parts_mut(self.b.as_ptr().add(offset), self.el) }
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
    if n <= 1 || shift == 0 { return; }
    let shift = shift % n;
    if shift == 0 { return; }
    let gcd = gca_calc_gcd(n as u32, shift as u32) as usize;
    for i in 0..gcd {
        let mut tmp = vec![0u8; es];
        tmp.copy_from_slice(&ptr[es * i..es * i + es]);
        let mut j = i;
        loop {
            let mut k = j + shift;
            if k >= n { k -= n; }
            if k == i { break; }
            let (src_start, dst_start) = (es * k, es * j);
            if src_start < dst_start {
                let (left, right) = ptr.split_at_mut(dst_start);
                right[..es].copy_from_slice(&left[src_start..src_start + es]);
            } else {
                let (left, right) = ptr.split_at_mut(src_start);
                left[dst_start..dst_start + es].copy_from_slice(&right[..es]);
            }
            j = k;
        }
        ptr[es * j..es * j + es].copy_from_slice(&tmp);
    }
}
fn gca_calc_gcd(mut a: u32, mut b: u32) -> u32 {
    if a == 0 { return b; }
    if b == 0 { return a; }
    let mut shift = 0u32;
    while (a | b) & 1 == 0 { a >>= 1; b >>= 1; shift += 1; }
    while a & 1 == 0 { a >>= 1; }
    loop {
        while b & 1 == 0 { b >>= 1; }
        if a > b { std::mem::swap(&mut a, &mut b); }
        b -= a;
        if b == 0 { break; }
    }
    a << shift
}
