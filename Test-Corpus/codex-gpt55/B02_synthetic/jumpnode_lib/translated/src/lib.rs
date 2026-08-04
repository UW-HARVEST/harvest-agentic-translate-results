use std::ffi::c_int;

#[derive(Clone, Copy)]
struct Node {
    id: c_int,
    parent_id: c_int,
    value: f64,
    data: [c_int; 4],
}

const MAX_NODES: usize = 100;
const EMPTY_NODE: Node = Node {
    id: 0,
    parent_id: 0,
    value: 0.0,
    data: [0; 4],
};

static mut NODE_STORAGE: [Node; MAX_NODES] = [EMPTY_NODE; MAX_NODES];
static mut NODE_COUNT: c_int = 0;

const STATUS_OK: c_int = 0o0000;
const STATUS_ERROR: c_int = 0o0002;

fn node_count() -> usize {
    unsafe { NODE_COUNT as usize }
}

fn find_node_by_id(id: c_int) -> Option<usize> {
    for i in 0..node_count() {
        let node_id = unsafe { NODE_STORAGE[i].id };
        if node_id == id {
            return Some(i);
        }
    }
    None
}

#[allow(dead_code)]
fn add_node(id: c_int, parent_id: c_int, value: f64) -> c_int {
    if node_count() >= MAX_NODES {
        return STATUS_ERROR;
    }

    unsafe {
        let index = NODE_COUNT as usize;
        NODE_STORAGE[index].id = id;
        NODE_STORAGE[index].parent_id = parent_id;
        NODE_STORAGE[index].value = value;
        NODE_STORAGE[index].data[0] = 0o100;
        NODE_STORAGE[index].data[1] = 0o200;
        NODE_STORAGE[index].data[2] = 0o300;
        NODE_STORAGE[index].data[3] = 0o400;
        NODE_COUNT += 1;
    }

    STATUS_OK
}

fn process_backward(array: &[c_int; 20], size: usize, start_offset: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr = size;
    let start = start_offset as usize;

    while ptr > start {
        ptr -= 1;
        sum = sum.wrapping_add(array[ptr]);
    }

    sum
}

fn compute_size_metric(s: &str) -> c_int {
    let mut metric = s.len() as c_int;
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
    let mut result: c_int;
    let mut temp_array = [0 as c_int; 20];

    match operation_mode {
        0o001 => {
            let Some(mut current_index) = find_node_by_id(node_id) else {
                return STATUS_ERROR | 0o020;
            };

            let mut accumulated_value = unsafe { NODE_STORAGE[current_index].value };
            let mut i = 0;
            while i < depth && unsafe { NODE_STORAGE[current_index].parent_id } != -1 {
                let parent_id = unsafe { NODE_STORAGE[current_index].parent_id };
                let Some(parent_index) = find_node_by_id(parent_id) else {
                    break;
                };

                accumulated_value += unsafe { NODE_STORAGE[parent_index].value } * 1.5;
                current_index = parent_index;
                i += 1;
            }

            result = safe_double_to_int(accumulated_value);
        }
        0o002 => {
            let Some(current_index) = find_node_by_id(node_id) else {
                return STATUS_ERROR | 0o040;
            };

            for i in 0..4 {
                temp_array[i] = unsafe { NODE_STORAGE[current_index].data[i] };
            }

            let mut i = 4;
            while i < 0o20 {
                temp_array[i] = (i as c_int).wrapping_mul(0o007);
                i += 1;
            }

            let array_size = 0o20usize;
            result = process_backward(&temp_array, array_size, depth);
            result = result.wrapping_add((array_size as c_int).wrapping_mul(flags));
        }
        0o003 => {
            let buffer = format!("Node_{}_Depth_{}", node_id, depth);
            result = compute_size_metric(&buffer);
            result = result.wrapping_add(flags & 0o177);
        }
        0o004 => {
            let Some(current_index) = find_node_by_id(node_id) else {
                return STATUS_ERROR | 0o100;
            };

            let mut accumulated_value = 0.0;
            for i in 0..4 {
                accumulated_value +=
                    (unsafe { NODE_STORAGE[current_index].data[i] } as f64).sqrt() * 2.718281828;
            }

            accumulated_value *= 1.0 + depth as f64 * 0.1;
            result = safe_double_to_int(accumulated_value);

            if node_count() > 2 {
                let mut iter = node_count();
                let mut backward_sum: c_int = 0;
                let mut i = 0;

                while i < 3 && iter > 0 {
                    iter -= 1;
                    backward_sum =
                        backward_sum.wrapping_add(safe_double_to_int(unsafe { NODE_STORAGE[iter].value }));
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

#[allow(dead_code)]
fn initialize_test_data() {
    unsafe {
        NODE_COUNT = 0;
    }

    add_node(1, -1, 100.5);
    add_node(2, 1, 50.25);
    add_node(3, 1, 75.75);
    add_node(4, 2, 25.125);
    add_node(5, 2, 30.875);
    add_node(6, 3, 40.0625);
    add_node(7, 4, 12.5);
}
