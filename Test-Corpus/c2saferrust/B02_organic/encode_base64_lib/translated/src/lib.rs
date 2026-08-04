

extern "C" {
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
fn encode(u: u8) -> char {
    match u {
        0..=25 => (b'A' + u) as char,
        26..=51 => (b'a' + (u - 26)) as char,
        52..=61 => (b'0' + (u - 52)) as char,
        62 => '+',
        _ => '/',
    }
}

#[no_mangle]
pub fn encode_base64(size: i32, src: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char {
    if src.is_null() {
        return ::core::ptr::null_mut();
    }

    let actual_size = if size == 0 {
        unsafe { strlen(src as *mut ::core::ffi::c_char) as i32 }
    } else {
        size
    };

    if actual_size < 0 {
        return ::core::ptr::null_mut();
    }

    let input = unsafe { ::core::slice::from_raw_parts(src as *const u8, actual_size as usize) };

    let mut out = Vec::with_capacity((actual_size as usize * 4 / 3) + 5);

    let mut i = 0usize;
    while i < input.len() {
        let b1 = input[i];
        let b2 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b3 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        let b4 = b1 >> 2;
        let b5 = ((b1 & 0x03) << 4) | (b2 >> 4);
        let b6 = ((b2 & 0x0f) << 2) | (b3 >> 6);
        let b7 = b3 & 0x3f;

        out.push(unsafe { encode(b4) } as u8);
        out.push(unsafe { encode(b5) } as u8);

        if i + 1 < input.len() {
            out.push(unsafe { encode(b6) } as u8);
        } else {
            out.push(b'=');
        }

        if i + 2 < input.len() {
            out.push(unsafe { encode(b7) } as u8);
        } else {
            out.push(b'=');
        }

        i += 3;
    }

    out.push(0);

    let boxed = out.into_boxed_slice();
    Box::into_raw(boxed) as *mut ::core::ffi::c_char
}

