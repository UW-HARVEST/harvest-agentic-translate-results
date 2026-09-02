// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/lib.c`.
//!
//! The C build globs `src/lib.c` into one shared library that exports 13 public
//! symbols (11 functions, 2 data objects). This crate reproduces all of them
//! with identical linker names, signatures, and observable behavior, including
//! the quirks and undefined-behavior corners of the original.

#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// typedef enum { OP_ADD = 1, ... } Operation;
//
// A C enum whose values all fit in `int` is `int`-sized, so every function that
// takes or returns an `Operation` uses `c_int` at the ABI boundary.
// ---------------------------------------------------------------------------

const OP_ADD: c_int = 1;
const OP_MULTIPLY: c_int = 2;
const OP_SUBTRACT: c_int = 3;
const OP_DIVIDE: c_int = 4;
const OP_MODULO: c_int = 5;

// ---------------------------------------------------------------------------
// typedef struct { ... } TreeNode;
//
// 5 * sizeof(int) + 32 bytes of label = 52 bytes, alignment 4. This matches the
// C layout, confirmed by the C `.so`: node_table spans 0x4060..0x4a88 = 2600
// bytes = 50 * 52.
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

/// `TreeNode node_table[MAX_NODES];` — zero-initialized, so it lands in `.bss`
/// exactly like the C definition.
#[unsafe(no_mangle)]
pub static mut node_table: [TreeNode; MAX_NODES] = [TreeNode {
    id: 0,
    value: 0,
    parent_id: 0,
    left_child_id: 0,
    right_child_id: 0,
    label: [0; 32],
}; MAX_NODES];

/// `int node_count = 0;`
#[unsafe(no_mangle)]
pub static mut node_count: c_int = 0;

/// `typedef int (*OperationFunc)(int a, int b, int unused1, int unused2);`
pub type OperationFunc = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// libc helpers, reimplemented so the exact C semantics are visible and so the
// crate needs no external dependencies.
// ---------------------------------------------------------------------------

/// Base pointer to `node_table`, avoiding a reference to a `static mut`.
#[inline]
fn node_table_ptr() -> *mut TreeNode {
    (&raw mut node_table).cast::<TreeNode>()
}

/// `strchr(3)`: returns a pointer to the first occurrence of `c` in the
/// NUL-terminated string `s`, or NULL. A search for `\0` finds the terminator.
unsafe fn strchr(s: *const c_char, c: c_int) -> *const c_char {
    unsafe {
        let needle = c as u8 as c_char;
        let mut p = s;
        loop {
            let ch = *p;
            if ch == needle {
                return p;
            }
            if ch == 0 {
                return std::ptr::null();
            }
            p = p.add(1);
        }
    }
}

/// `strncpy(3)`: copies at most `n` bytes from `src`, stopping after the NUL,
/// then zero-pads `dst` out to `n` bytes. No NUL is appended when `src` is
/// longer than `n`.
unsafe fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) {
    unsafe {
        let mut i = 0usize;
        while i < n {
            let b = *src.add(i);
            *dst.add(i) = b;
            if b == 0 {
                break;
            }
            i += 1;
        }
        while i < n {
            *dst.add(i) = 0;
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Arithmetic operations.
//
// Signed overflow is undefined in C but wraps on the x86-64 targets this
// library is built for, so the wrapping operators reproduce the compiled
// behavior without introducing Rust panics.
//
// Signed *division* overflow is the exception: `INT_MIN / -1` and
// `INT_MIN % -1` pass the `b == 0` guard and then fault the `idiv` instruction,
// so the C library dies with SIGFPE rather than returning. `wrapping_div` /
// `wrapping_rem` would silently return `INT_MIN` / `0` instead, so the division
// is issued through `idiv` directly to keep that observable behavior.
// ---------------------------------------------------------------------------

/// One `idiv` yielding both quotient (`eax`) and remainder (`edx`), exactly as
/// the C compiler emits for `a / b` and `a % b`. Faults with SIGFPE when the
/// quotient is not representable (`INT_MIN / -1`), matching the C `.so`.
///
/// # Safety
/// `b` must be non-zero; the caller replicates the C code's `b == 0` guard.
#[cfg(target_arch = "x86_64")]
#[inline(never)]
unsafe fn idiv_i32(a: c_int, b: c_int) -> (c_int, c_int) {
    let quot: c_int;
    let rem: c_int;
    unsafe {
        core::arch::asm!(
            "cdq",              // sign-extend eax into edx:eax
            "idiv {divisor:e}", // edx:eax / divisor -> eax (quot), edx (rem)
            divisor = in(reg) b,
            inout("eax") a => quot,
            out("edx") rem,
            options(nomem, nostack),
        );
    }
    (quot, rem)
}

/// Portable fallback for non-x86-64 hosts: wraps instead of faulting.
#[cfg(not(target_arch = "x86_64"))]
#[inline(never)]
unsafe fn idiv_i32(a: c_int, b: c_int) -> (c_int, c_int) {
    (a.wrapping_div(b), a.wrapping_rem(b))
}

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
    unsafe { idiv_i32(a, b).0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_op(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    unsafe { idiv_i32(a, b).1 }
}

// ---------------------------------------------------------------------------
// Tree table management.
// ---------------------------------------------------------------------------

/// Linear scan over the first `node_count` entries; returns NULL when no entry
/// carries `id`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_node_by_id(id: c_int) -> *mut TreeNode {
    unsafe {
        let base = node_table_ptr();
        let mut i: c_int = 0;
        while i < node_count {
            let node = base.add(i as usize);
            if (*node).id == id {
                return node;
            }
            i += 1;
        }
        std::ptr::null_mut()
    }
}

/// Appends a node and links it under `parent_id`.
///
/// Faithful to the C, including the quirk that a failed parent lookup returns
/// -1 *after* the node's fields have already been written into
/// `node_table[node_count]`; `node_count` is simply not incremented, so the
/// half-initialized slot is left behind to be overwritten by the next call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_tree_node(
    id: c_int,
    value: c_int,
    parent_id: c_int,
    label: *const c_char,
) -> c_int {
    unsafe {
        if node_count >= MAX_NODES as c_int {
            return -1;
        }

        let node = node_table_ptr().add(node_count as usize);
        (*node).id = id;
        (*node).value = value;
        (*node).parent_id = parent_id;
        (*node).left_child_id = -1;
        (*node).right_child_id = -1;
        strncpy((&raw mut (*node).label).cast::<c_char>(), label, 31);
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

/// Recursive sum of `node_id` and its descendants. Unknown ids contribute 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_tree_sum(node_id: c_int) -> c_int {
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

// ---------------------------------------------------------------------------
// Operation dispatch.
// ---------------------------------------------------------------------------

/// Maps the first operator character found in `op_str` to an `Operation`.
///
/// The NULL check is folded into the first `if` in the C, so a NULL argument
/// yields `OP_ADD` rather than crashing; the check order is preserved.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_operation(op_str: *const c_char) -> c_int {
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

/// The C switches on `(int)op` and falls back to `add_op`, so out-of-range
/// values are accepted rather than rejected.
#[unsafe(no_mangle)]
pub extern "C" fn get_operation_func(op: c_int) -> OperationFunc {
    match op {
        OP_ADD => add_op,
        OP_MULTIPLY => multiply_op,
        OP_SUBTRACT => subtract_op,
        OP_DIVIDE => divide_op,
        OP_MODULO => modulo_op,
        _ => add_op,
    }
}

// ---------------------------------------------------------------------------
// Entry point.
// ---------------------------------------------------------------------------

/// Byte-for-byte replica of the string-literal block the C compiler emits into
/// `.rodata` for `lib.c`, in emission order: "root", "left", "right",
/// "left-left", "+*-%".
///
/// `inreftree` evaluates `op_string[tree_sum % 4]`. C's `%` truncates toward
/// zero, so a negative `tree_sum` produces a negative index that reads the
/// bytes *preceding* the "+*-%" literal. Keeping the neighbouring literals in
/// the same layout reproduces those out-of-bounds reads exactly instead of
/// guessing at their values. Verified against the built C `.so`, whose
/// `.rodata` holds `root\0left\0right\0left-left\0+*-%\0` with "+*-%" at
/// offset 26.
static RODATA_LITERALS: [u8; 31] = *b"root\0left\0right\0left-left\0+*-%\0";
const OP_STRING_OFFSET: isize = 26;

/// Builds a fixed 4-node tree from the parameters, picks a target node, then
/// selects an arithmetic operation from the tree's sum and applies it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn inreftree(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        // Resets the logical length only; stale bytes from previous calls stay
        // in node_table, matching the C.
        node_count = 0;

        add_tree_node(1, param1, -1, c"root".as_ptr());
        add_tree_node(2, param2, 1, c"left".as_ptr());
        add_tree_node(3, param3, 1, c"right".as_ptr());
        add_tree_node(4, param4, 2, c"left-left".as_ptr());

        // First node whose label contains 'l'. "root" has none, so this settles
        // on node 2 ("left") for the tree built above.
        let mut target_id: c_int = -1;
        let base = node_table_ptr();
        let mut i: c_int = 0;
        while i < node_count {
            let node = base.add(i as usize);
            if !strchr((&raw const (*node).label).cast::<c_char>(), 'l' as c_int).is_null() {
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

        // op_string = "+*-%"; see RODATA_LITERALS for the negative-index case.
        let idx = OP_STRING_OFFSET + tree_sum.wrapping_rem(4) as isize;
        let op_char: [c_char; 2] = [RODATA_LITERALS[idx as usize] as c_char, 0];
        let op = parse_operation(op_char.as_ptr());

        // Computed and discarded by the C; kept for fidelity of the translation.
        let _op_value: c_int = op;

        let func = get_operation_func(op);

        func(tree_sum, target_id, 0, 0)
    }
}
