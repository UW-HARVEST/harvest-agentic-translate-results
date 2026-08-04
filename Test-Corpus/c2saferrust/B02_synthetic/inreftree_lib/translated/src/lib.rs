




extern "C" {
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
}
pub type size_t = usize;
pub type Operation = ::core::ffi::c_uint;
pub const OP_MODULO: Operation = 5;
pub const OP_DIVIDE: Operation = 4;
pub const OP_SUBTRACT: Operation = 3;
pub const OP_MULTIPLY: Operation = 2;
pub const OP_ADD: Operation = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TreeNode {
    pub id: ::core::ffi::c_int,
    pub value: ::core::ffi::c_int,
    pub parent_id: ::core::ffi::c_int,
    pub left_child_id: ::core::ffi::c_int,
    pub right_child_id: ::core::ffi::c_int,
    pub label: [::core::ffi::c_char; 32],
}
pub type OperationFunc = Option<
    unsafe extern "C" fn(
        ::core::ffi::c_int,
        ::core::ffi::c_int,
        ::core::ffi::c_int,
        ::core::ffi::c_int,
    ) -> ::core::ffi::c_int,
>;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const MAX_NODES: ::core::ffi::c_int = 50 as ::core::ffi::c_int;
#[no_mangle]
pub static mut node_table: [TreeNode; 50] = [TreeNode {
    id: 0,
    value: 0,
    parent_id: 0,
    left_child_id: 0,
    right_child_id: 0,
    label: [0; 32],
}; 50];
#[no_mangle]
pub static mut node_count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn add_op(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut unused1: ::core::ffi::c_int,
    mut unused2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a + b;
}
#[no_mangle]
pub unsafe extern "C" fn multiply_op(
    a: ::core::ffi::c_int,
    b: ::core::ffi::c_int,
    _unused1: ::core::ffi::c_int,
    _unused2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    a * b
}

#[no_mangle]
pub unsafe extern "C" fn subtract_op(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut unused1: ::core::ffi::c_int,
    mut unused2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a - b;
}
#[no_mangle]
pub unsafe extern "C" fn divide_op(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut unused1: ::core::ffi::c_int,
    mut unused2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if b == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    return a / b;
}
#[no_mangle]
pub unsafe extern "C" fn modulo_op(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut unused1: ::core::ffi::c_int,
    mut unused2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if b == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    return a % b;
}
#[no_mangle]
pub unsafe extern "C" fn find_node_by_id(id: i32) -> *mut TreeNode {
    node_table[..node_count as usize]
        .iter_mut()
        .find(|node| node.id == id)
        .map_or(::core::ptr::null_mut(), |node| node as *mut TreeNode)
}

#[no_mangle]
pub fn add_tree_node(
    id: i32,
    value: i32,
    parent_id: i32,
    label: &str,
) -> i32 {
    unsafe {
        if node_count >= MAX_NODES {
            return -1;
        }

        if parent_id != -1 {
            let parent = find_node_by_id(parent_id);
            if parent.is_null() || (*parent).id != parent_id {
                return -1;
            }
        }

        let index = node_count as usize;
        let node = &mut node_table[index];
        node.id = id;
        node.value = value;
        node.parent_id = parent_id;
        node.left_child_id = -1;
        node.right_child_id = -1;

        node.label.fill(0);
        for (dst, src) in node.label.iter_mut().take(31).zip(label.bytes()) {
            *dst = src as ::core::ffi::c_char;
        }

        if parent_id != -1 {
            let parent = &mut *find_node_by_id(parent_id);
            if parent.left_child_id == -1 {
                parent.left_child_id = id;
            } else if parent.right_child_id == -1 {
                parent.right_child_id = id;
            }
        }

        node_count += 1;
        node_count - 1
    }
}

#[no_mangle]
pub unsafe extern "C" fn calculate_tree_sum(mut node_id: ::core::ffi::c_int) -> ::core::ffi::c_int {
    let mut node: *mut TreeNode = find_node_by_id(node_id);
    if node.is_null() || (*node).id != node_id {
        return 0 as ::core::ffi::c_int;
    }
    let mut sum: ::core::ffi::c_int = (*node).value;
    if (*node).left_child_id != -(1 as ::core::ffi::c_int) {
        sum += calculate_tree_sum((*node).left_child_id);
    }
    if (*node).right_child_id != -(1 as ::core::ffi::c_int) {
        sum += calculate_tree_sum((*node).right_child_id);
    }
    return sum;
}
#[no_mangle]
pub fn parse_operation(op_str: &[::core::ffi::c_char]) -> Operation {
    let first = match op_str.first() {
        Some(&ch) => ch,
        None => return OP_ADD,
    };

    if first == '+' as ::core::ffi::c_char {
        OP_ADD
    } else if first == '*' as ::core::ffi::c_char {
        OP_MULTIPLY
    } else if first == '-' as ::core::ffi::c_char {
        OP_SUBTRACT
    } else if first == '/' as ::core::ffi::c_char {
        OP_DIVIDE
    } else if first == '%' as ::core::ffi::c_char {
        OP_MODULO
    } else {
        OP_ADD
    }
}

#[no_mangle]
pub fn get_operation_func(op: Operation) -> OperationFunc {
    match op as i32 {
        1 => Some(add_op),
        2 => Some(multiply_op),
        3 => Some(subtract_op),
        4 => Some(divide_op),
        5 => Some(modulo_op),
        _ => Some(add_op),
    }
}

#[no_mangle]
pub unsafe extern "C" fn inreftree(
    mut param1: ::core::ffi::c_int,
    mut param2: ::core::ffi::c_int,
    mut param3: ::core::ffi::c_int,
    mut param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    node_count = 0 as ::core::ffi::c_int;
    add_tree_node(1, param1, -1, "root");
    add_tree_node(2, param2, 1, "left");
    add_tree_node(3, param3, 1, "right");
    add_tree_node(4, param4, 2, "left-left");
    let mut target_id: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < node_count {
        if !strchr(
            &raw mut (*(&raw mut node_table as *mut TreeNode).offset(i as isize)).label
                as *mut ::core::ffi::c_char,
            'l' as i32,
        )
        .is_null()
        {
            target_id = node_table[i as usize].id;
            break;
        } else {
            i += 1;
        }
    }
    let mut target: *mut TreeNode = find_node_by_id(target_id);
    if target.is_null() || (*target).value == 0 as ::core::ffi::c_int {
        target_id = 1 as ::core::ffi::c_int;
    }
    let mut tree_sum: ::core::ffi::c_int = calculate_tree_sum(1 as ::core::ffi::c_int);
    let mut op_string: *const ::core::ffi::c_char =
        b"+*-%\0" as *const u8 as *const ::core::ffi::c_char;
    let mut op_char: [::core::ffi::c_char; 2] = [
        *op_string.offset((tree_sum % 4 as ::core::ffi::c_int) as isize),
        '\0' as i32 as ::core::ffi::c_char,
    ];
    let mut op: Operation = parse_operation(&op_char);
    let mut op_value: ::core::ffi::c_int = op as ::core::ffi::c_int;
    let mut func: OperationFunc = get_operation_func(op);
    let mut result: ::core::ffi::c_int = func.expect("non-null function pointer")(
        tree_sum,
        target_id,
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    );
    return result;
}
