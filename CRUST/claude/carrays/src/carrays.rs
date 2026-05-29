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
        let new_size_rounded = gca_roundup64(new_size as u64) as usize;
        ptr.resize(new_size_rounded * es, 0u8);
        *size = new_size_rounded;
    }
    Some(ptr)
}

pub fn gca_swapm(a: &mut [u8], b: &mut [u8]) {
    let len = a.len().min(b.len());
    for i in 0..len {
        mem::swap(&mut a[i], &mut b[i]);
    }
}

pub fn gca_cycle_left(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
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
        // tmp = arr[i]
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
            // arr[j] = arr[k];
            ptr.copy_within(es * k..es * (k + 1), es * j);
            j = k;
        }
        // arr[j] = tmp;
        ptr[es * j..es * (j + 1)].copy_from_slice(&tmp);
    }
}

pub fn gca_cycle_right(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
    if n == 0 || shift == 0 {
        return;
    }
    let shift = shift % n;
    if shift == 0 {
        return;
    }
    gca_cycle_left(ptr, n, es, n - shift);
}

pub fn gca_reverse(ptr: &mut [u8], n: usize, es: usize) {
    if n <= 1 || es == 0 {
        return;
    }
    let mut a = 0usize;
    let mut b = es * (n - 1);
    while a < b {
        // Swap es bytes between [a..a+es] and [b..b+es]
        let (left, right) = ptr.split_at_mut(b);
        let a_slice = &mut left[a..a + es];
        let b_slice = &mut right[0..es];
        a_slice.swap_with_slice(b_slice);
        a += es;
        b -= es;
    }
}

pub fn gca_is_sorted<T, F>(base: &[T], compar: F) -> bool
where
    F: Fn(&T, &T) -> Ordering,
{
    base.windows(2).all(|w| compar(&w[0], &w[1]) != Ordering::Greater)
}

pub fn gca_is_rsorted<T, F>(base: &[T], compar: F) -> bool
where
    F: Fn(&T, &T) -> Ordering,
{
    base.windows(2).all(|w| compar(&w[0], &w[1]) != Ordering::Less)
}

pub fn gca_max<T, F>(base: &[T], compar: F) -> Option<&T>
where
    F: Fn(&T, &T) -> Ordering,
{
    let mut iter = base.iter();
    let mut max = iter.next()?;
    for item in iter {
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
    let mut iter = base.iter();
    let mut min = iter.next()?;
    for item in iter {
        if compar(min, item) == Ordering::Greater {
            min = item;
        }
    }
    Some(min)
}
