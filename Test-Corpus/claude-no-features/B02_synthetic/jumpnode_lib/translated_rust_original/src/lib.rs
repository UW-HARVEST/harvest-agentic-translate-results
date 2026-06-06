use std::ffi::c_int;

#[derive(Copy, Clone)]
#[repr(C)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

const MAX_NODES: usize = 100;

static mut NODE_STORAGE: [Node; MAX_NODES] = [Node {
    id: 0,
    parent_id: 0,
    value: 0.0,
    data: [0; 4],
}; MAX_NODES];
static mut NODE_COUNT: c_int = 0;

const STATUS_OK: c_int = 0o0;
#[allow(dead_code)]
const STATUS_WARNING: c_int = 0o1;
const STATUS_ERROR: c_int = 0o2;
#[allow(dead_code)]
const STATUS_CRITICAL: c_int = 0o377;

unsafe fn find_node_by_id(id: c_int) -> *mut Node {
    let count = unsafe { NODE_COUNT };
    for i in 0..count as usize {
        unsafe {
            if NODE_STORAGE[i].id == id {
                return &mut NODE_STORAGE[i] as *mut Node;
            }
        }
    }
    std::ptr::null_mut()
}

#[allow(dead_code)]
unsafe fn add_node(id: c_int, parent_id: c_int, value: f64) -> c_int {
    let count = unsafe { NODE_COUNT };
    if count as usize >= MAX_NODES {
        return STATUS_ERROR;
    }

    unsafe {
        let idx = count as usize;
        NODE_STORAGE[idx].id = id;
        NODE_STORAGE[idx].parent_id = parent_id;
        NODE_STORAGE[idx].value = value;

        NODE_STORAGE[idx].data[0] = 0o100;
        NODE_STORAGE[idx].data[1] = 0o200;
        NODE_STORAGE[idx].data[2] = 0o300;
        NODE_STORAGE[idx].data[3] = 0o400;

        NODE_COUNT += 1;
    }
    STATUS_OK
}

fn process_backward(array: &[c_int], size: usize, start_offset: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr: usize = size;
    let start: usize = start_offset as usize;

    while ptr > start {
        ptr -= 1;
        sum = sum.wrapping_add(array[ptr]);
    }

    sum
}

fn compute_size_metric(s: &[u8]) -> c_int {
    // strlen: bytes until first NUL
    let mut len: usize = 0;
    while len < s.len() && s[len] != 0 {
        len += 1;
    }
    let mut metric: c_int = len as c_int;
    metric = metric.wrapping_mul(2).wrapping_add(0o10);
    metric
}

fn safe_double_to_int(mut value: f64) -> c_int {
    if value > 2147483647.0 {
        value = 2147483647.0;
    }
    if value < -2147483648.0 {
        value = -2147483648.0;
    }
    value as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn jumpnode(
    operation_mode: c_int,
    node_id: c_int,
    depth: c_int,
    flags: c_int,
) -> c_int {
    #[allow(unused_assignments)]
    let mut result: c_int = 0;
    let mut accumulated_value: f64;
    let mut temp_array: [c_int; 20] = [0; 20];
    let array_size: usize;
    let mut buffer: [u8; 50] = [0; 50];

    match operation_mode {
        0o1 => {
            let mut current_node = unsafe { find_node_by_id(node_id) };
            if current_node.is_null() {
                return STATUS_ERROR | 0o20;
            }

            accumulated_value = unsafe { (*current_node).value };

            let mut i: c_int = 0;
            while i < depth && unsafe { (*current_node).parent_id } != -1 {
                let parent_node = unsafe { find_node_by_id((*current_node).parent_id) };
                if parent_node.is_null() {
                    break;
                }

                accumulated_value += unsafe { (*parent_node).value } * 1.5;
                current_node = parent_node;
                i += 1;
            }

            result = safe_double_to_int(accumulated_value);
        }
        0o2 => {
            let current_node = unsafe { find_node_by_id(node_id) };
            if current_node.is_null() {
                return STATUS_ERROR | 0o40;
            }

            for i in 0..4 {
                temp_array[i] = unsafe { (*current_node).data[i] };
            }

            for i in 4..0o20 {
                temp_array[i] = (i as c_int).wrapping_mul(0o7);
            }

            array_size = 0o20;

            result = process_backward(&temp_array, array_size, depth);

            result = result.wrapping_add((array_size as c_int).wrapping_mul(flags));
        }
        0o3 => {
            // Reproduce sprintf(buffer, "Node_%d_Depth_%d", node_id, depth)
            let formatted = format!("Node_{}_Depth_{}", node_id, depth);
            let bytes = formatted.as_bytes();
            // Buffer is 50 bytes. C would overflow if too long, but typical input fits.
            let copy_len = bytes.len().min(buffer.len() - 1);
            buffer[..copy_len].copy_from_slice(&bytes[..copy_len]);
            buffer[copy_len] = 0;

            result = compute_size_metric(&buffer);

            result = result.wrapping_add(flags & 0o177);
        }
        0o4 => {
            let current_node = unsafe { find_node_by_id(node_id) };
            if current_node.is_null() {
                return STATUS_ERROR | 0o100;
            }

            accumulated_value = 0.0;
            for i in 0..4 {
                accumulated_value +=
                    (unsafe { (*current_node).data[i] } as f64).sqrt() * 2.718281828;
            }

            accumulated_value *= 1.0 + (depth as f64) * 0.1;

            result = safe_double_to_int(accumulated_value);

            let count = unsafe { NODE_COUNT };
            if count > 2 {
                let mut iter: isize = count as isize;
                let mut backward_sum: c_int = 0;

                let mut i: c_int = 0;
                while i < 3 && iter > 0 {
                    iter -= 1;
                    backward_sum = backward_sum.wrapping_add(safe_double_to_int(unsafe {
                        NODE_STORAGE[iter as usize].value
                    }));
                    i += 1;
                }

                result = result.wrapping_add(backward_sum);
            }
        }
        _ => {
            result = STATUS_ERROR | 0o200;
        }
    }

    result
}
