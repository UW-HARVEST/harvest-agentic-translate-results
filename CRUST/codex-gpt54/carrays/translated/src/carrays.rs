use std::cmp::Ordering;
pub fn gca_roundup32(mut x: u32) -> u32 {
    if x == 0 {
        return 0;
    }
    x -= 1;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x + 1
}
pub fn gca_roundup64(mut x: u64) -> u64 {
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
pub fn gca_calc_gcd(mut a: u32, mut b: u32) -> u32 {
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
pub fn gca_capacity<'a>(ptr: &'a mut Vec<u8>, size: &'a mut usize, es: usize, new_size: usize) -> Option<&'a mut Vec<u8>> {
    if new_size > *size {
        let rounded = gca_roundup64(new_size as u64) as usize;
        ptr.resize(rounded.saturating_mul(es), 0);
        *size = rounded;
    }
    Some(ptr)
}
pub fn gca_swapm(a: &mut [u8], b: &mut [u8]) {
    assert_eq!(a.len(), b.len());
    for (lhs, rhs) in a.iter_mut().zip(b.iter_mut()) {
        std::mem::swap(lhs, rhs);
    }
}
pub fn gca_cycle_left(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
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

    for i in 0..(n / 2) {
        let left = i * es;
        let right = (n - 1 - i) * es;
        let (lo, hi) = ptr.split_at_mut(right);
        gca_swapm(&mut lo[left..left + es], &mut hi[..es]);
    }
}
pub fn gca_is_sorted<T, F>(base: &[T], compar: F) -> bool
where
F: Fn(&T, &T) -> Ordering,
{
    base.windows(2)
        .all(|pair| compar(&pair[0], &pair[1]) != Ordering::Greater)
}
pub fn gca_is_rsorted<T, F>(base: &[T], compar: F) -> bool
where
F: Fn(&T, &T) -> Ordering,
{
    base.windows(2)
        .all(|pair| compar(&pair[0], &pair[1]) != Ordering::Less)
}
pub fn gca_max<T, F>(base: &[T], compar: F) -> Option<&T>
where
F: Fn(&T, &T) -> Ordering,
{
    let mut iter = base.iter();
    let mut best = iter.next()?;
    for item in iter {
        if compar(best, item) == Ordering::Less {
            best = item;
        }
    }
    Some(best)
}
pub fn gca_min<T, F>(base: &[T], compar: F) -> Option<&T>
where
F: Fn(&T, &T) -> Ordering,
{
    let mut iter = base.iter();
    let mut best = iter.next()?;
    for item in iter {
        if compar(best, item) == Ordering::Greater {
            best = item;
        }
    }
    Some(best)
}
