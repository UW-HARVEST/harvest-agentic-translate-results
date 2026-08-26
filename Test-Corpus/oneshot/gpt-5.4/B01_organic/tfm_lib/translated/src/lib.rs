use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn tfm(dest: *mut f32, src: *const f32, count: c_int) {
    if dest.is_null() || src.is_null() || count <= 0 {
        return;
    }

    let count = count as usize;

    unsafe {
        let src = std::slice::from_raw_parts(src, count * 3);
        let dest = std::slice::from_raw_parts_mut(dest, count * 2);

        for i in 0..count {
            let s = &src[i * 3..i * 3 + 3];
            let d = &mut dest[i * 2..i * 2 + 2];

            if s[0] < s[1] {
                let dx2 = s[0];
                let dy2 = s[1];
                let dxy = s[2];
                let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) + (4.0f32 * dxy * dxy);
                let lambda = 0.5f32 * (dy2 + dx2 + sqd.max(0.0).sqrt());
                d[0] = dx2 - lambda;
                d[1] = dxy;
            } else {
                let dy2 = s[0];
                let dx2 = s[1];
                let dxy = s[2];
                let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) + (4.0f32 * dxy * dxy);
                let lambda = 0.5f32 * (dy2 + dx2 + sqd.max(0.0).sqrt());
                d[0] = dxy;
                d[1] = dx2 - lambda;
            }
        }
    }
}
