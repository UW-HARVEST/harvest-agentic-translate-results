//! Extra Phase B/C edge cases that the per-row tests do not reach:
//!
//!  * `size_t` byte-count multiplications that WRAP to a small NON-ZERO value
//!    (so the allocation succeeds and the library keeps a bogus huge capacity),
//!  * the unprototyped (`int f()`) declaration of `calculate_matrix_checksum`,
//!  * a large repeated-doubling chain,
//!  * a whole-API randomized fuzz loop comparing every observable at every step.

mod common;

use common::{load, make_array, SEED};
use std::ffi::c_int;
use std::ptr;

// ---------------------------------------------------------------------------
// expand_array: new_capacity * sizeof(int) wraps to a small NON-ZERO size, so
// realloc SUCCEEDS and the C stores the absurd capacity without complaint.
// ---------------------------------------------------------------------------
#[test]
fn x1_expand_array_bytes_wrap_to_small_nonzero() {
    let p = load();
    // cap = 2^61 + 1 -> new = 2^62 + 2 -> bytes = 2^64 + 8 == 8   (mod 2^64)
    // cap = 2^62 + 1 -> new = 2^63 + 2 -> bytes = 2^65 + 8 == 8   (mod 2^64)
    // cap = 2^61 + 2 -> new = 2^62 + 4 -> bytes = 2^64 + 16 == 16 (mod 2^64)
    let caps: [usize; 5] = [
        (1usize << 61) + 1,
        (1usize << 62) + 1,
        (1usize << 61) + 2,
        (1usize << 62) + 2,
        (1usize << 63) + 1,
    ];
    for cap in caps {
        unsafe {
            let a = make_array(4, 0, &[1, 2, 3, 4]);
            let b = make_array(4, 0, &[1, 2, 3, 4]);
            (*a).capacity = cap;
            (*b).capacity = cap;

            let rc = p.c.expand_array(a);
            let rr = p.rs.expand_array(b);
            assert_eq!(
                rc, rr,
                "expand_array(cap={cap:#x}) rc diverged (wrapped bytes = {:#x})",
                cap.wrapping_mul(2).wrapping_mul(4)
            );
            let ha = ptr::read(a);
            let hb = ptr::read(b);
            assert_eq!(
                ha.capacity, hb.capacity,
                "expand_array(cap={cap:#x}) capacity diverged"
            );
            assert_eq!(ha.size, hb.size);
            assert_eq!(ha.data.is_null(), hb.data.is_null());
            if rc != 0 {
                // succeeded: the C assigns the wrapped-around capacity verbatim
                assert_eq!(
                    ha.capacity,
                    cap.wrapping_mul(2),
                    "on success capacity must be exactly capacity*2 (wrapping)"
                );
            } else {
                assert_eq!(ha.capacity, cap, "on failure capacity is untouched");
            }
            // Leak deliberately: on the realloc(p, 0) sub-case `data` is dangling
            // in BOTH libraries, and a double free would abort the test process.
        }
    }
}

// ---------------------------------------------------------------------------
// init_array: capacity * sizeof(int) wraps to a small NON-ZERO size, then the
// array is actually used within the real (tiny) allocation.
// ---------------------------------------------------------------------------
#[test]
fn x2_init_array_bytes_wrap_then_use() {
    let p = load();
    // (2^62 + n) * 4 == 4n (mod 2^64): a real 4n-byte buffer with a huge capacity
    for n in 1usize..=4 {
        let cap = (1usize << 62) + n;
        unsafe {
            let a = p.c.init_array(cap);
            let b = p.rs.init_array(cap);
            assert_eq!(a.is_null(), b.is_null(), "init_array({cap:#x}) NULL-ness");
            assert!(!a.is_null(), "expected the wrapped {}-byte alloc to succeed", 4 * n);
            assert_eq!(ptr::read(a).capacity, ptr::read(b).capacity);
            assert_eq!(ptr::read(a).capacity, cap);
            assert_eq!(ptr::read(a).size, ptr::read(b).size);
            // size < capacity, so add_element never expands: it writes straight
            // into the tiny real buffer. Stay within the 4*n real bytes.
            let mut rng = common::Rng::new(SEED ^ 0x2222);
            let mut vals = Vec::new();
            for _ in 0..n {
                let v = rng.spicy_i32();
                vals.push(v);
                assert_eq!(p.c.add_element(a, v), p.rs.add_element(b, v));
            }
            let ha = ptr::read(a);
            let hb = ptr::read(b);
            assert_eq!(ha.size, hb.size);
            assert_eq!(ha.size, n);
            assert_eq!(ha.capacity, hb.capacity);
            assert_eq!(p.c.elements(a, n), p.rs.elements(b, n));
            assert_eq!(p.c.elements(a, n), vals);
            p.c.free_array(a);
            p.rs.free_array(b);
        }
    }
}

// ---------------------------------------------------------------------------
// `int calculate_matrix_checksum()` is declared with an EMPTY parameter list in
// C, so a K&R-style caller may legally pass arguments; they must be ignored.
// Exercise that exact ABI shape against both libraries.
// ---------------------------------------------------------------------------
#[test]
fn x3_checksum_unprototyped_extra_args() {
    let p = load();
    use libloading::{Library, Symbol};
    type F4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    unsafe {
        let lc = Library::new(&p.c.path).unwrap();
        let lr = Library::new(&p.rs.path).unwrap();
        let fc: Symbol<F4> = lc.get(b"calculate_matrix_checksum\0").unwrap();
        let fr: Symbol<F4> = lr.get(b"calculate_matrix_checksum\0").unwrap();
        let mut rng = common::Rng::new(SEED ^ 0x3333);
        for _ in 0..256 {
            let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
            assert_eq!(
                fc(a, b, c, d),
                fr(a, b, c, d),
                "calculate_matrix_checksum called with extra args diverged"
            );
            // and it still matches the zero-arg form
            assert_eq!(fc(a, b, c, d), p.c.calculate_matrix_checksum());
            assert_eq!(fr(a, b, c, d), p.rs.calculate_matrix_checksum());
        }
    }
}

// ---------------------------------------------------------------------------
// Long repeated-doubling chain: 1 -> 2 -> 4 -> ... with full buffer comparison.
// ---------------------------------------------------------------------------
#[test]
fn x4_long_growth_chain() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0x4444);
    for cap in [1usize, 2, 3, 5] {
        let n = 10_000usize;
        let vals: Vec<c_int> = (0..n).map(|_| rng.spicy_i32()).collect();
        unsafe {
            let a = p.c.init_array(cap);
            let b = p.rs.init_array(cap);
            for (i, &v) in vals.iter().enumerate() {
                let x = p.c.add_element(a, v);
                let y = p.rs.add_element(b, v);
                assert_eq!(x, y, "cap={cap} add #{i} rc");
                assert_eq!(x, 1, "cap={cap} add #{i} should succeed");
            }
            let ha = ptr::read(a);
            let hb = ptr::read(b);
            assert_eq!(ha.size, hb.size, "cap={cap} final size");
            assert_eq!(ha.capacity, hb.capacity, "cap={cap} final capacity");
            assert_eq!(ha.size, n);
            let ea = p.c.elements(a, n);
            let eb = p.rs.elements(b, n);
            assert_eq!(ea, eb, "cap={cap} final buffer");
            assert_eq!(ea, vals, "cap={cap} buffer contents wrong");
            p.c.free_array(a);
            p.rs.free_array(b);
        }
    }
}

// ---------------------------------------------------------------------------
// Partial writes to the exported `matrix`: the checksum must read all 12 slots.
// ---------------------------------------------------------------------------
#[test]
fn x5_matrix_partial_writes() {
    let p = load();
    for slot in 0..12usize {
        for delta in [1i32, -1, i32::MAX, i32::MIN, 0x1000, -0x1000] {
            let mut m = common::DEFAULT_MATRIX;
            m[slot] = delta;
            p.c.matrix_write(&m);
            p.rs.matrix_write(&m);
            assert_eq!(
                p.c.calculate_matrix_checksum(),
                p.rs.calculate_matrix_checksum(),
                "checksum diverged with matrix[{slot}] = {delta}"
            );
            assert_eq!(
                p.c.matrixsum(1, 0, -1, 7),
                p.rs.matrixsum(1, 0, -1, 7),
                "matrixsum diverged with matrix[{slot}] = {delta}"
            );
        }
    }
    p.c.matrix_reset();
    p.rs.matrix_reset();
}

// ---------------------------------------------------------------------------
// Whole-API randomized fuzz: every exported entry point, interleaved, with
// every observable compared after every single step.
// ---------------------------------------------------------------------------
#[test]
fn x6_whole_api_fuzz() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0x6666);

    // parallel pools of live arrays (index i of one corresponds to index i of
    // the other); `expands[i]` bounds the doubling chain so it stays allocatable
    let mut ca: Vec<*mut common::DynamicArray> = Vec::new();
    let mut ra: Vec<*mut common::DynamicArray> = Vec::new();

    for step in 0..20_000u32 {
        match rng.below(9) {
            // init_array with a sane capacity (>= 1: capacity 0 makes `data`
            // dangle in both libraries, which is covered separately in E6/E8)
            0 => unsafe {
                if ca.len() < 24 {
                    let cap = 1 + rng.below(8) as usize;
                    let a = p.c.init_array(cap);
                    let b = p.rs.init_array(cap);
                    assert_eq!(a.is_null(), b.is_null(), "step {step}: init NULL-ness");
                    if !a.is_null() {
                        assert_eq!(ptr::read(a).size, ptr::read(b).size);
                        assert_eq!(ptr::read(a).capacity, ptr::read(b).capacity);
                        ca.push(a);
                        ra.push(b);
                    }
                }
            },
            // add_element on a live array
            1 | 2 | 3 => unsafe {
                if !ca.is_empty() {
                    let i = rng.below(ca.len() as u64) as usize;
                    let v = rng.spicy_i32();
                    let x = p.c.add_element(ca[i], v);
                    let y = p.rs.add_element(ra[i], v);
                    assert_eq!(x, y, "step {step}: add rc");
                    let ha = ptr::read(ca[i]);
                    let hb = ptr::read(ra[i]);
                    assert_eq!(ha.size, hb.size, "step {step}: size");
                    assert_eq!(ha.capacity, hb.capacity, "step {step}: capacity");
                    assert_eq!(
                        p.c.elements(ca[i], ha.size),
                        p.rs.elements(ra[i], hb.size),
                        "step {step}: buffer"
                    );
                }
            },
            // expand_array on a live array (bounded so the chain stays small)
            4 => unsafe {
                if !ca.is_empty() {
                    let i = rng.below(ca.len() as u64) as usize;
                    if ptr::read(ca[i]).capacity < (1 << 18) {
                        let x = p.c.expand_array(ca[i]);
                        let y = p.rs.expand_array(ra[i]);
                        assert_eq!(x, y, "step {step}: expand rc");
                        assert_eq!(
                            ptr::read(ca[i]).capacity,
                            ptr::read(ra[i]).capacity,
                            "step {step}: capacity after expand"
                        );
                        assert_eq!(ptr::read(ca[i]).size, ptr::read(ra[i]).size);
                    }
                }
            },
            // free_array a live array (or NULL)
            5 => unsafe {
                if !ca.is_empty() && rng.below(4) != 0 {
                    let i = rng.below(ca.len() as u64) as usize;
                    p.c.free_array(ca.swap_remove(i));
                    p.rs.free_array(ra.swap_remove(i));
                } else {
                    p.c.free_array(ptr::null_mut());
                    p.rs.free_array(ptr::null_mut());
                }
            },
            // process_flags
            6 => {
                let f = rng.spicy_i32();
                assert_eq!(
                    p.c.process_flags(f),
                    p.rs.process_flags(f),
                    "step {step}: process_flags({f})"
                );
            }
            // mutate the global matrix, then checksum
            7 => {
                let mut m = [0i32; 12];
                for s in m.iter_mut() {
                    *s = rng.spicy_i32();
                }
                p.c.matrix_write(&m);
                p.rs.matrix_write(&m);
                assert_eq!(p.c.matrix_bytes(), p.rs.matrix_bytes(), "step {step}: matrix bytes");
                assert_eq!(
                    p.c.calculate_matrix_checksum(),
                    p.rs.calculate_matrix_checksum(),
                    "step {step}: checksum with matrix {m:?}"
                );
            }
            // matrixsum against whatever the current global matrix is
            _ => {
                let (a, b, c, d) =
                    (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
                assert_eq!(
                    p.c.matrixsum(a, b, c, d),
                    p.rs.matrixsum(a, b, c, d),
                    "step {step}: matrixsum({a},{b},{c},{d}) with matrix {:?}",
                    p.c.matrix_read()
                );
            }
        }
    }

    // drain
    unsafe {
        for a in ca {
            p.c.free_array(a);
        }
        for b in ra {
            p.rs.free_array(b);
        }
    }
    p.c.matrix_reset();
    p.rs.matrix_reset();
}
