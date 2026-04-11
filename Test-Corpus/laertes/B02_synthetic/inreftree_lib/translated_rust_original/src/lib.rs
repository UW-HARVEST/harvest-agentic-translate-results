extern "C" {
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    fn strchr(__s: *const libc::c_char, __c: libc::c_int)
        -> *mut libc::c_char;
}
pub type size_t = usize;
pub type Operation = libc::c_uint;
pub const OP_MODULO: Operation = 5;
pub const OP_DIVIDE: Operation = 4;
pub const OP_SUBTRACT: Operation = 3;
pub const OP_MULTIPLY: Operation = 2;
pub const OP_ADD: Operation = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TreeNode {
    pub id: libc::c_int,
    pub value: libc::c_int,
    pub parent_id: libc::c_int,
    pub left_child_id: libc::c_int,
    pub right_child_id: libc::c_int,
    pub label: [libc::c_char; 32],
}
pub type OperationFunc = Option<
    unsafe extern "C" fn(
        libc::c_int,
        libc::c_int,
        libc::c_int,
        libc::c_int,
    ) -> libc::c_int,
>;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const MAX_NODES: libc::c_int = 50 as libc::c_int;
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
pub static mut node_count: libc::c_int = 0 as libc::c_int;
#[no_mangle]
pub extern "C" fn add_op(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused1: libc::c_int,
    mut unused2: libc::c_int,
) -> libc::c_int {
    return a + b;
}
#[no_mangle]
pub extern "C" fn multiply_op(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused1: libc::c_int,
    mut unused2: libc::c_int,
) -> libc::c_int {
    return a * b;
}
#[no_mangle]
pub extern "C" fn subtract_op(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused1: libc::c_int,
    mut unused2: libc::c_int,
) -> libc::c_int {
    return a - b;
}
#[no_mangle]
pub extern "C" fn divide_op(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused1: libc::c_int,
    mut unused2: libc::c_int,
) -> libc::c_int {
    if b == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    return a / b;
}
#[no_mangle]
pub extern "C" fn modulo_op(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused1: libc::c_int,
    mut unused2: libc::c_int,
) -> libc::c_int {
    if b == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    return a % b;
}
#[no_mangle]
pub unsafe extern "C" fn find_node_by_id(mut id: libc::c_int) -> *mut TreeNode {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < node_count {
        if node_table[i as usize].id == id {
            return (&raw mut node_table as *mut TreeNode).offset(i as isize) as *mut TreeNode;
        }
        i += 1;
    }
    return std::ptr::null_mut::<TreeNode>();
}
#[no_mangle]
pub unsafe extern "C" fn add_tree_node(
    mut id: libc::c_int,
    mut value: libc::c_int,
    mut parent_id: libc::c_int,
    mut label: *const libc::c_char,
) -> libc::c_int {
    if node_count >= MAX_NODES {
        return -(1 as libc::c_int);
    }
    let mut node: *mut TreeNode =
        (&raw mut node_table as *mut TreeNode).offset(node_count as isize) as *mut TreeNode;
    (*node).id = id;
    (*node).value = value;
    (*node).parent_id = parent_id;
    (*node).left_child_id = -(1 as libc::c_int);
    (*node).right_child_id = -(1 as libc::c_int);
    strncpy(
        &raw mut (*node).label as *mut libc::c_char,
        label,
        31 as size_t,
    );
    (*node).label[31 as libc::c_int as usize] = '\0' as i32 as libc::c_char;
    if parent_id != -(1 as libc::c_int) {
        let mut parent: *mut TreeNode = find_node_by_id(parent_id);
        if parent.is_null() || (*parent).id != parent_id {
            return -(1 as libc::c_int);
        }
        if (*parent).left_child_id == -(1 as libc::c_int) {
            (*parent).left_child_id = id;
        } else if (*parent).right_child_id == -(1 as libc::c_int) {
            (*parent).right_child_id = id;
        }
    }
    node_count += 1;
    return node_count - 1 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn calculate_tree_sum(mut node_id: libc::c_int) -> libc::c_int {
    let mut node: *mut TreeNode = find_node_by_id(node_id);
    if node.is_null() || (*node).id != node_id {
        return 0 as libc::c_int;
    }
    let mut sum: libc::c_int = (*node).value;
    if (*node).left_child_id != -(1 as libc::c_int) {
        sum += calculate_tree_sum((*node).left_child_id);
    }
    if (*node).right_child_id != -(1 as libc::c_int) {
        sum += calculate_tree_sum((*node).right_child_id);
    }
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn parse_operation(mut op_str: *const libc::c_char) -> Operation {
    if op_str.is_null() || !strchr(op_str, '+' as i32).is_null() {
        return OP_ADD;
    }
    if !strchr(op_str, '*' as i32).is_null() {
        return OP_MULTIPLY;
    }
    if !strchr(op_str, '-' as i32).is_null() {
        return OP_SUBTRACT;
    }
    if !strchr(op_str, '/' as i32).is_null() {
        return OP_DIVIDE;
    }
    if !strchr(op_str, '%' as i32).is_null() {
        return OP_MODULO;
    }
    return OP_ADD;
}
#[no_mangle]
pub extern "C" fn get_operation_func(mut op: Operation) -> OperationFunc {
    match op as libc::c_int {
        1 => {
            return Some(
                add_op
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        2 => {
            return Some(
                multiply_op
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        3 => {
            return Some(
                subtract_op
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        4 => {
            return Some(
                divide_op
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        5 => {
            return Some(
                modulo_op
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        _ => {
            return Some(
                add_op
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn inreftree(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    node_count = 0 as libc::c_int;
    add_tree_node(
        1 as libc::c_int,
        param1,
        -(1 as libc::c_int),
        b"root\0" as *const u8 as *const libc::c_char,
    );
    add_tree_node(
        2 as libc::c_int,
        param2,
        1 as libc::c_int,
        b"left\0" as *const u8 as *const libc::c_char,
    );
    add_tree_node(
        3 as libc::c_int,
        param3,
        1 as libc::c_int,
        b"right\0" as *const u8 as *const libc::c_char,
    );
    add_tree_node(
        4 as libc::c_int,
        param4,
        2 as libc::c_int,
        b"left-left\0" as *const u8 as *const libc::c_char,
    );
    let mut target_id: libc::c_int = -(1 as libc::c_int);
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < node_count {
        if !strchr(
            &raw mut (*(&raw mut node_table as *mut TreeNode).offset(i as isize)).label
                as *mut libc::c_char,
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
    if target.is_null() || (*target).value == 0 as libc::c_int {
        target_id = 1 as libc::c_int;
    }
    let mut tree_sum: libc::c_int = calculate_tree_sum(1 as libc::c_int);
    let mut op_string: *const libc::c_char =
        b"+*-%\0" as *const u8 as *const libc::c_char;
    let mut op_char: [libc::c_char; 2] = [
        *op_string.offset((tree_sum % 4 as libc::c_int) as isize),
        '\0' as i32 as libc::c_char,
    ];
    let mut op: Operation = parse_operation(&raw mut op_char as *mut libc::c_char);
    let mut op_value: libc::c_int = op as libc::c_int;
    let mut func: OperationFunc = get_operation_func(op);
    let mut result: libc::c_int = func.expect("non-null function pointer")(
        tree_sum,
        target_id,
        0 as libc::c_int,
        0 as libc::c_int,
    );
    return result;
}
