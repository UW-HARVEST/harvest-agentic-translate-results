pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type tflac_u8 = u8;
pub type tflac_u32 = u32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac_md5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}
impl std::default::Default for tflac_md5 {
    fn default() -> Self {
        tflac_md5 {
        a: u32::default(),
        b: u32::default(),
        c: u32::default(),
        d: u32::default()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn md5_digest<'a1>(mut m: Option<&'a1 crate::src::lib::tflac_md5>, mut out: * mut u8) {
    *out.offset(0 as libc::c_int as isize) = (*m.unwrap()).a as tflac_u8;
    *out.offset(1 as libc::c_int as isize) = ((*(m).clone().unwrap()).a >> 8 as libc::c_int) as tflac_u8;
    *out.offset(2 as libc::c_int as isize) =
        ((*(m).clone().unwrap()).a >> 16 as libc::c_int) as tflac_u8;
    *out.offset(3 as libc::c_int as isize) =
        ((*(m).clone().unwrap()).a >> 24 as libc::c_int) as tflac_u8;
    *out.offset(4 as libc::c_int as isize) = (*m.unwrap()).b as tflac_u8;
    *out.offset(5 as libc::c_int as isize) = ((*(m).clone().unwrap()).b >> 8 as libc::c_int) as tflac_u8;
    *out.offset(6 as libc::c_int as isize) =
        ((*(m).clone().unwrap()).b >> 16 as libc::c_int) as tflac_u8;
    *out.offset(7 as libc::c_int as isize) =
        ((*(m).clone().unwrap()).b >> 24 as libc::c_int) as tflac_u8;
    *out.offset(8 as libc::c_int as isize) = (*m.unwrap()).c as tflac_u8;
    *out.offset(9 as libc::c_int as isize) = ((*(m).clone().unwrap()).c >> 8 as libc::c_int) as tflac_u8;
    *out.offset(10 as libc::c_int as isize) =
        ((*(m).clone().unwrap()).c >> 16 as libc::c_int) as tflac_u8;
    *out.offset(11 as libc::c_int as isize) =
        ((*(m).clone().unwrap()).c >> 24 as libc::c_int) as tflac_u8;
    *out.offset(12 as libc::c_int as isize) = (*m.unwrap()).d as tflac_u8;
    *out.offset(13 as libc::c_int as isize) =
        ((*(m).clone().unwrap()).d >> 8 as libc::c_int) as tflac_u8;
    *out.offset(14 as libc::c_int as isize) =
        ((*(m).clone().unwrap()).d >> 16 as libc::c_int) as tflac_u8;
    *out.offset(15 as libc::c_int as isize) =
        ((*(m).clone().unwrap()).d >> 24 as libc::c_int) as tflac_u8;
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

