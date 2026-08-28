//! Level 2: the functions that read and mutate the module-level node table.
//! Both libraries are driven with an identical operation script and every
//! intermediate result is compared, so state evolution is verified too.
mod harness;

use harness::{impls, reset_all, Api, NodeSnapshot};
use std::ffi::{c_char, c_double, c_int};

#[derive(Debug, Clone)]
enum Op {
    Add(c_int, c_int, Vec<u8>, c_double),
    Find(c_int),
    Children(c_int),
    Subtree(c_int),
    Maxnmin(c_int, c_int, c_int, c_int),
}

#[derive(Debug, PartialEq, Clone)]
enum Out {
    Int(c_int),
    Bits(u64),
    Node(Option<NodeSnapshot>),
}

fn apply(api: &Api, op: &Op) -> Out {
    unsafe {
        match op {
            Op::Add(id, pid, name, value) => {
                let mut buf: Vec<c_char> = name.iter().map(|&b| b as c_char).collect();
                buf.push(0);
                Out::Int((api.add_node)(*id, *pid, buf.as_ptr(), *value))
            }
            Op::Find(id) => Out::Node(api.snapshot((api.find_node_by_id)(*id))),
            Op::Children(pid) => Out::Int((api.get_children_count)(*pid)),
            Op::Subtree(id) => Out::Bits((api.calculate_subtree_sum)(*id).to_bits()),
            Op::Maxnmin(a, b, c, d) => Out::Int((api.maxnmin)(*a, *b, *c, *d)),
        }
    }
}

/// Run `ops` against the C reference and every Rust build, comparing each step.
fn run_script(ops: &[Op]) {
    let i = impls();
    reset_all(&i);
    for (n, op) in ops.iter().enumerate() {
        let expected = apply(&i.c, op);
        for r in &i.rust {
            let got = apply(r, op);
            assert_eq!(
                expected, got,
                "step {n} {op:?}: C={expected:?} {}={got:?}",
                r.label
            );
        }
    }
}

/// The six nodes `maxnmin` installs, as they exist after `reset_all`.
#[test]
fn baseline_tree_is_identical() {
    let mut ops = Vec::new();
    for id in -3..=10 {
        ops.push(Op::Find(id));
        ops.push(Op::Children(id));
    }
    for id in 1..=6 {
        ops.push(Op::Subtree(id));
    }
    // ids with no node: subtree sum must be exactly 0.0
    for id in [-1, 0, 7, 100, c_int::MIN, c_int::MAX] {
        ops.push(Op::Subtree(id));
    }
    run_script(&ops);
}

#[test]
fn add_node_return_values_and_capacity() {
    let mut ops = Vec::new();
    // baseline already holds 6 nodes; push past MAX_NODES (100) to hit the -1 path
    for k in 0..120i32 {
        ops.push(Op::Add(1000 + k, k % 7, format!("n{k}").into_bytes(), k as c_double));
    }
    // once full, further adds keep returning -1 and must not corrupt anything
    for k in 0..5i32 {
        ops.push(Op::Add(5000 + k, -1, b"overflow".to_vec(), 1.0));
        ops.push(Op::Find(5000 + k));
    }
    ops.push(Op::Children(0));
    ops.push(Op::Children(1));
    for id in [1000, 1050, 1093, 1094, 1095, 1119] {
        ops.push(Op::Find(id));
    }
    run_script(&ops);
}

#[test]
fn add_node_name_truncation() {
    let long = vec![b'Z'; 200];
    let exact49 = vec![b'A'; 49];
    let exact50 = vec![b'B'; 50];
    let exact51 = vec![b'C'; 51];
    let ops = vec![
        Op::Add(700, -1, long, 1.0),
        Op::Find(700),
        Op::Add(701, -1, exact49, 2.0),
        Op::Find(701),
        Op::Add(702, -1, exact50, 3.0),
        Op::Find(702),
        Op::Add(703, -1, exact51, 4.0),
        Op::Find(703),
        Op::Add(704, -1, b"".to_vec(), 5.0),
        Op::Find(704),
        // high-bit bytes survive the copy unchanged
        Op::Add(705, -1, vec![0xff, 0x80, 0x7f, 0x41], 6.0),
        Op::Find(705),
        Op::Add(706, -1, (1u8..=60).collect(), 7.0),
        Op::Find(706),
    ];
    run_script(&ops);
}

#[test]
fn duplicate_ids_and_negative_ids() {
    let ops = vec![
        // find_node_by_id returns the *first* match
        Op::Add(42, -1, b"first".to_vec(), 1.0),
        Op::Add(42, -1, b"second".to_vec(), 2.0),
        Op::Find(42),
        Op::Subtree(42),
        Op::Add(-9, -1, b"neg".to_vec(), -3.5),
        Op::Find(-9),
        Op::Children(-1),
        Op::Add(c_int::MIN, c_int::MAX, b"extreme".to_vec(), 0.0),
        Op::Find(c_int::MIN),
        Op::Children(c_int::MAX),
        Op::Add(c_int::MAX, c_int::MIN, b"extreme2".to_vec(), -0.0),
        Op::Find(c_int::MAX),
        Op::Children(c_int::MIN),
        // NB: nodes MIN/MAX are each other's parent, so calculate_subtree_sum
        // on them would recurse forever in the C too -- not exercised.
    ];
    run_script(&ops);
}

/// Floating point accumulation order matters, so build a wide/deep tree and
/// compare exact bit patterns.
#[test]
fn subtree_sum_accumulation_order() {
    let mut ops = Vec::new();
    ops.push(Op::Add(10, -1, b"r".to_vec(), 0.1));
    // wide fan-out with values that do not sum exactly in binary floating point
    for k in 0..30i32 {
        ops.push(Op::Add(100 + k, 10, format!("w{k}").into_bytes(), 0.1 + k as f64 * 0.7));
    }
    // a deep chain hanging off one of them
    let mut prev = 100;
    for k in 0..25i32 {
        let id = 200 + k;
        ops.push(Op::Add(id, prev, format!("d{k}").into_bytes(), 1.0 / (k as f64 + 3.0)));
        prev = id;
    }
    for id in [10, 100, 101, 129, 200, 210, 224] {
        ops.push(Op::Subtree(id));
        ops.push(Op::Children(id));
    }
    run_script(&ops);
}

/// Non-finite and extreme node values propagate through the sum identically.
#[test]
fn subtree_sum_non_finite_values() {
    let ops = vec![
        Op::Add(1, -1, b"root".to_vec(), f64::MAX),
        Op::Add(2, 1, b"a".to_vec(), f64::MAX),
        Op::Add(3, 1, b"b".to_vec(), f64::NEG_INFINITY),
        Op::Add(4, 1, b"c".to_vec(), f64::INFINITY),
        Op::Add(5, 2, b"d".to_vec(), -0.0),
        Op::Add(6, 3, b"e".to_vec(), f64::NAN),
        Op::Add(7, 4, b"f".to_vec(), 5e-324),
        Op::Subtree(1),
        Op::Subtree(2),
        Op::Subtree(3),
        Op::Subtree(4),
        Op::Subtree(5),
        Op::Subtree(6),
        Op::Subtree(7),
        Op::Children(1),
        Op::Children(2),
    ];
    run_script(&ops);
}

/// `maxnmin` resets `node_count` but leaves stale entries in storage; verify
/// both implementations expose the same view before and after that reset.
#[test]
fn maxnmin_reset_leaves_identical_state() {
    let mut ops = Vec::new();
    for k in 0..20i32 {
        ops.push(Op::Add(900 + k, 900 + k - 1, format!("s{k}").into_bytes(), k as f64 * 1.5));
    }
    ops.push(Op::Find(915));
    ops.push(Op::Children(910));
    ops.push(Op::Maxnmin(3, 4, 5, 6));
    // node_count is now 6 again; the stale nodes must be invisible
    for id in [900, 910, 919, 1, 2, 3, 4, 5, 6, 7] {
        ops.push(Op::Find(id));
        ops.push(Op::Children(id));
        if (1..=6).contains(&id) {
            ops.push(Op::Subtree(id));
        }
    }
    // adding again overwrites slot 6 onwards
    ops.push(Op::Add(77, 1, b"reused".to_vec(), 9.25));
    ops.push(Op::Find(77));
    ops.push(Op::Children(1));
    ops.push(Op::Subtree(1));
    run_script(&ops);
}

/// Interleave every operation pseudo-randomly. Parent ids are kept away from
/// self-references so `calculate_subtree_sum` cannot recurse forever.
#[test]
fn randomised_interleaved_script() {
    let mut x: u64 = 0x0bad_c0de_1234_5678;
    let mut next = || {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (x >> 32) as u32
    };
    let mut ops = Vec::new();
    for step in 0..600i32 {
        let r = next();
        match r % 5 {
            0 => {
                // id > parent_id keeps the forest acyclic
                let id = 10 + (next() % 90) as c_int;
                let pid = (next() % 10) as c_int;
                let v = (next() as f64 / 1e6) - 2000.0;
                ops.push(Op::Add(id, pid, format!("r{step}").into_bytes(), v));
            }
            1 => ops.push(Op::Find((next() % 110) as c_int - 5)),
            2 => ops.push(Op::Children((next() % 15) as c_int - 5)),
            3 => ops.push(Op::Subtree((next() % 110) as c_int - 5)),
            _ => ops.push(Op::Maxnmin(
                next() as c_int,
                next() as c_int,
                (next() % 50) as c_int,
                next() as c_int,
            )),
        }
    }
    run_script(&ops);
}

/// The pointer returned by `find_node_by_id` must have the same stride between
/// adjacent slots in both libraries, i.e. identical `Node` layout.
#[test]
fn node_layout_stride_matches() {
    let i = impls();
    reset_all(&i);
    let stride = |api: &Api| unsafe {
        let a = (api.find_node_by_id)(1);
        let b = (api.find_node_by_id)(2);
        assert!(!a.is_null() && !b.is_null(), "{}", api.label);
        (b as isize) - (a as isize)
    };
    let expected = stride(&i.c);
    assert_eq!(expected, std::mem::size_of::<harness::Node>() as isize);
    for r in &i.rust {
        assert_eq!(expected, stride(r), "Node stride differs in {}", r.label);
    }

    // and the full six-node table must be laid out contiguously and identically
    let dump = |api: &Api| unsafe {
        let base = (api.find_node_by_id)(1) as *const u8;
        std::slice::from_raw_parts(base, 6 * std::mem::size_of::<harness::Node>()).to_vec()
    };
    let cbytes = dump(&i.c);
    for r in &i.rust {
        assert_eq!(cbytes, dump(r), "node table bytes differ in {}", r.label);
    }
}
