use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn tfm(dest: *mut f32, src: *const f32, count: c_int) {
    if count <= 0 {
        return;
    }
    let count = count as usize;
    let dest_slice = unsafe { std::slice::from_raw_parts_mut(dest, count * 2) };
    let src_slice = unsafe { std::slice::from_raw_parts(src, count * 3) };

    for i in 0..count {
        let src_chunk = &src_slice[i * 3..i * 3 + 3];
        let dest_chunk = &mut dest_slice[i * 2..i * 2 + 2];

        if src_chunk[0] < src_chunk[1] {
            let dx2 = src_chunk[0];
            let dy2 = src_chunk[1];
            let dxy = src_chunk[2];
            let sqd = (dy2 * dy2) - (2.0 * dx2 * dy2) + (dx2 * dx2) + (4.0 * dxy * dxy);
            let sqd_val = if 0.0 > sqd { 0.0 } else { sqd };
            let lambda = 0.5 * (dy2 + dx2 + sqd_val.sqrt());
            dest_chunk[0] = dx2 - lambda;
            dest_chunk[1] = dxy;
        } else {
            let dy2 = src_chunk[0];
            let dx2 = src_chunk[1];
            let dxy = src_chunk[2];
            let sqd = (dy2 * dy2) - (2.0 * dx2 * dy2) + (dx2 * dx2) + (4.0 * dxy * dxy);
            let sqd_val = if 0.0 > sqd { 0.0 } else { sqd };
            let lambda = 0.5 * (dy2 + dx2 + sqd_val.sqrt());
            dest_chunk[0] = dxy;
            dest_chunk[1] = dx2 - lambda;
        }
    }
}
