// Rust translation of c_src/src/lib.c (MIT Lincoln Laboratory, 2025).
//
// The C library is built as one shared object that globs all of c_src/ and
// exports every non-static definition.  The complete public ABI is:
//
//   functions: add_op, multiply_op, subtract_op, divide_op, modulo_op,
//              find_node_by_id, add_tree_node, calculate_tree_sum,
//              parse_operation, get_operation_func, inreftree
//   objects:   node_table (TreeNode[50], 2600 bytes), node_count (int)
//
// All of them are reproduced below with identical signatures and identical
// (including buggy / implementation-defined) behaviour.

#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int};
use std::ptr;

// ---------------------------------------------------------------------------
// typedef enum { OP_ADD = 1, ... } Operation;
// ---------------------------------------------------------------------------

pub const OP_ADD: c_int = 1;
pub const OP_MULTIPLY: c_int = 2;
pub const OP_SUBTRACT: c_int = 3;
pub const OP_DIVIDE: c_int = 4;
pub const OP_MODULO: c_int = 5;

// ---------------------------------------------------------------------------
// typedef struct { ... } TreeNode;   (5 * int + char[32] == 52 bytes, align 4)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TreeNode {
    pub id: c_int,
    pub value: c_int,
    pub parent_id: c_int,
    pub left_child_id: c_int,
    pub right_child_id: c_int,
    pub label: [c_char; 32],
}

const MAX_NODES: usize = 50;

const EMPTY_NODE: TreeNode = TreeNode {
    id: 0,
    value: 0,
    parent_id: 0,
    left_child_id: 0,
    right_child_id: 0,
    label: [0; 32],
};

// `TreeNode node_table[MAX_NODES];` and `int node_count = 0;` are non-static
// definitions in the C translation unit, so they are part of the exported ABI.
#[unsafe(no_mangle)]
pub static mut node_table: [TreeNode; MAX_NODES] = [EMPTY_NODE; MAX_NODES];

#[unsafe(no_mangle)]
pub static mut node_count: c_int = 0;

#[inline]
fn table_ptr() -> *mut TreeNode {
    ptr::addr_of_mut!(node_table) as *mut TreeNode
}

// ---------------------------------------------------------------------------
// libc, used directly.
//
// `lib.c` includes <string.h> and calls `strchr` / `strncpy`.  Re-declaring and
// calling the very same libc entry points (rather than re-implementing them in
// Rust) is what makes the behaviour identical *by construction*, including the
// cases the C standard leaves undefined:
//
//   * `strncpy(dst, NULL, n)` faults with SIGSEGV inside libc, exactly as it
//     does for the C build.  A hand-written Rust loop would instead trip
//     rustc's debug-only "null pointer dereference" check and abort with
//     SIGABRT, which is a different observable outcome.
//   * `strchr` on the deliberately out-of-bounds `op_string[negative]` byte
//     behaves the same because it is literally the same code.
// ---------------------------------------------------------------------------

extern "C" {
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}

// ---------------------------------------------------------------------------
// Signed division, with gcc's x86-64 code generation.
//
// `divide_op` / `modulo_op` guard against `b == 0`, but *not* against the other
// overflowing case, `INT_MIN / -1`.  In C that is undefined behaviour; gcc
// emits `cdq; idiv`, and the CPU raises #DE, so the process dies from SIGFPE.
// Executing the same instruction reproduces that exactly (a plain
// `wrapping_div` would silently return `INT_MIN` instead).
// ---------------------------------------------------------------------------

/// Returns `(quotient, remainder)` the way `cdq; idivl` does.  `b` must be
/// non-zero (both callers check).  Traps on `INT_MIN / -1`, as gcc's code does.
#[inline]
fn c_idiv(a: c_int, b: c_int) -> (c_int, c_int) {
    #[cfg(target_arch = "x86_64")]
    {
        let mut quot: i32 = a;
        let rem: i32;
        unsafe {
            core::arch::asm!(
                "cdq",
                "idiv {divisor:e}",
                divisor = in(reg) b,
                inout("eax") quot,
                out("edx") rem,
            );
        }
        (quot, rem)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        if a == c_int::MIN && b == -1 {
            // Same observable outcome as the hardware trap: death by SIGFPE.
            extern "C" {
                fn raise(sig: c_int) -> c_int;
            }
            const SIGFPE: c_int = 8;
            unsafe { raise(SIGFPE) };
        }
        (a.wrapping_div(b), a.wrapping_rem(b))
    }
}

// ---------------------------------------------------------------------------
// Operation implementations
// ---------------------------------------------------------------------------

pub type OperationFunc = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub extern "C" fn add_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    c_idiv(a, b).0
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    c_idiv(a, b).1
}

// ---------------------------------------------------------------------------
// Tree handling
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    unsafe {
        let table = table_ptr();
        let count = node_count;
        let mut i: c_int = 0;
        while i < count {
            let node = table.offset(i as isize);
            if (*node).id == id {
                return node;
            }
            i += 1;
        }
        ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn add_tree_node(
    id: c_int,
    value: c_int,
    parent_id: c_int,
    label: *const c_char,
) -> c_int {
    unsafe {
        if node_count >= MAX_NODES as c_int {
            return -1;
        }

        let node = table_ptr().offset(node_count as isize);
        (*node).id = id;
        (*node).value = value;
        (*node).parent_id = parent_id;
        (*node).left_child_id = -1;
        (*node).right_child_id = -1;
        strncpy(ptr::addr_of_mut!((*node).label) as *mut c_char, label, 31);
        (*node).label[31] = 0;

        if parent_id != -1 {
            let parent = find_node_by_id(parent_id);
            if parent.is_null() || (*parent).id != parent_id {
                return -1;
            }

            if (*parent).left_child_id == -1 {
                (*parent).left_child_id = id;
            } else if (*parent).right_child_id == -1 {
                (*parent).right_child_id = id;
            }
        }

        node_count += 1;
        node_count - 1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
    unsafe {
        let node = find_node_by_id(node_id);

        if node.is_null() || (*node).id != node_id {
            return 0;
        }

        let mut sum = (*node).value;

        if (*node).left_child_id != -1 {
            sum = sum.wrapping_add(calculate_tree_sum((*node).left_child_id));
        }

        if (*node).right_child_id != -1 {
            sum = sum.wrapping_add(calculate_tree_sum((*node).right_child_id));
        }

        sum
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
    unsafe {
        if op_str.is_null() || !strchr(op_str, '+' as c_int).is_null() {
            return OP_ADD;
        }
        if !strchr(op_str, '*' as c_int).is_null() {
            return OP_MULTIPLY;
        }
        if !strchr(op_str, '-' as c_int).is_null() {
            return OP_SUBTRACT;
        }
        if !strchr(op_str, '/' as c_int).is_null() {
            return OP_DIVIDE;
        }
        if !strchr(op_str, '%' as c_int).is_null() {
            return OP_MODULO;
        }
        OP_ADD
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_func(op: c_int) -> OperationFunc {
    match op {
        1 => add_op,
        2 => multiply_op,
        3 => subtract_op,
        4 => divide_op,
        5 => modulo_op,
        _ => add_op,
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

// The C translation unit's string literals live contiguously in .rodata in the
// order they are emitted:  "root" "left" "right" "left-left" "+*-%".
// `inreftree` indexes op_string with `tree_sum % 4`, which is negative when
// tree_sum is negative (C truncating remainder), reading the bytes that precede
// the literal.  Keeping the literals in one blob reproduces those reads.
const RODATA: &[u8; 32] = b"root\0left\0right\0left-left\0+*-%\0\0";
const OFF_ROOT: usize = 0;
const OFF_LEFT: usize = 5;
const OFF_RIGHT: usize = 10;
const OFF_LEFT_LEFT: usize = 16;
const OFF_OP_STRING: usize = 26;

#[inline]
fn rodata_at(off: usize) -> *const c_char {
    unsafe { (RODATA.as_ptr() as *const c_char).add(off) }
}

#[unsafe(no_mangle)]
pub extern "C" fn inreftree(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        node_count = 0;

        add_tree_node(1, param1, -1, rodata_at(OFF_ROOT));
        add_tree_node(2, param2, 1, rodata_at(OFF_LEFT));
        add_tree_node(3, param3, 1, rodata_at(OFF_RIGHT));
        add_tree_node(4, param4, 2, rodata_at(OFF_LEFT_LEFT));

        let table = table_ptr();
        let mut target_id: c_int = -1;
        let mut i: c_int = 0;
        while i < node_count {
            let node = table.offset(i as isize);
            if !strchr(ptr::addr_of!((*node).label) as *const c_char, 'l' as c_int).is_null() {
                target_id = (*node).id;
                break;
            }
            i += 1;
        }

        let target = find_node_by_id(target_id);
        if target.is_null() || (*target).value == 0 {
            target_id = 1;
        }

        let tree_sum = calculate_tree_sum(1);

        // const char* op_string = "+*-%";
        // char op_char[2] = {op_string[tree_sum % 4], '\0'};
        let idx = (OFF_OP_STRING as isize) + (tree_sum % 4) as isize;
        let op_char: [c_char; 2] = [RODATA[idx as usize] as c_char, 0];
        let op = parse_operation(op_char.as_ptr());

        let _op_value = op;

        let func = get_operation_func(op);

        func(tree_sum, target_id, 0, 0)
    }
}
