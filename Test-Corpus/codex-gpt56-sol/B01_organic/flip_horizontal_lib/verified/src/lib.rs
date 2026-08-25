use std::ffi::{c_int, c_void};
use std::mem::{MaybeUninit, size_of};

unsafe extern "C" {
    fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

unsafe fn c_read<T>(source: *const T) -> T {
    let mut value = MaybeUninit::<T>::uninit();
    unsafe {
        memcpy(value.as_mut_ptr().cast(), source.cast(), size_of::<T>());
        value.assume_init()
    }
}

unsafe fn c_write<T>(destination: *mut T, value: *const T) {
    unsafe {
        memcpy(destination.cast(), value.cast(), size_of::<T>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flip_horizontal(img: *mut cp_image_t) {
    unsafe {
        let image = c_read(img);
        let pix = image.pix;
        let w = image.w;
        let h = image.h;
        let flips = h / 2;
        let mut i = 0;

        while i < flips {
            let mut a = pix.wrapping_offset(w.wrapping_mul(i) as isize);
            let mut b = pix.wrapping_offset(w.wrapping_mul(h - i - 1) as isize);
            let mut j = 0;

            while j < w {
                let t = c_read(a);
                let b_value = c_read(b);
                c_write(a, &raw const b_value);
                c_write(b, &raw const t);
                a = a.wrapping_offset(1);
                b = b.wrapping_offset(1);
                j += 1;
            }

            i += 1;
        }
    }
}
