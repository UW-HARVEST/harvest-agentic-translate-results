// Phase B -- valid-path differential tests.
//
// One test per row of `CONFIGS.md` (C1..C40). Every row drives BOTH shared
// objects through their exported C symbols with many randomized inputs from a
// fixed seed and compares results bit-for-bit.

mod common;
use common::*;

const INT_MAX: i32 = i32::MAX;
const INT_MIN: i32 = i32::MIN;

/// Moves `x` `k` ULPs away from zero (k>0) / toward zero (k<0).
fn ulp_step(x: f64, k: i64) -> f64 {
    let bits = x.to_bits();
    let sign = bits & (1u64 << 63);
    let mag = (bits & !(1u64 << 63)) as i64;
    let nm = (mag + k).max(0) as u64;
    f64::from_bits(sign | nm)
}

// ===========================================================================
// C1..C8 -- safe_double_to_int (pure; one library pair suffices)
// ===========================================================================

#[test]
fn c1_safe_double_to_int_in_range_positive() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC001);
    // exhaustively-ish sample the whole in-range positive domain
    for _ in 0..20_000 {
        let d = rng.unit() * 2147483648.0;
        both_d2i(&p, "C1", d);
        both_d2i(&p, "C1", d.trunc());
        both_d2i(&p, "C1", d.trunc() + 0.5);
        both_d2i(&p, "C1", d.fract());
    }
    for k in 0..2000 {
        both_d2i(&p, "C1", k as f64 + 0.75);
        both_d2i(&p, "C1", k as f64 * 1_000_003.0 + 0.25);
    }
}

#[test]
fn c2_safe_double_to_int_in_range_negative() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC002);
    for _ in 0..20_000 {
        let d = -(rng.unit() * 2147483648.0);
        both_d2i(&p, "C2", d);
        both_d2i(&p, "C2", d.trunc());
        both_d2i(&p, "C2", d.trunc() - 0.5);
        both_d2i(&p, "C2", -d.fract());
    }
    // truncation direction for negatives: -1.9 -> -1
    for k in 0..2000 {
        both_d2i(&p, "C2", -(k as f64) - 0.9);
        both_d2i(&p, "C2", -(k as f64) - 0.1);
    }
}

#[test]
fn c3_safe_double_to_int_exact_integrals() {
    let p = Pair::fresh();
    both_d2i(&p, "C3", 0.0);
    both_d2i(&p, "C3", -0.0);
    for k in 0..=31u32 {
        let v = 2f64.powi(k as i32);
        both_d2i(&p, "C3", v);
        both_d2i(&p, "C3", -v);
        both_d2i(&p, "C3", v - 1.0);
        both_d2i(&p, "C3", -(v - 1.0));
        both_d2i(&p, "C3", v + 1.0);
        both_d2i(&p, "C3", -(v + 1.0));
    }
    both_d2i(&p, "C3", INT_MAX as f64);
    both_d2i(&p, "C3", INT_MIN as f64);
    for k in -1000..=1000i64 {
        both_d2i(&p, "C3", k as f64);
        both_d2i(&p, "C3", INT_MAX as f64 + k as f64);
        both_d2i(&p, "C3", INT_MIN as f64 + k as f64);
    }
}

#[test]
fn c4_safe_double_to_int_boundary_neighbourhood() {
    let p = Pair::fresh();
    let anchors = [
        INT_MAX as f64,
        INT_MIN as f64,
        2147483648.0,
        -2147483648.0,
        2147483647.5,
        -2147483648.5,
        2147483646.5,
        0.0,
        1.0,
        -1.0,
    ];
    for a in anchors {
        for k in -64..=64i64 {
            both_d2i(&p, "C4", ulp_step(a, k));
        }
        both_d2i(&p, "C4", a + 0.5);
        both_d2i(&p, "C4", a - 0.5);
        both_d2i(&p, "C4", a + 0.9999999);
        both_d2i(&p, "C4", a - 0.9999999);
    }
}

#[test]
fn c5_safe_double_to_int_out_of_range() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC005);
    both_d2i(&p, "C5", f64::INFINITY);
    both_d2i(&p, "C5", f64::NEG_INFINITY);
    both_d2i(&p, "C5", f64::MAX);
    both_d2i(&p, "C5", f64::MIN);
    for e in 31..=308 {
        let v = 10f64.powi(e.min(308));
        both_d2i(&p, "C5", v);
        both_d2i(&p, "C5", -v);
        both_d2i(&p, "C5", 2f64.powi(e.min(1023)));
        both_d2i(&p, "C5", -2f64.powi(e.min(1023)));
    }
    for _ in 0..20_000 {
        let mant = rng.unit() + 1.0;
        let exp = rng.range_i32(31, 1023);
        let v = mant * 2f64.powi(exp);
        both_d2i(&p, "C5", v);
        both_d2i(&p, "C5", -v);
    }
}

#[test]
fn c6_safe_double_to_int_nan_family() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC006);
    both_d2i(&p, "C6", f64::NAN);
    both_d2i(&p, "C6", -f64::NAN);
    // quiet NaN: exponent all ones, mantissa MSB set; signalling: MSB clear
    for _ in 0..20_000 {
        let payload = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        let payload = if payload == 0 { 1 } else { payload };
        for sign in [0u64, 1u64 << 63] {
            let quiet = sign | 0x7FF8_0000_0000_0000 | payload;
            both_d2i(&p, "C6", f64::from_bits(quiet));
            let sig = sign | 0x7FF0_0000_0000_0000 | payload;
            both_d2i(&p, "C6", f64::from_bits(sig));
        }
    }
}

#[test]
fn c7_safe_double_to_int_subnormal_and_tiny() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC007);
    for v in [
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        1e-300,
        -1e-300,
        1e-8,
        -1e-8,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        -0.0,
        0.0,
    ] {
        both_d2i(&p, "C7", v);
    }
    for _ in 0..20_000 {
        let sub = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF; // exponent 0 => subnormal
        both_d2i(&p, "C7", f64::from_bits(sub));
        both_d2i(&p, "C7", f64::from_bits(sub | 1u64 << 63));
        both_d2i(&p, "C7", rng.unit());
        both_d2i(&p, "C7", -rng.unit());
    }
}

#[test]
fn c8_safe_double_to_int_random_bit_patterns() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC008);
    for _ in 0..200_000 {
        both_d2i(&p, "C8", rng.next_f64_bits());
    }
}

// ===========================================================================
// C9..C14 -- process_string (pure)
// ===========================================================================

#[test]
fn c9_process_string_empty() {
    let p = Pair::fresh();
    both_process(&p, "C9", b"\0");
    // several buffers whose first byte is NUL but which have trailing garbage
    both_process(&p, "C9", b"\0abc\0");
    both_process(&p, "C9", b"\0\x7f\xff\0");
}

#[test]
fn c10_process_string_ascii() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC010);
    for _ in 0..5_000 {
        let len = 1 + rng.below(64) as usize;
        let mut v: Vec<u8> = Vec::with_capacity(len + 1);
        for _ in 0..len {
            v.push(0x20 + (rng.below(0x5F) as u8));
        }
        v.push(0);
        both_process(&p, "C10", &v);
    }
}

#[test]
fn c11_process_string_high_bit_bytes() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC011);
    for _ in 0..5_000 {
        let len = 1 + rng.below(256) as usize;
        let mut v: Vec<u8> = Vec::with_capacity(len + 1);
        for _ in 0..len {
            v.push(rng.nonzero_byte());
        }
        v.push(0);
        both_process(&p, "C11", &v);
    }
}

#[test]
fn c12_process_string_homogeneous() {
    let p = Pair::fresh();
    for b in [0x01u8, 0x7f, 0x80, 0xff, 0x41] {
        for len in [1usize, 2, 3, 48, 49, 50, 51, 127, 128, 1000] {
            let mut v = vec![b; len];
            v.push(0);
            both_process(&p, "C12", &v);
        }
    }
}

#[test]
fn c13_process_string_long_buffers() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC013);
    for _ in 0..40 {
        let len = 4096 + rng.below(60 * 1024) as usize;
        let mut v: Vec<u8> = Vec::with_capacity(len + 1);
        for _ in 0..len {
            v.push(rng.nonzero_byte());
        }
        v.push(0);
        both_process(&p, "C13", &v);
    }
}

#[test]
fn c14_process_string_interior_nul() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC014);
    for _ in 0..5_000 {
        let len = 2 + rng.below(200) as usize;
        let mut v: Vec<u8> = Vec::with_capacity(len + 1);
        for _ in 0..len {
            v.push(rng.nonzero_byte());
        }
        let cut = rng.below(len as u64) as usize;
        v[cut] = 0;
        v.push(0);
        both_process(&p, "C14", &v);
    }
}

// ===========================================================================
// helpers for the state-based rows
// ===========================================================================

use std::collections::HashSet;

/// Generates node ids and parent ids such that the child relation
/// (`parent_id -> id`) is guaranteed ACYCLIC: an id is never reused, and a
/// parent is either `-1`, the id of an *earlier* node, or a value that is
/// permanently forbidden from ever becoming an id. Cycles would make the C
/// `calculate_subtree_sum` recurse forever (see ERRORS.md E39).
struct IdPool {
    ids: Vec<i32>,
    used: HashSet<i32>,
    forbidden: HashSet<i32>,
    lo: i32,
    hi: i32,
}

impl IdPool {
    fn new(lo: i32, hi: i32) -> IdPool {
        let mut forbidden = HashSet::new();
        forbidden.insert(-1);
        IdPool { ids: Vec::new(), used: HashSet::new(), forbidden, lo, hi }
    }
    fn fresh_value(&self, rng: &mut Rng) -> i32 {
        loop {
            let v = rng.range_i32(self.lo, self.hi);
            if !self.used.contains(&v) && !self.forbidden.contains(&v) {
                return v;
            }
        }
    }
    fn new_id(&mut self, rng: &mut Rng) -> i32 {
        let v = self.fresh_value(rng);
        self.used.insert(v);
        self.ids.push(v);
        v
    }
    fn absent(&mut self, rng: &mut Rng) -> i32 {
        let v = self.fresh_value(rng);
        self.forbidden.insert(v);
        v
    }
    /// A legal parent for the node about to be inserted: `-1`, a never-an-id
    /// value, or the id of an ALREADY inserted node (never the node's own id,
    /// which would be a self-loop => infinite recursion in the C).
    fn parent(&mut self, rng: &mut Rng) -> i32 {
        match rng.below(4) {
            0 => -1,
            1 => self.absent(rng),
            _ if self.ids.is_empty() => -1,
            _ => self.ids[rng.below(self.ids.len() as u64) as usize],
        }
    }

    /// `(id, parent_id)` for the next node -- parent chosen BEFORE the id is
    /// registered, so it can never be the new node's own id.
    fn next_node(&mut self, rng: &mut Rng) -> (i32, i32) {
        let parent = self.parent(rng);
        let id = self.new_id(rng);
        (id, parent)
    }
}

/// A few "interesting" doubles for `Node.value` -- chosen so that floating
/// point summation ORDER inside `calculate_subtree_sum` is observable.
fn interesting_value(rng: &mut Rng) -> f64 {
    match rng.below(12) {
        0 => 0.0,
        1 => -0.0,
        2 => 1e308,
        3 => -1e308,
        4 => f64::INFINITY,
        5 => f64::NEG_INFINITY,
        6 => f64::NAN,
        7 => 1e-300,
        8 => f64::from_bits(1),
        9 => rng.next_f64_bits(),
        10 => (rng.next_i32() as f64) / 7.0,
        _ => rng.unit() * 1e6 - 5e5,
    }
}

fn random_name(rng: &mut Rng, maxlen: usize) -> Vec<u8> {
    let len = rng.below(maxlen as u64 + 1) as usize;
    let mut v = Vec::with_capacity(len);
    for _ in 0..len {
        v.push(rng.nonzero_byte());
    }
    v
}

// ===========================================================================
// C15..C30 -- add_node / find_node_by_id / get_children_count /
//             calculate_subtree_sum over every state & input shape
// ===========================================================================

#[test]
fn c15_pristine_store_queries() {
    let mut rng = Rng::new(0xC015);
    let p = Pair::fresh();
    for id in [0, 1, -1, 2, -2, 6, 100, INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1] {
        both_query(&p, "C15", id);
    }
    for _ in 0..5_000 {
        both_query(&p, "C15", rng.next_i32());
    }
}

#[test]
fn c16_single_root_node() {
    let mut rng = Rng::new(0xC016);
    for _ in 0..200 {
        let p = Pair::fresh();
        let mut pool = IdPool::new(-1_000_000, 1_000_000);
        let id = pool.new_id(&mut rng);
        let name = random_name(&mut rng, 60);
        let value = interesting_value(&mut rng);
        let idx = both_add(&p, "C16", id, -1, &name, value);
        assert_eq!(idx, 0, "first insert must land in slot 0");
        both_query(&p, "C16", id);
        both_query(&p, "C16", -1);
        both_query(&p, "C16", 0);
        both_query(&p, "C16", rng.next_i32());
        both_query(&p, "C16", id.wrapping_add(1));
    }
}

#[test]
fn c17_sequential_inserts_return_indices() {
    let mut rng = Rng::new(0xC017);
    for _ in 0..200 {
        let p = Pair::fresh();
        let mut pool = IdPool::new(-30_000, 30_000);
        let n = 1 + rng.below(20) as usize;
        let mut inserted = Vec::new();
        for k in 0..n {
            let (id, parent) = pool.next_node(&mut rng);
            let name = random_name(&mut rng, 60);
            let value = interesting_value(&mut rng);
            let idx = both_add(&p, "C17", id, parent, &name, value);
            assert_eq!(idx, k as i32, "add_node must return the new slot index");
            inserted.push((id, parent));
        }
        for (id, parent) in &inserted {
            both_query(&p, "C17", *id);
            both_query(&p, "C17", *parent);
        }
        both_delta(&p, "C17", inserted[0].0, inserted[n - 1].0);
    }
}

#[test]
fn c18_name_shapes() {
    let mut rng = Rng::new(0xC018);
    let lens = [0usize, 1, 2, 3, 10, 47, 48, 49, 50, 51, 52, 60, 120, 200];
    for &len in &lens {
        for trial in 0..40 {
            let p = Pair::fresh();
            let mut name = Vec::with_capacity(len);
            for i in 0..len {
                // trial 0: ASCII 'A'.., trial 1: all 0xff, else random non-zero
                name.push(match trial {
                    0 => b'A' + (i % 26) as u8,
                    1 => 0xff,
                    2 => 0x80,
                    _ => rng.nonzero_byte(),
                });
            }
            let id = 1 + rng.range_i32(1, 1000);
            both_add(&p, "C18", id, -1, &name, 1.5);
            both_query(&p, "C18", id);
            // also feed the stored (possibly truncated) name back through
            // process_string via the pointer the library handed us
            let (fc, fr) = both_find(&p, "C18", id);
            let sc = unsafe { (p.c.process_string)((*fc).name.as_mut_ptr()) };
            let sr = unsafe { (p.rust.process_string)((*fr).name.as_mut_ptr()) };
            eq_i32("C18", format!("process_string(stored name, len={len})"), sc, sr);
        }
    }
}

#[test]
fn c19_value_shapes_and_sum_propagation() {
    let mut rng = Rng::new(0xC019);
    let specials = [
        0.0f64,
        -0.0,
        1.0,
        -1.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        1e308,
        -1e308,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(1),
        1e-300,
        2147483647.5,
        -2147483648.5,
        4e9,
        -4e9,
    ];
    // every ordered pair of specials as (root value, child value)
    for &rv in &specials {
        for &cv in &specials {
            let p = Pair::fresh();
            both_add(&p, "C19", 1, -1, b"r", rv);
            both_add(&p, "C19", 2, 1, b"c", cv);
            both_query(&p, "C19", 1);
            both_query(&p, "C19", 2);
        }
    }
    // random multi-child fan-out: summation order is observable
    for _ in 0..300 {
        let p = Pair::fresh();
        both_add(&p, "C19", 1, -1, b"r", interesting_value(&mut rng));
        let k = 1 + rng.below(20) as i32;
        for c in 0..k {
            both_add(&p, "C19", 2 + c, 1, b"c", interesting_value(&mut rng));
        }
        both_query(&p, "C19", 1);
        for c in 0..k {
            both_query(&p, "C19", 2 + c);
        }
    }
}

#[test]
fn c20_id_shapes_and_duplicates() {
    let mut rng = Rng::new(0xC020);
    // extreme ids
    {
        let p = Pair::fresh();
        for (k, id) in [0, INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1, -2, 2]
            .iter()
            .enumerate()
        {
            both_add(&p, "C20", *id, -1, format!("n{k}").as_bytes(), k as f64);
        }
        for id in [0, INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1, -2, 2, -1, 1, 3] {
            both_query(&p, "C20", id);
        }
    }
    // duplicate ids: find_node_by_id must return the FIRST active match
    for _ in 0..300 {
        let p = Pair::fresh();
        let anchor = 777;
        both_add(&p, "C20", anchor, -1, b"anchor", 1.0);
        // never -1: a node whose id equals its own parent_id would make the C
        // `calculate_subtree_sum` recurse forever (ERRORS.md E39)
        let dup = rng.range_i32(1, 50);
        let copies = 2 + rng.below(5) as usize;
        for k in 0..copies {
            both_add(&p, "C20", dup, -1, format!("dup{k}").as_bytes(), k as f64);
        }
        both_query(&p, "C20", dup);
        both_delta(&p, "C20", anchor, dup);
        // deactivate the first copy -> the next one must win
        for _ in 0..copies {
            both_delta(&p, "C20", anchor, dup);
            both_query_nosum(&p, "C20", dup);
            if !both_mutate(&p, "C20", dup, &|n| unsafe { (*n).active = 0 }) {
                break;
            }
        }
        both_query_nosum(&p, "C20", dup);
    }
}

#[test]
fn c21_flat_tree_fan_out() {
    let mut rng = Rng::new(0xC021);
    for k in [0usize, 1, 2, 7, 50, 98] {
        for _ in 0..10 {
            let p = Pair::fresh();
            both_add(&p, "C21", 1, -1, b"root", interesting_value(&mut rng));
            for c in 0..k {
                both_add(
                    &p,
                    "C21",
                    2 + c as i32,
                    1,
                    format!("c{c}").as_bytes(),
                    interesting_value(&mut rng),
                );
            }
            both_query(&p, "C21", 1);
            both_query(&p, "C21", -1);
            for c in 0..=k + 2 {
                both_query(&p, "C21", 2 + c as i32);
            }
            both_query(&p, "C21", 0);
            both_query(&p, "C21", rng.next_i32());
        }
    }
}

#[test]
fn c22_deep_chain() {
    let mut rng = Rng::new(0xC022);
    for depth in [1usize, 2, 3, 10, 50, 99, 100] {
        let p = Pair::fresh();
        for k in 0..depth {
            let id = k as i32 + 1;
            let parent = if k == 0 { -1 } else { k as i32 };
            both_add(
                &p,
                "C22",
                id,
                parent,
                format!("d{k}").as_bytes(),
                interesting_value(&mut rng),
            );
        }
        for k in 0..=depth + 1 {
            both_query(&p, "C22", k as i32);
        }
        both_query(&p, "C22", -1);
    }
}

#[test]
fn c23_forest_multiple_roots() {
    let mut rng = Rng::new(0xC023);
    for m in [2usize, 5, 30] {
        for _ in 0..10 {
            let p = Pair::fresh();
            let mut next = 1i32;
            let mut roots = Vec::new();
            for _ in 0..m {
                let r = next;
                next += 1;
                roots.push(r);
                both_add(&p, "C23", r, -1, b"root", interesting_value(&mut rng));
                let kids = rng.below(3) as i32;
                for _ in 0..kids {
                    if next > 99 {
                        break;
                    }
                    let c = next;
                    next += 1;
                    both_add(&p, "C23", c, r, b"kid", interesting_value(&mut rng));
                }
            }
            both_query(&p, "C23", -1);
            for id in 0..=next + 1 {
                both_query(&p, "C23", id);
            }
        }
    }
}

#[test]
fn c24_random_forest() {
    let mut rng = Rng::new(0xC024);
    for _ in 0..200 {
        let p = Pair::fresh();
        let mut pool = IdPool::new(-200, 200);
        let n = rng.below(101) as usize;
        let mut parents = Vec::new();
        for _ in 0..n {
            let (id, parent) = pool.next_node(&mut rng);
            parents.push(parent);
            both_add(
                &p,
                "C24",
                id,
                parent,
                &random_name(&mut rng, 55),
                interesting_value(&mut rng),
            );
        }
        let ids: Vec<i32> = pool.ids.clone();
        for id in &ids {
            both_query(&p, "C24", *id);
        }
        for parent in &parents {
            both_query(&p, "C24", *parent);
        }
        for _ in 0..30 {
            both_query(&p, "C24", rng.range_i32(-250, 250));
            both_query(&p, "C24", rng.next_i32());
        }
    }
}

#[test]
fn c25_near_full_store() {
    let mut rng = Rng::new(0xC025);
    for _ in 0..20 {
        let p = Pair::fresh();
        for k in 0..99 {
            let idx = both_add(
                &p,
                "C25",
                k + 1,
                if k == 0 { -1 } else { k },
                format!("n{k}").as_bytes(),
                interesting_value(&mut rng),
            );
            assert_eq!(idx, k);
        }
        // slot 99 -- the last legal one
        let idx = both_add(&p, "C25", 100, 99, b"last", interesting_value(&mut rng));
        assert_eq!(idx, 99);
        both_query(&p, "C25", 100);
        both_query(&p, "C25", 99);
        both_query(&p, "C25", 1);
    }
}

#[test]
fn c26_full_store_queries() {
    let mut rng = Rng::new(0xC026);
    for _ in 0..20 {
        let p = Pair::fresh();
        let mut pool = IdPool::new(-500, 500);
        let mut ids = Vec::new();
        for _ in 0..MAX_NODES {
            let (id, parent) = pool.next_node(&mut rng);
            ids.push(id);
            both_add(
                &p,
                "C26",
                id,
                parent,
                &random_name(&mut rng, 55),
                interesting_value(&mut rng),
            );
        }
        for id in &ids {
            both_query(&p, "C26", *id);
        }
        both_query(&p, "C26", -1);
        for _ in 0..50 {
            both_query(&p, "C26", rng.range_i32(-600, 600));
        }
        both_delta(&p, "C26", ids[0], ids[MAX_NODES - 1]);
    }
}

#[test]
fn c27_deactivate_through_returned_pointer() {
    let mut rng = Rng::new(0xC027);
    for _ in 0..100 {
        let p = Pair::fresh();
        let mut ids = Vec::new();
        let n = 1 + rng.below(30) as i32;
        for k in 0..n {
            let id = k + 1;
            ids.push(id);
            both_add(
                &p,
                "C27",
                id,
                if k == 0 { -1 } else { 1 },
                format!("n{k}").as_bytes(),
                interesting_value(&mut rng),
            );
        }
        // deactivate a random subset, re-querying everything after each change
        let mut order: Vec<i32> = ids.clone();
        for i in (1..order.len()).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            order.swap(i, j);
        }
        for id in order {
            both_mutate(&p, "C27", id, &|n| unsafe { (*n).active = 0 });
            for q in &ids {
                both_query(&p, "C27", *q);
            }
            both_query(&p, "C27", -1);
        }
        // everything inactive now
        for q in &ids {
            both_query(&p, "C27", *q);
        }
    }
}

#[test]
fn c28_in_place_mutation_through_pointer() {
    let mut rng = Rng::new(0xC028);
    for _ in 0..200 {
        let p = Pair::fresh();
        let n = 1 + rng.below(12) as i32;
        for k in 0..n {
            both_add(
                &p,
                "C28",
                k + 1,
                if k == 0 { -1 } else { 1 },
                format!("n{k}").as_bytes(),
                interesting_value(&mut rng),
            );
        }
        // rewrite `value` (safe for the recursion: shape untouched)
        for k in 0..n {
            let v = interesting_value(&mut rng);
            both_mutate(&p, "C28", k + 1, &|node| unsafe { (*node).value = v });
        }
        for k in 0..n {
            both_query(&p, "C28", k + 1);
        }
        // rewrite `name` in place, then read it back through process_string
        for k in 0..n {
            let bytes = random_name(&mut rng, 49);
            both_mutate(&p, "C28", k + 1, &|node| unsafe {
                for i in 0..MAX_NAME_LEN {
                    (*node).name[i] = if i < bytes.len() { bytes[i] as i8 } else { 0 };
                }
            });
        }
        for k in 0..n {
            let (fc, fr) = both_find(&p, "C28", k + 1);
            let sc = unsafe { (p.c.process_string)((*fc).name.as_mut_ptr()) };
            let sr = unsafe { (p.rust.process_string)((*fr).name.as_mut_ptr()) };
            eq_i32("C28", "process_string(mutated name)", sc, sr);
        }
        // re-point every node at a parent that cannot form a cycle
        // (a value that is not any node's id)
        let orphan = 10_000 + rng.range_i32(0, 1000);
        for k in 0..n {
            both_mutate(&p, "C28", k + 1, &|node| unsafe {
                (*node).parent_id = orphan
            });
        }
        for k in 0..=n {
            both_query(&p, "C28", k + 1);
        }
        both_query(&p, "C28", orphan);
        both_query(&p, "C28", -1);
        // rewrite ids (all distinct, parents still all `orphan` => acyclic)
        for k in 0..n {
            let newid = 5_000 + k;
            both_mutate(&p, "C28", k + 1, &move |node| unsafe { (*node).id = newid });
        }
        for k in 0..n {
            both_query(&p, "C28", 5_000 + k);
            both_query(&p, "C28", k + 1);
        }
        both_query(&p, "C28", orphan);
    }
}

#[test]
fn c29_active_truthiness_non_boolean_values() {
    let mut rng = Rng::new(0xC029);
    let actives = [0i32, 1, 2, -1, -2, INT_MIN, INT_MAX, 0x8000_0000u32 as i32, 256, 0x100];
    for &a in &actives {
        for _ in 0..20 {
            let p = Pair::fresh();
            let n = 1 + rng.below(6) as i32;
            for k in 0..n {
                both_add(
                    &p,
                    "C29",
                    k + 1,
                    if k == 0 { -1 } else { 1 },
                    b"x",
                    interesting_value(&mut rng),
                );
            }
            for k in 0..n {
                both_mutate(&p, "C29", k + 1, &|node| unsafe { (*node).active = a });
            }
            for k in 0..=n {
                both_query(&p, "C29", k + 1);
            }
            both_query(&p, "C29", -1);
            both_query(&p, "C29", 1);
        }
    }
}

#[test]
fn c30_forward_references() {
    let mut rng = Rng::new(0xC030);
    for _ in 0..100 {
        let p = Pair::fresh();
        // insert a chain in REVERSE order: node k's parent is node k+1, which
        // is inserted later. The parent graph stays acyclic.
        let depth = 2 + rng.below(20) as i32;
        for k in 0..depth {
            let id = k + 1;
            let parent = if k + 1 == depth { -1 } else { k + 2 };
            both_add(
                &p,
                "C30",
                id,
                parent,
                format!("f{k}").as_bytes(),
                interesting_value(&mut rng),
            );
        }
        for k in 0..=depth {
            both_query(&p, "C30", k + 1);
        }
        both_query(&p, "C30", -1);
        both_delta(&p, "C30", 1, depth);
    }
}

// ===========================================================================
// C31..C38 -- maxnmin (the one-shot wrapper from include/lib.h)
// ===========================================================================

#[test]
fn c31_maxnmin_residue_cross_product() {
    let p = Pair::fresh();
    for a in 0..36 {
        for b in 0..36 {
            both_maxnmin(&p, "C31", a, b, 1, 0);
        }
    }
}

#[test]
fn c32_maxnmin_negative_params() {
    let p = Pair::fresh();
    for a in -26..=0 {
        for b in -26..=0 {
            both_maxnmin(&p, "C32", a, b, 1, 0);
            both_maxnmin(&p, "C32", a, b, 3, -4);
        }
    }
}

#[test]
fn c33_maxnmin_param3_shapes() {
    let p = Pair::fresh();
    let p3s = [
        0,
        1,
        -1,
        2,
        -2,
        5,
        -7,
        INT_MAX,
        INT_MIN,
        INT_MAX - 1,
        INT_MIN + 1,
        1_000_000_000,
        -1_000_000_000,
        2_000_000_000,
        -2_000_000_000,
    ];
    for &p3 in &p3s {
        for a in -7..=7 {
            for b in -7..=7 {
                for d in [-3, -1, 0, 1, 3, INT_MAX, INT_MIN] {
                    both_maxnmin(&p, "C33", a, b, p3, d);
                }
            }
        }
    }
}

#[test]
fn c34_maxnmin_param4_residues() {
    let p = Pair::fresh();
    for d in -15..=15 {
        for a in [0, 1, 5, 6, -1, -6, 7] {
            for b in [0, 1, 5, 6, -1, -6, 7] {
                both_maxnmin(&p, "C34", a, b, 2, d);
                both_maxnmin(&p, "C34", a, b, -1, d);
            }
        }
    }
}

#[test]
fn c35_maxnmin_extremes_cross_product() {
    let p = Pair::fresh();
    let vals = [INT_MIN, INT_MIN + 1, -7, -1, 0, 1, 7, INT_MAX - 1, INT_MAX];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    both_maxnmin(&p, "C35", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn c36_maxnmin_random_sweep() {
    let p = Pair::fresh();
    let mut rng = Rng::new(0xC036);
    for _ in 0..20_000 {
        both_maxnmin(&p, "C36", rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    // small magnitudes, where the residue selection changes fastest
    for _ in 0..20_000 {
        both_maxnmin(
            &p,
            "C36",
            rng.range_i32(-40, 40),
            rng.range_i32(-40, 40),
            rng.range_i32(-40, 40),
            rng.range_i32(-40, 40),
        );
    }
}

#[test]
fn c37_maxnmin_interleaved_with_add_node() {
    let mut rng = Rng::new(0xC037);
    for _ in 0..100 {
        let p = Pair::fresh();
        for round in 0..6 {
            let r = both_maxnmin(&p, "C37", rng.next_i32(), rng.next_i32(), rng.range_i32(-3, 3), rng.next_i32());
            let _ = r;
            // after maxnmin the store always holds exactly the 6 builtins
            for (id, parent, name, value) in BUILTINS {
                let (fc, _) = both_find(&p, "C37", id);
                assert!(!fc.is_null(), "builtin {id} missing after maxnmin");
                let s = unsafe { NodeSnap::read(fc) };
                assert_eq!(s.parent_id, parent);
                assert_eq!(s.value_bits, value.to_bits());
                assert_eq!(&s.name[..name.len()], name.as_bytes());
                assert_eq!(s.name[name.len()], 0);
            }
            // append fresh nodes; ids stay clear of the builtins (1..=6)
            let extra = rng.below(8) as i32;
            for k in 0..extra {
                let id = 1000 + round * 100 + k;
                let parent = match rng.below(3) {
                    0 => -1,
                    1 => rng.range_i32(1, 6),
                    _ if k == 0 => -1,
                    _ => 1000 + round * 100 + rng.range_i32(0, k - 1),
                };
                let idx = both_add(&p, "C37", id, parent, format!("x{id}").as_bytes(), interesting_value(&mut rng));
                assert_eq!(idx, 6 + k);
            }
            for id in [-1, 0, 1, 2, 3, 4, 5, 6, 7, 1000, 1000 + round * 100] {
                both_query(&p, "C37", id);
            }
        }
    }
}

#[test]
fn c38_maxnmin_resets_full_and_mutated_store() {
    let mut rng = Rng::new(0xC038);
    for _ in 0..30 {
        let p = Pair::fresh();
        // fill the store completely with an acyclic shape
        for k in 0..MAX_NODES as i32 {
            both_add(
                &p,
                "C38",
                1000 + k,
                if k == 0 { -1 } else { 1000 + k - 1 },
                format!("f{k}").as_bytes(),
                interesting_value(&mut rng),
            );
        }
        // one past the limit
        eq_i32(
            "C38",
            "add_node on full store",
            unsafe { (p.c.add_node)(9999, -1, CBuf::from_str("over").ptr(), 1.0) },
            unsafe { (p.rust.add_node)(9999, -1, CBuf::from_str("over").ptr(), 1.0) },
        );
        // mutate a few nodes through the returned pointers
        for _ in 0..10 {
            let id = 1000 + rng.range_i32(0, MAX_NODES as i32 - 1);
            let v = interesting_value(&mut rng);
            let a = rng.range_i32(-3, 3);
            both_mutate(&p, "C38", id, &|n| unsafe {
                (*n).value = v;
                (*n).active = a;
            });
        }
        both_query(&p, "C38", 1000);
        // maxnmin must silently reset node_count to 0 and re-seed the builtins
        both_maxnmin(&p, "C38", rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        for id in [-1, 0, 1, 2, 3, 4, 5, 6, 7, 1000, 1050, 1099, 9999] {
            both_query(&p, "C38", id);
        }
        // and the store must be appendable again from index 6
        let idx = both_add(&p, "C38", 4242, -1, b"again", 3.25);
        assert_eq!(idx, 6);
        both_query(&p, "C38", 4242);
    }
}

// ===========================================================================
// C39 -- randomized operation sequences over ALL 7 entry points
// ===========================================================================

#[test]
fn c39_operation_sequence_fuzz() {
    let mut rng = Rng::new(0xC039);
    for run in 0..60 {
        let p = Pair::fresh();
        // ids that actually exist, in insertion order; parents are only ever
        // -1, a builtin id, or an EARLIER existing id => the child relation is
        // always a forest (no infinite recursion, see ERRORS.md E39)
        let mut existing: Vec<i32> = Vec::new();
        let mut count: usize = 0;
        let mut next_id: i32 = 20_000 + run * 1000;
        let mut builtins_present = false;
        for _ in 0..200 {
            match rng.below(10) {
                0 | 1 | 2 => {
                    // add_node
                    let id = next_id;
                    next_id += 1;
                    let parent = match rng.below(4) {
                        0 => -1,
                        1 if builtins_present => rng.range_i32(1, 6),
                        2 if !existing.is_empty() => {
                            existing[rng.below(existing.len() as u64) as usize]
                        }
                        _ => 777_777, // dangling: never an id
                    };
                    let idx = both_add(
                        &p,
                        "C39",
                        id,
                        parent,
                        &random_name(&mut rng, 60),
                        interesting_value(&mut rng),
                    );
                    if count < MAX_NODES {
                        assert_eq!(idx, count as i32);
                        count += 1;
                        existing.push(id);
                    } else {
                        assert_eq!(idx, -1);
                    }
                }
                3 => {
                    let id = if !existing.is_empty() && rng.bool() {
                        existing[rng.below(existing.len() as u64) as usize]
                    } else {
                        rng.next_i32()
                    };
                    both_find(&p, "C39", id);
                }
                4 => {
                    let id = if !existing.is_empty() && rng.bool() {
                        existing[rng.below(existing.len() as u64) as usize]
                    } else {
                        rng.next_i32()
                    };
                    both_children(&p, "C39", id);
                }
                5 => {
                    let id = if !existing.is_empty() && rng.bool() {
                        existing[rng.below(existing.len() as u64) as usize]
                    } else {
                        rng.range_i32(-20, 20)
                    };
                    both_subtree(&p, "C39", id);
                }
                6 => {
                    // mutate value / active / name (never id or parent_id, so
                    // the forest shape -- and termination -- is preserved)
                    if !existing.is_empty() {
                        let id = existing[rng.below(existing.len() as u64) as usize];
                        let v = interesting_value(&mut rng);
                        let a = rng.range_i32(-2, 2);
                        let bytes = random_name(&mut rng, 49);
                        both_mutate(&p, "C39", id, &|n| unsafe {
                            (*n).value = v;
                            (*n).active = a;
                            for i in 0..MAX_NAME_LEN {
                                (*n).name[i] = if i < bytes.len() { bytes[i] as i8 } else { 0 };
                            }
                        });
                    }
                }
                7 => {
                    both_process(&p, "C39", &{
                        let mut v = random_name(&mut rng, 80);
                        v.push(0);
                        v
                    });
                }
                8 => {
                    both_d2i(&p, "C39", interesting_value(&mut rng));
                }
                _ => {
                    both_maxnmin(
                        &p,
                        "C39",
                        rng.next_i32(),
                        rng.next_i32(),
                        rng.range_i32(-4, 4),
                        rng.next_i32(),
                    );
                    // maxnmin wipes the store back to the 6 builtins
                    existing = vec![1, 2, 3, 4, 5, 6];
                    count = 6;
                    builtins_present = true;
                }
            }
        }
    }
}

// ===========================================================================
// C40 -- struct layout / raw byte image of the stored nodes
// ===========================================================================

#[test]
fn c40_raw_node_image() {
    let mut rng = Rng::new(0xC040);
    for _ in 0..100 {
        let p = Pair::fresh();
        let n = 1 + rng.below(10) as i32;
        for k in 0..n {
            both_add(
                &p,
                "C40",
                k + 1,
                if k == 0 { -1 } else { 1 },
                &random_name(&mut rng, 60),
                interesting_value(&mut rng),
            );
        }
        for k in 0..n {
            let (fc, fr) = both_find(&p, "C40", k + 1);
            // the API-visible fields, byte for byte, at the C offsets
            let cb = unsafe { std::slice::from_raw_parts(fc as *const u8, NODE_SIZE) };
            let rb = unsafe { std::slice::from_raw_parts(fr as *const u8, NODE_SIZE) };
            assert_eq!(&cb[0..4], &rb[0..4], "[C40] id bytes differ");
            assert_eq!(&cb[4..8], &rb[4..8], "[C40] parent_id bytes differ");
            assert_eq!(&cb[8..58], &rb[8..58], "[C40] name bytes differ");
            assert_eq!(&cb[64..72], &rb[64..72], "[C40] value bytes differ");
            assert_eq!(&cb[72..76], &rb[72..76], "[C40] active bytes differ");
            // the inter-field padding (58..64) and tail padding (76..80) are
            // not part of the observable API, but they match too
            assert_eq!(cb, rb, "[C40] whole 80-byte Node image differs");
        }
        // consecutive slots must be exactly 80 bytes apart in both libraries
        if n >= 2 {
            both_delta(&p, "C40", 1, 2);
        }
    }
}

// ===========================================================================
// C41 -- NaN payload propagation through nested accumulation
//        (regression row: the C `sum += child` keeps the CHILD's NaN)
// ===========================================================================

#[test]
fn c41_nan_payload_propagation() {
    let mut rng = Rng::new(0xC041);
    let nans = [
        0x7ff8_0000_0000_0000u64, // +qNaN, empty payload
        0xfff8_0000_0000_0000,    // -qNaN, empty payload (x86 "default" NaN)
        0x7ff8_0000_0000_0001,
        0xfff8_dead_beef_0001,
        0x7ff0_0000_0000_0001, // +sNaN
        0xfff0_0000_0000_0002, // -sNaN
        0x7ff4_1234_5678_9abc,
        0xfffc_0000_0000_0000,
    ];
    let others = [
        0x7ff0_0000_0000_0000u64, // +inf
        0xfff0_0000_0000_0000,    // -inf
        0x0000_0000_0000_0000,    // +0
        0x8000_0000_0000_0000,    // -0
        0x3ff0_0000_0000_0000,    // 1.0
    ];
    // chains: every ordered pair of NaNs at depth 3, plus inf/-inf mixes
    for &a in &nans {
        for &b in &nans {
            for &c in others.iter().chain(nans.iter()) {
                let p = Pair::fresh();
                both_add(&p, "C41", 1, -1, b"r", f64::from_bits(a));
                both_add(&p, "C41", 2, 1, b"m", f64::from_bits(b));
                both_add(&p, "C41", 3, 2, b"l", f64::from_bits(c));
                both_query(&p, "C41", 1);
                both_query(&p, "C41", 2);
                both_query(&p, "C41", 3);
            }
        }
    }
    // fan-out: root with several NaN/inf children (accumulation order matters)
    for _ in 0..400 {
        let p = Pair::fresh();
        let pick = |r: &mut Rng| {
            let all: Vec<u64> = nans.iter().chain(others.iter()).cloned().collect();
            f64::from_bits(all[r.below(all.len() as u64) as usize])
        };
        both_add(&p, "C41", 1, -1, b"r", pick(&mut rng));
        let k = 1 + rng.below(8) as i32;
        for c in 0..k {
            both_add(&p, "C41", 2 + c, 1, b"c", pick(&mut rng));
            // grandchildren, to nest the accumulation
            if rng.bool() {
                both_add(&p, "C41", 100 + c, 2 + c, b"g", pick(&mut rng));
            }
        }
        for id in 0..=110 {
            both_query(&p, "C41", id);
        }
    }
}
