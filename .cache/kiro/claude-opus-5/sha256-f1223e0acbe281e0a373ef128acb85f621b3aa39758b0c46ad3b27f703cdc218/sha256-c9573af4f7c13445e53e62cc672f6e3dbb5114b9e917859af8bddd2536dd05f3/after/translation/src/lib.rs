// Rust translation of c_src/src/lib.c
//
// Original C copyright header retained for provenance:
//
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
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_double, c_int};

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

/// Mirrors the C `Node` struct exactly:
///
/// ```c
/// typedef struct {
///     int id;
///     int parent_id;
///     char name[MAX_NAME_LEN];
///     double value;
///     int active;
/// } Node;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Node {
    pub id: c_int,
    pub parent_id: c_int,
    pub name: [c_char; MAX_NAME_LEN],
    pub value: c_double,
    pub active: c_int,
}

impl Node {
    const ZEROED: Node = Node {
        id: 0,
        parent_id: 0,
        name: [0; MAX_NAME_LEN],
        value: 0.0,
        active: 0,
    };
}

// `static Node node_storage[MAX_NODES];` and `static int node_count = 0;`
// Not exported (they were `static` in C), but they hold process-wide state that
// persists across calls exactly as the C globals do.
static mut NODE_STORAGE: [Node; MAX_NODES] = [Node::ZEROED; MAX_NODES];
static mut NODE_COUNT: c_int = 0;

#[inline]
fn storage() -> &'static mut [Node; MAX_NODES] {
    // Single-threaded access pattern matching the C library's use of file-scope
    // globals. Freshly derived from the raw pointer on each call so no long-lived
    // aliasing reference is held.
    unsafe { &mut *(&raw mut NODE_STORAGE) }
}

#[inline]
fn node_count() -> c_int {
    unsafe { *(&raw const NODE_COUNT) }
}

#[inline]
fn set_node_count(v: c_int) {
    unsafe { *(&raw mut NODE_COUNT) = v };
}

/// `int add_node(int id, int parent_id, const char *name, double value)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_node(
    id: c_int,
    parent_id: c_int,
    name: *const c_char,
    value: c_double,
) -> c_int {
    let count = node_count();
    if count as usize >= MAX_NODES {
        return -1;
    }

    // Designated initializer zero-fills `name`; the unmentioned bytes stay NUL.
    let mut new_node = Node {
        id,
        parent_id,
        name: [0; MAX_NAME_LEN],
        value,
        active: 1,
    };

    // strncpy(new_node.name, name, MAX_NAME_LEN - 1);
    // Copies at most 49 bytes, stopping at (and not copying past) the source NUL,
    // NUL-padding the remainder -- which is already zero above.
    unsafe {
        let mut i = 0usize;
        while i < MAX_NAME_LEN - 1 {
            let ch = *name.add(i);
            if ch == 0 {
                break;
            }
            new_node.name[i] = ch;
            i += 1;
        }
    }
    // new_node.name[MAX_NAME_LEN - 1] = '\0';
    new_node.name[MAX_NAME_LEN - 1] = 0;

    storage()[count as usize] = new_node;
    set_node_count(count.wrapping_add(1));
    node_count().wrapping_sub(1)
}

/// `Node* find_node_by_id(int id)`
#[unsafe(no_mangle)]
pub extern "C" fn find_node_by_id(id: c_int) -> *mut Node {
    let count = node_count();
    let nodes = storage();
    let mut i: c_int = 0;
    while i < count {
        let n = &mut nodes[i as usize];
        if n.id == id && n.active != 0 {
            return n as *mut Node;
        }
        i += 1;
    }
    std::ptr::null_mut()
}

/// `int get_children_count(int parent_id)`
#[unsafe(no_mangle)]
pub extern "C" fn get_children_count(parent_id: c_int) -> c_int {
    let mut count: c_int = 0;
    let total = node_count();
    let nodes = storage();
    let mut i: c_int = 0;
    while i < total {
        let n = &nodes[i as usize];
        if n.parent_id == parent_id && n.active != 0 {
            count = count.wrapping_add(1);
        }
        i += 1;
    }
    count
}

/// x86-64 SSE `addsd dst, src` semantics: `dst + src`, but with the hardware's
/// NaN-propagation rule spelled out.
///
/// The C statement being translated is
///
/// ```c
/// sum += calculate_subtree_sum(node_storage[i].id);
/// ```
///
/// gcc leaves the callee's return value in `%xmm0` and emits
/// `addsd %xmm1,%xmm0` — i.e. the *destination* register holds the **child's**
/// subtree sum and the *source* holds the running accumulator. `addsd` returns
/// the destination operand (quieted) when it is a NaN, and only otherwise falls
/// back to the source operand, so the child's NaN sign and payload win over the
/// accumulator's.
///
/// Writing `child + sum` in Rust would express that at the source level, but
/// LLVM treats `fadd` as commutative and is free to canonicalise the operand
/// order, so the rule is written out explicitly here. When neither operand is a
/// NaN this is a plain `addsd` and any NaN *created* by the addition
/// (`inf + -inf`) is the hardware's "indefinite" QNaN on both sides.
#[inline]
fn addsd(dst: c_double, src: c_double) -> c_double {
    /// Bit 51 — setting it turns a signalling NaN into the quiet NaN with the
    /// same sign and payload, and is a no-op for an already-quiet NaN.
    const QUIET_BIT: u64 = 0x0008_0000_0000_0000;

    if dst.is_nan() {
        return c_double::from_bits(dst.to_bits() | QUIET_BIT);
    }
    if src.is_nan() {
        return c_double::from_bits(src.to_bits() | QUIET_BIT);
    }
    dst + src
}

/// `double calculate_subtree_sum(int node_id)`
///
/// Recursive, exactly as in C -- including the fact that a parent/child cycle
/// would recurse forever. Behaviour is reproduced, not "fixed".
///
/// The accumulation goes through [`addsd`] rather than `sum += child` so that
/// NaN payload/sign propagation matches the C build bit-for-bit; see the
/// comment on [`addsd`].
#[unsafe(no_mangle)]
pub extern "C" fn calculate_subtree_sum(node_id: c_int) -> c_double {
    let node = find_node_by_id(node_id);
    if node.is_null() {
        return 0.0;
    }

    let mut sum: c_double = unsafe { (*node).value };

    let total = node_count();
    let mut i: c_int = 0;
    while i < total {
        let (parent_id, active, child_id) = {
            let n = &storage()[i as usize];
            (n.parent_id, n.active, n.id)
        };
        if parent_id == node_id && active != 0 {
            let child = calculate_subtree_sum(child_id);
            sum = addsd(child, sum);
        }
        i += 1;
    }

    sum
}

/// `int process_string(char *str)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str: *mut c_char) -> c_int {
    let mut result: c_int = 0;

    unsafe {
        let mut p = str;
        if *p != 0 {
            while *p != 0 {
                // `char` is signed on the target ABI, so this sign-extends.
                result = result.wrapping_add(*p as c_int);
                p = p.add(1);
            }
        }
    }

    result
}

/// `int safe_double_to_int(double d)`
///
/// Note the C order of checks: the range clamps happen *before* the NaN test,
/// so NaN falls through both comparisons (all false) and yields 0.
#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d < c_int::MIN as c_double {
        return c_int::MIN;
    }

    if d != d {
        return 0;
    }

    // Guaranteed in-range here, so `as` (saturating) matches C truncation.
    d as c_int
}

/// `int maxnmin(int param1, int param2, int param3, int param4)`
#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    set_node_count(0);

    unsafe {
        add_node(1, -1, c"root".as_ptr(), 10.5);
        add_node(2, 1, c"child1".as_ptr(), 20.7);
        add_node(3, 1, c"child2".as_ptr(), 15.3);
        add_node(4, 2, c"grandchild1".as_ptr(), 5.9);
        add_node(5, 2, c"grandchild2".as_ptr(), 8.2);
        add_node(6, 3, c"grandchild3".as_ptr(), 12.4);
    }

    let node_id = param1.wrapping_rem(6).wrapping_add(1);
    let selected_node = find_node_by_id(node_id);

    if !selected_node.is_null() {
        let name_ptr = unsafe { (*selected_node).name.as_mut_ptr() };

        if unsafe { *name_ptr } != 0 {
            result = result.wrapping_add(unsafe { process_string(name_ptr) });
        }

        let subtree_sum = calculate_subtree_sum(node_id);

        let sum_as_int = safe_double_to_int(subtree_sum);
        result = result.wrapping_add(sum_as_int);
    }

    let second_node_id = param2.wrapping_rem(6).wrapping_add(1);
    let second_node = find_node_by_id(second_node_id);

    if !second_node.is_null() {
        let value_multiplied = unsafe { (*second_node).value } * param3 as c_double;

        let converted_value = safe_double_to_int(value_multiplied);
        result = result.wrapping_add(converted_value);
    }

    let parent_id = param4.wrapping_rem(3).wrapping_add(1);
    let children = get_children_count(parent_id);
    result = result.wrapping_add(children.wrapping_mul(10));

    let mut calculation =
        (param1.wrapping_add(param2)) as c_double / (param3.wrapping_add(1)) as c_double;
    calculation *= param4 as c_double;

    let final_calc = safe_double_to_int(calculation);
    result = result.wrapping_add(final_calc);

    result
}
