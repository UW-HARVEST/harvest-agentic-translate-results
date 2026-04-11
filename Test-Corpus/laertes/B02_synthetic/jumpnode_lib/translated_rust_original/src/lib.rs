extern "C" {
    fn sprintf(
        __s: *mut libc::c_char,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn strlen(__s: *const libc::c_char) -> size_t;
    fn sqrt(__x: libc::c_double) -> libc::c_double;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Node {
    pub id: libc::c_int,
    pub parent_id: libc::c_int,
    pub value: libc::c_double,
    pub data: [libc::c_int; 4],
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
static mut node_storage: [Node; 100] = [Node {
    id: 0,
    parent_id: 0,
    value: 0.,
    data: [0; 4],
}; 100];
static mut node_count: libc::c_int = 0 as libc::c_int;
pub const STATUS_ERROR: libc::c_int = 0o2 as libc::c_int;
unsafe extern "C" fn find_node_by_id(mut id: libc::c_int) -> *mut Node {
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < node_count {
        if node_storage[i as usize].id == id {
            return (&raw mut node_storage as *mut Node).offset(i as isize) as *mut Node;
        }
        i += 1;
    }
    return std::ptr::null_mut::<Node>();
}
unsafe extern "C" fn process_backward(
    mut array: *mut libc::c_int,
    mut size: size_t,
    mut start_offset: libc::c_int,
) -> libc::c_int {
    let mut sum: libc::c_int = 0 as libc::c_int;
    let mut ptr: *mut libc::c_int = std::ptr::null_mut::<libc::c_int>();
    let mut start: *mut libc::c_int = std::ptr::null_mut::<libc::c_int>();
    ptr = array.offset(size as isize);
    start = array.offset(start_offset as isize);
    while ptr > start {
        ptr = ptr.offset(-1);
        sum += *ptr;
    }
    return sum;
}
unsafe extern "C" fn compute_size_metric(
    mut str: *const libc::c_char,
) -> libc::c_int {
    let mut len: size_t = strlen(str);
    let mut metric: libc::c_int = 0;
    metric = len as libc::c_int;
    metric = metric * 2 as libc::c_int + 0o10 as libc::c_int;
    return metric;
}
 extern "C" fn safe_double_to_int(mut value: libc::c_double) -> libc::c_int {
    if value > 2147483647.0f64 {
        value = 2147483647.0f64;
    }
    if value < -2147483648.0f64 {
        value = -2147483648.0f64;
    }
    return value as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn jumpnode(
    mut operation_mode: libc::c_int,
    mut node_id: libc::c_int,
    mut depth: libc::c_int,
    mut flags: libc::c_int,
) -> libc::c_int {
    let mut current_node: *mut Node = std::ptr::null_mut::<Node>();
    let mut parent_node: *mut Node = std::ptr::null_mut::<Node>();
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut i: libc::c_int = 0;
    let mut accumulated_value: libc::c_double = 0.;
    let mut temp_array: [libc::c_int; 20] = [0; 20];
    let mut array_size: size_t = 0;
    let mut buffer: [libc::c_char; 50] = [0; 50];
    match operation_mode {
        1 => {
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o20 as libc::c_int;
            }
            accumulated_value = (*current_node).value;
            i = 0 as libc::c_int;
            while i < depth && (*current_node).parent_id != -(1 as libc::c_int) {
                parent_node = find_node_by_id((*current_node).parent_id);
                if parent_node.is_null() {
                    break;
                }
                accumulated_value += (*parent_node).value * 1.5f64;
                current_node = parent_node;
                i += 1;
            }
            result = safe_double_to_int(accumulated_value);
        }
        2 => {
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o40 as libc::c_int;
            }
            i = 0 as libc::c_int;
            while i < 4 as libc::c_int {
                temp_array[i as usize] = (*current_node).data[i as usize];
                i += 1;
            }
            i = 4 as libc::c_int;
            while i < 0o20 as libc::c_int {
                temp_array[i as usize] = i * 0o7 as libc::c_int;
                i += 1;
            }
            array_size = 0o20 as size_t;
            result = process_backward(
                &raw mut temp_array as *mut libc::c_int,
                array_size,
                depth,
            );
            result += array_size as libc::c_int * flags;
        }
        3 => {
            sprintf(
                &raw mut buffer as *mut libc::c_char,
                b"Node_%d_Depth_%d\0" as *const u8 as *const libc::c_char,
                node_id,
                depth,
            );
            result = compute_size_metric(&raw mut buffer as *mut libc::c_char);
            result += flags & 0o177 as libc::c_int;
        }
        4 => {
            current_node = find_node_by_id(node_id);
            if current_node.is_null() {
                return STATUS_ERROR | 0o100 as libc::c_int;
            }
            accumulated_value = 0.0f64;
            i = 0 as libc::c_int;
            while i < 4 as libc::c_int {
                accumulated_value +=
                    sqrt((*current_node).data[i as usize] as libc::c_double)
                        * 2.718281828f64;
                i += 1;
            }
            accumulated_value *= 1.0f64 + depth as libc::c_double * 0.1f64;
            result = safe_double_to_int(accumulated_value);
            if node_count > 2 as libc::c_int {
                let mut end_ptr: *mut Node =
                    (&raw mut node_storage as *mut Node).offset(node_count as isize) as *mut Node;
                let mut iter: *mut Node = end_ptr;
                let mut backward_sum: libc::c_int = 0 as libc::c_int;
                i = 0 as libc::c_int;
                while i < 3 as libc::c_int && iter > &raw mut node_storage as *mut Node {
                    iter = iter.offset(-1);
                    backward_sum += safe_double_to_int((*iter).value);
                    i += 1;
                }
                result += backward_sum;
            }
        }
        _ => {
            result = STATUS_ERROR | 0o200 as libc::c_int;
        }
    }
    return result;
}
