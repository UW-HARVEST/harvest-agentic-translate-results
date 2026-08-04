


use std::ffi::CString;

use std::ffi::CStr;

extern "C" {
    fn sprintf(
        __s: *mut ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn sqrt(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Node {
    pub id: ::core::ffi::c_int,
    pub parent_id: ::core::ffi::c_int,
    pub value: ::core::ffi::c_double,
    pub data: [::core::ffi::c_int; 4],
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
static mut node_storage: [Node; 100] = [Node {
    id: 0,
    parent_id: 0,
    value: 0.,
    data: [0; 4],
}; 100];
static mut node_count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const STATUS_ERROR: ::core::ffi::c_int = 0o2 as ::core::ffi::c_int;
fn find_node_by_id(id: i32) -> *mut Node {
    unsafe {
        node_storage
            .iter_mut()
            .take(node_count as usize)
            .find(|node| node.id == id)
            .map(|node| node as *mut Node)
            .unwrap_or(std::ptr::null_mut())
    }
}

fn process_backward(array: &[i32], start_offset: i32) -> i32 {
    let start = usize::try_from(start_offset).unwrap_or(0);
    array.get(start..).unwrap_or(&[]).iter().rev().copied().sum()
}

fn compute_size_metric(s: &CStr) -> i32 {
    let len = s.to_bytes().len() as i32;
    len * 2 + 8
}

fn safe_double_to_int(value: f64) -> i32 {
    value.clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

#[no_mangle]
pub fn jumpnode(
    operation_mode: ::core::ffi::c_int,
    node_id: ::core::ffi::c_int,
    depth: ::core::ffi::c_int,
    flags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0;
    let mut accumulated_value: ::core::ffi::c_double = 0.0;
    let mut temp_array: [::core::ffi::c_int; 20] = [0; 20];

    match operation_mode {
        1 => {
            let mut current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o20 as ::core::ffi::c_int;
            }

            unsafe {
                accumulated_value = (*current_node).value;
                let mut i = 0;
                while i < depth && (*current_node).parent_id != -1 {
                    let parent_node = find_node_by_id((*current_node).parent_id);
                    if parent_node.is_null() {
                        break;
                    }
                    accumulated_value += (*parent_node).value * 1.5f64;
                    current_node = parent_node;
                    i += 1;
                }
            }

            result = safe_double_to_int(accumulated_value);
        }
        2 => {
            let current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o40 as ::core::ffi::c_int;
            }

            unsafe {
                temp_array[..4].copy_from_slice(&(*current_node).data);
            }

            for i in 4..0o20usize {
                temp_array[i] = i as ::core::ffi::c_int * 0o7 as ::core::ffi::c_int;
            }

            result = process_backward(&temp_array, depth);
            result += temp_array.len() as ::core::ffi::c_int * flags;
        }
        3 => {
            let buffer = format!("Node_{}_Depth_{}", node_id, depth);
            let c_buffer = std::ffi::CString::new(buffer)
                .expect("formatted node/depth string should not contain interior nulls");
            result = compute_size_metric(c_buffer.as_c_str());
            result += flags & 0o177 as ::core::ffi::c_int;
        }
        4 => {
            let current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o100 as ::core::ffi::c_int;
            }

            unsafe {
                for &value in &(*current_node).data {
                    accumulated_value +=
                        (value as ::core::ffi::c_double).sqrt() * 2.718281828f64;
                }
            }

            accumulated_value *= 1.0f64 + depth as ::core::ffi::c_double * 0.1f64;
            result = safe_double_to_int(accumulated_value);

            unsafe {
                if node_count > 2 {
                    let count = node_count as usize;
                    let backward_sum: ::core::ffi::c_int = node_storage[..count]
                        .iter()
                        .rev()
                        .take(3)
                        .map(|node| safe_double_to_int(node.value))
                        .sum();
                    result += backward_sum;
                }
            }
        }
        _ => {
            result = STATUS_ERROR | 0o200 as ::core::ffi::c_int;
        }
    }

    result
}

