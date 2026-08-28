use std::ffi::c_int;

const STATUS_ERROR: c_int = 0o2;

fn decimal_len(value: c_int) -> c_int {
    if value == 0 {
        return 1;
    }

    let mut magnitude = i64::from(value);
    let mut len = 0;
    if magnitude < 0 {
        len += 1;
        magnitude = -magnitude;
    }

    while magnitude != 0 {
        len += 1;
        magnitude /= 10;
    }
    len
}

#[unsafe(no_mangle)]
pub extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    match operation_mode {
        0o1 => STATUS_ERROR | 0o20,
        0o2 => STATUS_ERROR | 0o40,
        0o3 => {
            let buffer_len = 5 + decimal_len(node_id) + 7 + decimal_len(depth);
            buffer_len * 2 + 0o10 + (flags & 0o177)
        }
        0o4 => STATUS_ERROR | 0o100,
        _ => STATUS_ERROR | 0o200,
    }
}
