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
            std::mem::swap(&mut a, &mut b);
        }
        b -= a;

        if b == 0 {
            return a << shift;
        }
    }
}
pub fn gca_capacity<'a>(
    ptr: &'a mut Vec<u8>,
    size: &'a mut usize,
    es: usize,
    new_size: usize,
) -> Option<&'a mut Vec<u8>> {
    if new_size > *size {
        let rounded = usize::try_from(gca_roundup64(new_size as u64)).ok()?;
        let new_len = rounded.checked_mul(es)?;
        let old_len = (*size).checked_mul(es)?;
        ptr.resize(new_len, 0);
        if old_len > new_len {
            return None;
        }
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
    if n <= 1 || shift == 0 {
        return;
    }

    let shift = shift % n;
    if shift == 0 {
        return;
    }
    if es == 0 {
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

    let needed = n
        .checked_mul(es)
        .expect("element count overflow in gca_reverse");
    assert!(ptr.len() >= needed);

    let mut left = 0;
    let mut right = n - 1;
    while left < right {
        let a = left * es;
        let b = right * es;
        for offset in 0..es {
            ptr.swap(a + offset, b + offset);
        }
        left += 1;
        right -= 1;
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
    let mut max = base.first()?;
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
    let mut min = base.first()?;
    for item in &base[1..] {
        if compar(min, item) == Ordering::Greater {
            min = item;
        }
    }
    Some(min)
}
