pub type cb_impairment = libc::c_uint;
pub const cbTritanopia: cb_impairment = 2;
pub const cbDeuteranopia: cb_impairment = 1;
pub const cbProtanopia: cb_impairment = 0;
unsafe extern "C" fn Protanopia<'a1, 'a2, 'a3>(
    mut Red: Option<&'a1 mut libc::c_float>,
    mut Green: Option<&'a2 mut libc::c_float>,
    mut Blue: Option<&'a3 mut libc::c_float>,
) {
    let mut R: libc::c_float = *borrow_mut(&mut Red).unwrap();
    let mut G: libc::c_float = *borrow_mut(&mut Green).unwrap();
    let mut B: libc::c_float = *borrow_mut(&mut Blue).unwrap();
    *borrow_mut(&mut Red).unwrap() = 0.17055699213417f32 * R + 0.82944301379913f32 * G + 2.91188E-9f32 * B;
    *borrow_mut(&mut Green).unwrap() = 0.17055699092998f32 * R + 0.82944300785005f32 * G - 5.98679E-10f32 * B;
    *borrow_mut(&mut Blue).unwrap() = -0.00451714424166f32 * R + 0.00451714427397f32 * G + B;
}
unsafe extern "C" fn Deuteranopia<'a1, 'a2, 'a3>(
    mut Red: Option<&'a1 mut libc::c_float>,
    mut Green: Option<&'a2 mut libc::c_float>,
    mut Blue: Option<&'a3 mut libc::c_float>,
) {
    let mut R: libc::c_float = *borrow_mut(&mut Red).unwrap();
    let mut G: libc::c_float = *borrow_mut(&mut Green).unwrap();
    let mut B: libc::c_float = *borrow_mut(&mut Blue).unwrap();
    *borrow_mut(&mut Red).unwrap() = 0.33066007266046f32 * R + 0.66933992517563f32 * G + 3.559314E-9f32 * B;
    *borrow_mut(&mut Green).unwrap() = 0.33066007387760f32 * R + 0.66933992719147f32 * G - 1.758327E-9f32 * B;
    *borrow_mut(&mut Blue).unwrap() = -0.02785538261323f32 * R + 0.02785538252318f32 * G + B;
}
unsafe extern "C" fn Tritanopia<'a1, 'a2, 'a3>(
    mut Red: Option<&'a1 mut libc::c_float>,
    mut Green: Option<&'a2 mut libc::c_float>,
    mut Blue: Option<&'a3 mut libc::c_float>,
) {
    let mut R: libc::c_float = *borrow_mut(&mut Red).unwrap();
    let mut G: libc::c_float = *borrow_mut(&mut Green).unwrap();
    let mut B: libc::c_float = *borrow_mut(&mut Blue).unwrap();
    *borrow_mut(&mut Red).unwrap() = R + 0.12739886310880f32 * G - 0.12739886341072f32 * B;
    *borrow_mut(&mut Green).unwrap() = -4.486E-11f32 * R + 0.87390929928361f32 * G + 0.12609070101523f32 * B;
    *borrow_mut(&mut Blue).unwrap() = 3.1113E-10f32 * R + 0.87390929725848f32 * G + 0.12609070067115f32 * B;
}
#[no_mangle]
pub unsafe extern "C" fn colourblind<'a1, 'a2, 'a3>(
    mut Impairment: libc::c_uint,
    mut R: Option<&'a1 mut libc::c_float>,
    mut G: Option<&'a2 mut libc::c_float>,
    mut B: Option<&'a3 mut libc::c_float>,
) {
    match Impairment as libc::c_uint {
        0 => {
            Protanopia(borrow_mut(&mut R), borrow_mut(&mut G), borrow_mut(&mut B));
        }
        1 => {
            Deuteranopia(borrow_mut(&mut R), borrow_mut(&mut G), borrow_mut(&mut B));
        }
        2 => {
            Tritanopia(borrow_mut(&mut R), borrow_mut(&mut G), borrow_mut(&mut B));
        }
        _ => {}
    };
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

