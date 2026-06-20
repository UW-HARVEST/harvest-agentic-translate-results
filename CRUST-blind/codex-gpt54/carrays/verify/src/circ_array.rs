use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
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
        let b = if el == 0 || size == 0 {
            NonNull::dangling()
        } else {
            let layout = Layout::array::<u8>(el.checked_mul(size).expect("buffer size overflow"))
                .expect("invalid layout");
            let ptr = unsafe { alloc(layout) };
            match NonNull::new(ptr) {
                Some(ptr) => ptr,
                None => handle_alloc_error(layout),
            }
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
        if self.el != 0 && self.size != 0 {
            let layout = Layout::array::<u8>(
                self.el
                    .checked_mul(self.size)
                    .expect("buffer size overflow"),
            )
            .expect("invalid layout");
            unsafe { dealloc(self.b.as_ptr(), layout) };
        }
        self.b = NonNull::dangling();
        self.start = 0;
        self.n = 0;
        self.size = 0;
        self.mask = 0;
    }
    pub fn resize(&mut self, size: usize) {
        assert!(size > self.size);
        assert!(size.is_power_of_two());

        if self.el == 0 {
            self.size = size;
            self.mask = size - 1;
            return;
        }

        let new_bytes = self.el.checked_mul(size).expect("buffer size overflow");
        let ptr = if self.size == 0 {
            let layout = Layout::array::<u8>(new_bytes).expect("invalid layout");
            let ptr = unsafe { alloc(layout) };
            match NonNull::new(ptr) {
                Some(ptr) => ptr,
                None => handle_alloc_error(layout),
            }
        } else {
            let old_bytes = self
                .el
                .checked_mul(self.size)
                .expect("buffer size overflow");
            let old_layout = Layout::array::<u8>(old_bytes).expect("invalid layout");
            let ptr = unsafe { realloc(self.b.as_ptr(), old_layout, new_bytes) };
            match NonNull::new(ptr) {
                Some(ptr) => ptr,
                None => handle_alloc_error(old_layout),
            }
        };
        self.b = ptr;

        if self.start + self.n > self.size {
            let nend = self.size - self.start;
            let nbeg = (self.start + self.n) & self.mask;
            unsafe {
                let base = self.b.as_ptr();
                if nend < nbeg {
                    let src = base.add(self.el * (self.size - nend));
                    let dst = base.add(self.el * (size - nend));
                    ptr::copy(src, dst, self.el * nend);
                } else {
                    let src = base;
                    let dst = base.add(self.el * self.size);
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

        let offset = old.checked_mul(self.el).expect("buffer offset overflow");
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
        self.n -= 1;
        self.get(self.n)
    }
    pub fn norm(&mut self) {
        if self.size == 0 || self.start + self.n <= self.size {
            return;
        }

        let newstart = (self.size - self.n) / 2;
        let nleft = self.start + self.n - self.size;
        let nright = self.size - self.start;

        unsafe {
            let base = self.b.as_ptr();
            if nleft <= newstart {
                ptr::copy(
                    base.add(self.el * self.start),
                    base.add(self.el * newstart),
                    self.el * nright,
                );
                ptr::copy_nonoverlapping(
                    base,
                    base.add(self.el * (newstart + nright)),
                    self.el * nleft,
                );
            } else {
                let total = self
                    .el
                    .checked_mul(self.size)
                    .expect("buffer size overflow");
                let buf = slice::from_raw_parts_mut(base, total);
                gca_cycle_left(buf, self.size, self.el, self.start - newstart);
            }
        }

        self.start = newstart;
    }
    fn get(&mut self, idx: usize) -> &mut [u8] {
        if self.el == 0 || self.size == 0 {
            return &mut [];
        }
        let pos = (self.start + idx) & self.mask;
        let offset = pos.checked_mul(self.el).expect("buffer offset overflow");
        unsafe { slice::from_raw_parts_mut(self.b.as_ptr().add(offset), self.el) }
    }
}
fn roundup64(x: u64) -> u64 {
    crate::carrays::gca_roundup64(x)
}
fn gca_cycle_left(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
    if n <= 1 || shift == 0 {
        return;
    }

    let shift = shift % n;
    if shift == 0 || es == 0 {
        return;
    }

    let needed = n
        .checked_mul(es)
        .expect("element count overflow in gca_cycle_left");
    assert!(ptr.len() >= needed);

    let gcd = gca_calc_gcd(n as u32, shift as u32) as usize;
    for i in 0..gcd {
        let mut tmp = vec![0_u8; es];
        let start = i * es;
        tmp.copy_from_slice(&ptr[start..start + es]);

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
            ptr.copy_within(src..src + es, dst);
            j = k;
        }

        let dst = j * es;
        ptr[dst..dst + es].copy_from_slice(&tmp);
    }
}
fn gca_calc_gcd(a: u32, b: u32) -> u32 {
    crate::carrays::gca_calc_gcd(a, b)
}
