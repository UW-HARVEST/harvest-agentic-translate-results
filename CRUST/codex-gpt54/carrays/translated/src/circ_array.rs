use std::alloc::{alloc_zeroed, dealloc, realloc, Layout};
use std::ptr::NonNull;
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
    let bytes = el.saturating_mul(size);
    let b = if bytes == 0 {
        NonNull::dangling()
    } else {
        let layout = layout(bytes);
        let ptr = unsafe { alloc_zeroed(layout) };
        NonNull::new(ptr).unwrap_or_else(|| std::alloc::handle_alloc_error(layout))
    };

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
    let bytes = self.el.saturating_mul(self.size);
    if bytes != 0 {
        unsafe {
            dealloc(self.b.as_ptr(), layout(bytes));
        }
    }
    self.b = NonNull::dangling();
    self.start = 0;
    self.n = 0;
    self.size = 0;
    self.mask = 0;
}
pub fn resize(&mut self, size: usize) {
    assert!(size > self.size);
    assert_eq!(size & (size - 1), 0);

    let old_size = self.size;
    let old_mask = self.mask;
    let old_bytes = self.el.saturating_mul(old_size);
    let new_bytes = self.el.saturating_mul(size);

    let new_ptr = unsafe {
        if old_bytes == 0 {
            let layout = layout(new_bytes);
            let ptr = alloc_zeroed(layout);
            NonNull::new(ptr).unwrap_or_else(|| std::alloc::handle_alloc_error(layout))
        } else {
            let old_layout = layout(old_bytes);
            let ptr = realloc(self.b.as_ptr(), old_layout, new_bytes);
            let ptr = NonNull::new(ptr).unwrap_or_else(|| std::alloc::handle_alloc_error(old_layout));
            std::ptr::write_bytes(ptr.as_ptr().add(old_bytes), 0, new_bytes - old_bytes);
            ptr
        }
    };
    self.b = new_ptr;

    if self.start + self.n > old_size && old_size != 0 {
        let nend = old_size - self.start;
        let nbeg = (self.start + self.n) & old_mask;
        unsafe {
            if nend < nbeg {
                std::ptr::copy(
                    self.b.as_ptr().add(self.el * (old_size - nend)),
                    self.b.as_ptr().add(self.el * (size - nend)),
                    self.el * nend,
                );
            } else {
                std::ptr::copy(
                    self.b.as_ptr(),
                    self.b.as_ptr().add(self.el * old_size),
                    self.el * nbeg,
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
        self.resize(if self.size == 0 { 1 } else { self.size * 2 });
    }
    self.start = if self.start != 0 {
        self.start - 1
    } else {
        self.size - 1
    };
    self.n += 1;
    let ptr = self.get(0);
    ptr.fill(0);
    ptr
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
    if self.n == self.size {
        self.resize(if self.size == 0 { 1 } else { self.size * 2 });
    }
    let idx = self.n;
    self.n += 1;
    let ptr = self.get(idx);
    ptr.fill(0);
    ptr
}
pub fn shift(&mut self) -> &mut [u8] {
    assert!(self.n > 0);
    let offset = self.el * (((self.start + self.n) & self.mask));
    self.n -= 1;
    unsafe { slice::from_raw_parts_mut(self.b.as_ptr().add(offset), self.el) }
}
pub fn norm(&mut self) {
    if self.start + self.n > self.size {
        let newstart = (self.size - self.n) / 2;
        let nleft = self.start + self.n - self.size;
        let nright = self.size - self.start;

        if nleft <= newstart {
            unsafe {
                std::ptr::copy(
                    self.b.as_ptr().add(self.el * self.start),
                    self.b.as_ptr().add(self.el * newstart),
                    self.el * nright,
                );
                std::ptr::copy_nonoverlapping(
                    self.b.as_ptr(),
                    self.b.as_ptr().add(self.el * (newstart + nright)),
                    self.el * nleft,
                );
            }
        } else {
            let shift = self.start - newstart;
            let bytes = self.size * self.el;
            unsafe {
                let buf = slice::from_raw_parts_mut(self.b.as_ptr(), bytes);
                gca_cycle_left(buf, self.size, self.el, shift);
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
    if x == 0 {
        return 0;
    }
    x -= 1;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x |= x >> 32;
    x + 1
}
fn gca_cycle_left(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
    if n <= 1 || shift == 0 || es == 0 {
        return;
    }

    let shift = shift % n;
    if shift == 0 {
        return;
    }

    let gcd = gca_calc_gcd(n as u32, shift as u32) as usize;
    for i in 0..gcd {
        let start = i * es;
        let tmp = ptr[start..start + es].to_vec();
        let mut j = i;
        loop {
            let mut k = j + shift;
            if k >= n {
                k -= n;
            }
            if k == i {
                break;
            }
            let src = k * es;
            let dst = j * es;
            let src_buf = ptr[src..src + es].to_vec();
            ptr[dst..dst + es].copy_from_slice(&src_buf);
            j = k;
        }
        let dst = j * es;
        ptr[dst..dst + es].copy_from_slice(&tmp);
    }
}
fn gca_calc_gcd(mut a: u32, mut b: u32) -> u32 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }

    let mut shift = 0;
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
            std::mem::swap(&mut a, &mut b);
        }
        b -= a;
        if b == 0 {
            break;
        }
    }

    a << shift
}

fn layout(bytes: usize) -> Layout {
    Layout::array::<u8>(bytes).unwrap()
}
