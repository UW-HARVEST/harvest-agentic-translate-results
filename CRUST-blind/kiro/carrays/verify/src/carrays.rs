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
    if a == 0 { return b; }
    if b == 0 { return a; }
    let mut shift = 0u32;
    while (a | b) & 1 == 0 {
        a >>= 1;
        b >>= 1;
        shift += 1;
    }
    while a & 1 == 0 { a >>= 1; }
    loop {
        while b & 1 == 0 { b >>= 1; }
        if a > b { mem::swap(&mut a, &mut b); }
        b -= a;
        if b == 0 { break; }
    }
    a << shift
}
pub fn gca_capacity<'a>(ptr: &'a mut Vec<u8>, size: &'a mut usize, es: usize, new_size: usize) -> Option<&'a mut Vec<u8>> {
    if new_size > *size {
        let new_cap = gca_roundup64(new_size as u64) as usize;
        ptr.resize(new_cap * es, 0);
        *size = new_cap;
    }
    Some(ptr)
}
pub fn gca_swapm(a: &mut [u8], b: &mut [u8]) {
    for i in 0..a.len().min(b.len()) {
        let tmp = a[i];
        a[i] = b[i];
        b[i] = tmp;
    }
}
pub fn gca_cycle_left(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
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
pub fn gca_cycle_right(ptr: &mut [u8], n: usize, es: usize, shift: usize) {
    if n == 0 || shift == 0 { return; }
    let shift = shift % n;
    if shift == 0 { return; }
    gca_cycle_left(ptr, n, es, n - shift);
}
pub fn gca_reverse(ptr: &mut [u8], n: usize, es: usize) {
    if n <= 1 || es == 0 { return; }
    let mut i = 0;
    let mut j = n - 1;
    while i < j {
        let (a_start, b_start) = (es * i, es * j);
        let (left, right) = ptr.split_at_mut(b_start);
        let a = &mut left[a_start..a_start + es];
        let b = &mut right[..es];
        gca_swapm(a, b);
        i += 1;
        j -= 1;
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
    base.iter().reduce(|max, x| if compar(max, x) == Ordering::Less { x } else { max })
}
pub fn gca_min<T, F>(base: &[T], compar: F) -> Option<&T>
where
F: Fn(&T, &T) -> Ordering,
{
    base.iter().reduce(|min, x| if compar(min, x) == Ordering::Greater { x } else { min })
}
