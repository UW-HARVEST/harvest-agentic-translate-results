// Phase B -- valid-path differential tests for the composed pipeline.
// Rows C30..C37 of CONFIGS.md.
//
// `maxnmin` resets `node_count` to 0 on entry, so these tests do not need a
// pristine pair -- except C36, which is specifically about state interaction.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// C30..C33 -- the axes maxnmin actually branches on
// ---------------------------------------------------------------------------

#[test]
fn c30_maxnmin_residue_cross_product_nonnegative() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC30_5EED);
    // param1 % 6 x param2 % 6, both non-negative, randomized param3/param4
    for r1 in 0..6i32 {
        for r2 in 0..6i32 {
            for _ in 0..400 {
                let p1 = r1 + 6 * rng.range_i32(0, 300_000_000);
                let p2 = r2 + 6 * rng.range_i32(0, 300_000_000);
                let p3 = rng.next_i32();
                let p4 = rng.next_i32();
                assert_eq!(p1.rem_euclid(6), r1);
                assert_eq!(p2.rem_euclid(6), r2);
                eq_i32(
                    "C30",
                    (r1, r2, p1, p2, p3, p4),
                    p.c.maxnmin(p1, p2, p3, p4),
                    p.r.maxnmin(p1, p2, p3, p4),
                );
            }
        }
    }
}

#[test]
fn c31_maxnmin_negative_params_null_node_paths() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC31_5EED);
    // Negative param1/param2 make `(param % 6) + 1 <= 0` for 5 of 6 residues,
    // so `find_node_by_id` returns NULL and whole blocks are skipped.
    for r1 in -5..=0i32 {
        for r2 in -5..=0i32 {
            for _ in 0..400 {
                let p1 = r1 - 6 * rng.range_i32(0, 300_000_000);
                let p2 = r2 - 6 * rng.range_i32(0, 300_000_000);
                let p3 = rng.next_i32();
                let p4 = rng.next_i32();
                eq_i32(
                    "C31",
                    (r1, r2, p1, p2, p3, p4),
                    p.c.maxnmin(p1, p2, p3, p4),
                    p.r.maxnmin(p1, p2, p3, p4),
                );
            }
        }
    }
    // mixed signs
    for _ in 0..20_000 {
        let p1 = rng.range_i32(-30, 30);
        let p2 = -rng.range_i32(0, 30);
        let p3 = rng.range_i32(-4, 4);
        let p4 = rng.range_i32(-9, 9);
        eq_i32(
            "C31/mixed",
            (p1, p2, p3, p4),
            p.c.maxnmin(p1, p2, p3, p4),
            p.r.maxnmin(p1, p2, p3, p4),
        );
    }
}

#[test]
fn c32_maxnmin_param4_residues() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC32_5EED);
    // parent_id = (param4 % 3) + 1 in { 1, 2, 3 } for non-negative param4 and
    // { -1, 0, 1 } for negative. parent_id == -1 hits the seeded root.
    for r4 in -2..=2i32 {
        for _ in 0..3_000 {
            let step = 3 * rng.range_i32(0, 600_000_000);
            let p4 = if r4 >= 0 { r4 + step } else { r4 - step };
            assert_eq!(p4 % 3, r4);
            let p1 = rng.next_i32();
            let p2 = rng.next_i32();
            let p3 = rng.next_i32();
            eq_i32(
                "C32",
                (r4, p4, p1, p2, p3),
                p.c.maxnmin(p1, p2, p3, p4),
                p.r.maxnmin(p1, p2, p3, p4),
            );
        }
    }
    // explicitly: param4 = -2 => parent_id = -1 => matches root's parent_id
    for p1 in 0..6 {
        for p3 in [-2, -1, 0, 1, 2] {
            eq_i32(
                "C32/root-parent",
                (p1, p3),
                p.c.maxnmin(p1, p1, p3, -2),
                p.r.maxnmin(p1, p1, p3, -2),
            );
        }
    }
}

#[test]
fn c33_maxnmin_param3_special_values() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC33_5EED);
    let specials = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000,
        -3,
        -2,
        -1, // param3 + 1 == 0 -> float division by zero
        0,
        1,
        2,
        3,
        1_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &p3 in &specials {
        for _ in 0..3_000 {
            let p1 = rng.next_i32();
            let p2 = rng.next_i32();
            let p4 = rng.next_i32();
            eq_i32(
                "C33",
                (p3, p1, p2, p4),
                p.c.maxnmin(p1, p2, p3, p4),
                p.r.maxnmin(p1, p2, p3, p4),
            );
        }
        // and with small params, where every residue is easy to hit
        for p1 in -7..=7 {
            for p2 in -7..=7 {
                for p4 in [-3, -2, -1, 0, 1, 2, 3] {
                    eq_i32(
                        "C33/small",
                        (p3, p1, p2, p4),
                        p.c.maxnmin(p1, p2, p3, p4),
                        p.r.maxnmin(p1, p2, p3, p4),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C34/C35 -- broad randomized and exhaustive-small sweeps
// ---------------------------------------------------------------------------

#[test]
fn c34_maxnmin_fully_random_i32() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xC34_5EED);
    for _ in 0..200_000 {
        let (a, b, c, d) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        eq_i32("C34", (a, b, c, d), p.c.maxnmin(a, b, c, d), p.r.maxnmin(a, b, c, d));
    }
    // extremes on every axis, all 4^4 combinations of the corner values
    let corners = [i32::MIN, -1, 0, i32::MAX];
    for &a in &corners {
        for &b in &corners {
            for &c in &corners {
                for &d in &corners {
                    eq_i32(
                        "C34/corners",
                        (a, b, c, d),
                        p.c.maxnmin(a, b, c, d),
                        p.r.maxnmin(a, b, c, d),
                    );
                }
            }
        }
    }
}

#[test]
fn c35_maxnmin_exhaustive_small_range() {
    // Every tuple in [-12, 12]^4 == 390 625 cases, covering every sign
    // combination and every residue class of both moduli.
    let p = fresh_pair();
    for a in -12..=12i32 {
        for b in -12..=12i32 {
            for c in -12..=12i32 {
                for d in -12..=12i32 {
                    let (x, y) = (p.c.maxnmin(a, b, c, d), p.r.maxnmin(a, b, c, d));
                    if x != y {
                        panic!("[C35] divergence at ({a},{b},{c},{d}): C {x}, Rust {y}");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C36 -- state interaction / composed pipeline
// ---------------------------------------------------------------------------

#[test]
fn c36_maxnmin_state_interaction() {
    let mut rng = Rng::new(0xC36_5EED);
    for _ in 0..400 {
        let p = fresh_pair();

        // 1. caller pushes its own nodes into pristine storage
        let n = rng.below(20) as usize;
        for i in 0..n {
            let nlen = 1 + rng.below(60) as usize;
            let name = rng.ascii_name(nlen);
            let v = rng.tame_f64();
            let (x, y) = (
                p.c.add_node(i as i32 + 1, -1, &name, v),
                p.r.add_node(i as i32 + 1, -1, &name, v),
            );
            eq_i32("C36/pre-add", i, x, y);
        }

        // 2. maxnmin -- resets node_count to 0 and rewrites indices 0..5
        let (a, b, c, d) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.range_i32(-6, 6),
            rng.next_i32(),
        );
        eq_i32("C36/first", (a, b, c, d), p.c.maxnmin(a, b, c, d), p.r.maxnmin(a, b, c, d));

        // 3. inspect the state maxnmin left behind, via the low-level API
        for id in [-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, i32::MIN, i32::MAX] {
            eq_node("C36/state", id, &p.c.find_snap(id), &p.r.find_snap(id));
            eq_i32("C36/children", id, p.c.children(id), p.r.children(id));
            eq_bits("C36/sum", id, p.c.subtree_bits(id), p.r.subtree_bits(id));
        }

        // 4. append after maxnmin: must land at index 6 (row E38)
        let name = rng.ascii_name(12);
        let v = rng.tame_f64();
        let (x, y) = (
            p.c.add_node(77, 1, &name, v),
            p.r.add_node(77, 1, &name, v),
        );
        eq_i32("C36/post-add", 0, x, y);
        assert_eq!(x, 6, "[C36] expected append at index 6, got {x}");
        eq_i32("C36/children-after", 1, p.c.children(1), p.r.children(1));
        eq_bits("C36/sum-after", 1, p.c.subtree_bits(1), p.r.subtree_bits(1));

        // 5. mutate in place, then call maxnmin again -- the reset must make the
        //    second result independent of everything above
        p.c.set_active(2, 0);
        p.r.set_active(2, 0);
        p.c.set_parent(3, 77);
        p.r.set_parent(3, 77);
        let (a2, b2, c2, d2) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.range_i32(-6, 6),
            rng.next_i32(),
        );
        eq_i32(
            "C36/second",
            (a2, b2, c2, d2),
            p.c.maxnmin(a2, b2, c2, d2),
            p.r.maxnmin(a2, b2, c2, d2),
        );
        // idempotence: same args again must give the same answer on both sides
        eq_i32(
            "C36/repeat",
            (a2, b2, c2, d2),
            p.c.maxnmin(a2, b2, c2, d2),
            p.r.maxnmin(a2, b2, c2, d2),
        );
    }
}

// ---------------------------------------------------------------------------
// C37 -- randomized operation-sequence fuzz across ALL entry points
// ---------------------------------------------------------------------------

/// Shadow model of the library's storage, used only to decide whether calling
/// `calculate_subtree_sum` would recurse forever (the C would stack-overflow;
/// so would the faithful Rust translation, and crashing both proves nothing).
#[derive(Clone)]
struct Model {
    nodes: Vec<(i32, i32, i32)>, // (id, parent_id, active)
}

impl Model {
    fn new() -> Model {
        Model { nodes: Vec::new() }
    }
    fn add(&mut self, id: i32, parent_id: i32) {
        if self.nodes.len() < MAX_NODES {
            self.nodes.push((id, parent_id, 1));
        }
    }
    fn find(&self, id: i32) -> Option<usize> {
        self.nodes.iter().position(|&(i, _, a)| i == id && a != 0)
    }
    fn reset_to_maxnmin_seed(&mut self) {
        self.nodes = vec![(1, -1, 1), (2, 1, 1), (3, 1, 1), (4, 2, 1), (5, 2, 1), (6, 3, 1)];
    }
    /// Would `calculate_subtree_sum(id)` terminate *and* finish quickly? Mirrors
    /// the C recursion: recurse on `node_storage[i].id` for every active node
    /// whose `parent_id` equals the current id, resolving ids through
    /// first-active-match.
    ///
    /// Returns false for two reasons, both of which mean "don't call it":
    ///  * an id is revisited while still on the recursion stack -> the C would
    ///    recurse forever and stack-overflow (so would the faithful Rust);
    ///  * the call count exceeds a budget -> duplicate ids can make the
    ///    traversal terminate but take exponential time.
    fn subtree_terminates(&self, id: i32) -> bool {
        fn go(m: &Model, id: i32, stack: &mut Vec<i32>, budget: &mut i64) -> bool {
            *budget -= 1;
            if *budget < 0 {
                return false;
            }
            if stack.contains(&id) {
                return false; // revisiting an id already on the recursion stack
            }
            if m.find(id).is_none() {
                return true; // returns 0.0 immediately
            }
            stack.push(id);
            for &(cid, pid, act) in &m.nodes {
                if pid == id && act != 0 && !go(m, cid, stack, budget) {
                    stack.pop();
                    return false;
                }
            }
            stack.pop();
            true
        }
        let mut budget = 20_000i64;
        go(self, id, &mut Vec::new(), &mut budget)
    }
}

#[test]
fn c37_random_operation_sequence_fuzz() {
    let mut rng = Rng::new(0xC37_5EED);
    for round in 0..250 {
        let p = fresh_pair();
        let mut m = Model::new();

        for step in 0..300 {
            let ctx = (round, step);
            match rng.below(11) {
                0 | 1 => {
                    let id = rng.range_i32(1, 8);
                    let parent = rng.range_i32(-2, 8);
                    let len = rng.below(60) as usize;
                    let name = if rng.below(3) == 0 {
                        rng.full_byte_name(len)
                    } else {
                        rng.ascii_name(len)
                    };
                    let v = if rng.below(8) == 0 {
                        rng.mixed_f64()
                    } else {
                        rng.tame_f64()
                    };
                    let (x, y) = (
                        p.c.add_node(id, parent, &name, v),
                        p.r.add_node(id, parent, &name, v),
                    );
                    eq_i32("C37/add_node", (ctx, id, parent, len), x, y);
                    m.add(id, parent);
                }
                2 => {
                    let id = rng.range_i32(-3, 9);
                    eq_node("C37/find", (ctx, id), &p.c.find_snap(id), &p.r.find_snap(id));
                }
                3 => {
                    let id = rng.range_i32(-3, 9);
                    eq_i32("C37/children", (ctx, id), p.c.children(id), p.r.children(id));
                }
                4 => {
                    let id = rng.range_i32(-3, 9);
                    if m.subtree_terminates(id) {
                        eq_bits("C37/sum", (ctx, id), p.c.subtree_bits(id), p.r.subtree_bits(id));
                    }
                }
                5 => {
                    let len = rng.below(80) as usize;
                    let s = rng.full_byte_name(len);
                    eq_i32("C37/process", (ctx, len), p.c.process_string(&s), p.r.process_string(&s));
                }
                6 => {
                    let d = rng.mixed_f64();
                    eq_i32("C37/d2i", (ctx, d.to_bits()), p.c.safe_d2i(d), p.r.safe_d2i(d));
                }
                7 => {
                    let (a, b, c, d) = (
                        rng.next_i32(),
                        rng.next_i32(),
                        rng.range_i32(-8, 8),
                        rng.next_i32(),
                    );
                    eq_i32(
                        "C37/maxnmin",
                        (ctx, a, b, c, d),
                        p.c.maxnmin(a, b, c, d),
                        p.r.maxnmin(a, b, c, d),
                    );
                    m.reset_to_maxnmin_seed();
                }
                8 => {
                    let id = rng.range_i32(-3, 9);
                    let act = (rng.below(2) as i32) * (1 + rng.below(3) as i32);
                    let (x, y) = (p.c.set_active(id, act), p.r.set_active(id, act));
                    assert_eq!(x, y, "C37/set_active reachability differs {ctx:?} id {id}");
                    if let Some(k) = m.find(id) {
                        m.nodes[k].2 = act;
                    }
                }
                9 => {
                    let id = rng.range_i32(-3, 9);
                    let parent = rng.range_i32(-2, 9);
                    let (x, y) = (p.c.set_parent(id, parent), p.r.set_parent(id, parent));
                    assert_eq!(x, y, "C37/set_parent reachability differs {ctx:?} id {id}");
                    if let Some(k) = m.find(id) {
                        m.nodes[k].1 = parent;
                    }
                }
                _ => {
                    let id = rng.range_i32(-3, 9);
                    let a = p.c.process_name_in_place(id);
                    let b = p.r.process_name_in_place(id);
                    assert_eq!(
                        a.is_some(),
                        b.is_some(),
                        "C37/process_name reachability differs {ctx:?} id {id}"
                    );
                    if let (Some(a), Some(b)) = (a, b) {
                        eq_i32("C37/process_name", (ctx, id), a, b);
                    }
                }
            }
        }
    }
}
