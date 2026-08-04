#[no_mangle]
pub extern "C" fn encode_quant(
    mut uni: libc::c_int,
    mut step: libc::c_int,
    mut pred: libc::c_int,
    mut tgt: libc::c_int,
    mut tgt2: libc::c_int,
    mut lsbit: libc::c_int,
) -> libc::c_int {
    let mut uni1: libc::c_int = 0;
    let mut uni2: libc::c_int = 0;
    let mut diff: libc::c_int = 0;
    let mut p0: libc::c_int = 0;
    let mut p1: libc::c_int = 0;
    let mut p2: libc::c_int = 0;
    let mut p3: libc::c_int = 0;
    let mut d0: libc::c_int = 0;
    let mut d1: libc::c_int = 0;
    let mut d2: libc::c_int = 0;
    let mut d3: libc::c_int = 0;
    uni1 = uni + 1 as libc::c_int;
    uni2 = uni - 1 as libc::c_int;
    if (uni ^ uni1) & !(7 as libc::c_int) != 0 {
        uni1 = uni;
    }
    if (uni ^ uni2) & !(7 as libc::c_int) != 0 {
        uni2 = uni;
    }
    if lsbit != 0 {
        if lsbit == 4 as libc::c_int {
            uni &= !(1 as libc::c_int);
            uni1 &= !(1 as libc::c_int);
            uni2 &= !(1 as libc::c_int);
            uni |= uni >> 1 as libc::c_int
                & uni >> 2 as libc::c_int
                & 1 as libc::c_int;
            uni1 |= uni1 >> 1 as libc::c_int
                & uni1 >> 2 as libc::c_int
                & 1 as libc::c_int;
            uni2 |= uni2 >> 1 as libc::c_int
                & uni2 >> 2 as libc::c_int
                & 1 as libc::c_int;
        } else if lsbit & 1 as libc::c_int != 0 {
            uni |= 1 as libc::c_int;
            uni1 |= 1 as libc::c_int;
            uni2 |= 1 as libc::c_int;
        } else {
            uni &= !(1 as libc::c_int);
            uni1 &= !(1 as libc::c_int);
            uni2 &= !(1 as libc::c_int);
        }
    }
    diff = (2 as libc::c_int * (uni & 7 as libc::c_int) + 1 as libc::c_int)
        * step
        / 8 as libc::c_int;
    if uni & 8 as libc::c_int != 0 {
        diff = -diff;
    }
    p0 = pred + diff;
    d0 = tgt - p0;
    d0 = d0 ^ d0 >> 31 as libc::c_int;
    diff = (2 as libc::c_int * (uni1 & 7 as libc::c_int) + 1 as libc::c_int)
        * step
        / 8 as libc::c_int;
    if uni1 & 8 as libc::c_int != 0 {
        diff = -diff;
    }
    p1 = pred + diff;
    d1 = tgt - p1;
    d1 = d1 ^ d1 >> 31 as libc::c_int;
    diff = (2 as libc::c_int * (uni2 & 7 as libc::c_int) + 1 as libc::c_int)
        * step
        / 8 as libc::c_int;
    if uni2 & 8 as libc::c_int != 0 {
        diff = -diff;
    }
    p2 = pred + diff;
    d2 = tgt - p2;
    d2 = d2 ^ d2 >> 31 as libc::c_int;
    d3 = tgt2 - p0;
    d3 = d3 ^ d3 >> 31 as libc::c_int;
    d0 += d3 >> 5 as libc::c_int;
    d3 = tgt2 - p1;
    d3 = d3 ^ d3 >> 31 as libc::c_int;
    d1 += d3 >> 5 as libc::c_int;
    d3 = tgt2 - p2;
    d3 = d3 ^ d3 >> 31 as libc::c_int;
    d2 += d3 >> 5 as libc::c_int;
    if d1 < d0 {
        uni = uni1;
    }
    if d2 < d0 {
        uni = uni2;
    }
    return uni;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

