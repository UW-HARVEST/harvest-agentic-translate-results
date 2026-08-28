use std::ffi::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flip_horizontal(img: *mut CpImage) {
    let pix = unsafe { (*img).pix };
    let w = unsafe { (*img).w };
    let h = unsafe { (*img).h };
    let flips = h / 2;

    let mut i = 0;
    while i < flips {
        let mut a = unsafe { pix.offset((w * i) as isize) };
        let mut b = unsafe { pix.offset((w * (h - i - 1)) as isize) };

        let mut j = 0;
        while j < w {
            let t = unsafe { a.read() };
            unsafe {
                a.write(b.read());
                b.write(t);
                a = a.add(1);
                b = b.add(1);
            }
            j += 1;
        }
        i += 1;
    }
}
