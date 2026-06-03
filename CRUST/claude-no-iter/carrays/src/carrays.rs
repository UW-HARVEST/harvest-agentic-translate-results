use std::cmp::Ordering;
use std::mem;

pub fn gca_roundup32(mut x: u32) -> u32 {
    x = x.wrapping_sub(1);
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x.wrapping_add(1)
}

pub fn gca_roundup64(mut x: u64) -> u64 {
    x = x.wrapping_sub(1);
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x |= x >> 32;
    x.wrapping_add(1)
}

pub fn gca_calc_gcd(mut a: u32, mut b: u32) -> u32 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }

    // Find power of two divisor
    let mut shift: u32 = 0;
    while ((a | b) & 1) == 0 {
        a >>= 1;
        b >>= 1;
        shift += 1;
    }

    // Remove remaining factors of two from a - they are not common
    while (a & 1) == 0 {
        a >>= 1;
    }

    loop {
        // Remove remaining factors of two from b - they are not common
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

pub fn gca_capacity<'a>(
    ptr: &'a mut Vec<u8>,
    size: &'a mut usize,
    es: usize,
    new_size: usize,
) -> Option<&'a mut Vec<u8>> {
    if new_size > *size {
        let new_size = gca_roundup64(new_size as u64) as usize;
        ptr.resize(new_size * es, 0);
        *size = new_size;
    }
    Some(ptr)
}

pub fn gca_swapm(a: &mut [u8], b: &mut [u8]) {
    let len = a.len().min(b.len());
    a[..len].swap_with_slice(&mut b[..len]);
}

pub fn gca_cycle_left(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
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

pub fn gca_cycle_right(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
    if n == 0 || shift == 0 {
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
    ptr[..total].rotate_right(shift * es);
}

pub fn gca_reverse(ptr: &mut [u8], n: usize, es: usize) {
    if n <= 1 || es == 0 {
        return;
    }
    let total = n * es;
    if total > ptr.len() {
        return;
    }
    let slice = &mut ptr[..total];
    let mut left_offset: usize = 0;
    let mut right_offset: usize = (n - 1) * es;
    while left_offset < right_offset {
        // Split the slice into two disjoint halves and swap chunks of `es` bytes
        let (front, back) = slice.split_at_mut(right_offset);
        front[left_offset..left_offset + es].swap_with_slice(&mut back[..es]);
        left_offset += es;
        right_offset -= es;
    }
}

pub fn gca_is_sorted<T, F>(base: &[T], compar: F) -> bool
where
    F: Fn(&T, &T) -> Ordering,
{
    for w in base.windows(2) {
        if compar(&w[0], &w[1]) == Ordering::Greater {
            return false;
        }
    }
    true
}

pub fn gca_is_rsorted<T, F>(base: &[T], compar: F) -> bool
where
    F: Fn(&T, &T) -> Ordering,
{
    for w in base.windows(2) {
        if compar(&w[0], &w[1]) == Ordering::Less {
            return false;
        }
    }
    true
}

pub fn gca_max<T, F>(base: &[T], compar: F) -> Option<&T>
where
    F: Fn(&T, &T) -> Ordering,
{
    if base.is_empty() {
        return None;
    }
    let mut max = &base[0];
    for item in &base[1..] {
        if compar(max, item) == Ordering::Less {
            max = item;
        }
    }
    Some(max)
}

pub fn gca_min<T, F>(base: &[T], compar: F) -> Option<&T>
where
    F: Fn(&T, &T) -> Ordering,
{
    if base.is_empty() {
        return None;
    }
    let mut min = &base[0];
    for item in &base[1..] {
        if compar(min, item) == Ordering::Greater {
            min = item;
        }
    }
    Some(min)
}
