use std::ffi::c_int;
use std::os::raw::c_void;

#[repr(C)]
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
pub extern "C" fn flip_horizontal(img: *mut CpImage) {
    unsafe {
        let img_ref = &mut *img;
        let pix = img_ref.pix;
        let w = img_ref.w as usize;
        let h = img_ref.h as usize;
        let flips = h / 2;
        
        for i in 0..flips {
            let a = pix.add(w * i);
            let b = pix.add(w * (h - i - 1));
            
            for j in 0..w {
                let a_ptr = a.add(j);
                let b_ptr = b.add(j);
                
                let t = (*a_ptr).r;
                (*a_ptr).r = (*b_ptr).r;
                (*b_ptr).r = t;
                
                let t = (*a_ptr).g;
                (*a_ptr).g = (*b_ptr).g;
                (*b_ptr).g = t;
                
                let t = (*a_ptr).b;
                (*a_ptr).b = (*b_ptr).b;
                (*b_ptr).b = t;
                
                let t = (*a_ptr).a;
                (*a_ptr).a = (*b_ptr).a;
                (*b_ptr).a = t;
            }
        }
    }
}
