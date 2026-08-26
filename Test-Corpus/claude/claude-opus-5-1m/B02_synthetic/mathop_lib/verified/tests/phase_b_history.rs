// Phase B — valid-path differential tests for the allocation and
// history-recording entry points. CONFIGS.md rows C13 .. C26.
//
// `perform_computation_with_history` is the lowest-level *composed* entry point
// (it fans out to `select_operation`, the op functions, `allocate_results` and
// `get_computation_timestamp`), so it is driven directly here rather than only
// through `mathop`.

mod common;

use common::*;
use std::ffi::c_int;

/// A caller-owned history buffer with guard slots on both sides so that
/// out-of-range writes land in memory we own and can compare.
const PAD: usize = 8;
const SLOTS: usize = 10;
const TOTAL: usize = PAD + SLOTS + PAD;

struct Buf {
    cells: Vec<ComputationResult>,
}

impl Buf {
    /// Filled with a recognisable pattern so that "untouched" is verifiable.
    fn new(tag: i32) -> Buf {
        let cells = (0..TOTAL)
            .map(|i| ComputationResult {
                value: tag * 1000 + i as i32,
                timestamp: -(i as i64) - 1,
                status: STATUS_WARNING,
            })
            .collect();
        Buf { cells }
    }
    /// Pointer to logical slot 0 (i.e. past the leading guard).
    fn base(&mut self) -> *mut ComputationResult {
        unsafe { self.cells.as_mut_ptr().add(PAD) }
    }
}

/// Two identically-initialised buffers, one per library.
fn pair(tag: i32) -> (Buf, Buf) {
    (Buf::new(tag), Buf::new(tag))
}

/// Compare two buffers ignoring the `timestamp` field of the slots written
/// during this call (the timestamp comes from `time()` and is compared
/// separately with a tolerance-free equality when the second didn't tick).
fn assert_bufs_eq(cb: &Buf, rb: &Buf, ctx: &str) {
    for i in 0..TOTAL {
        let cc = cb.cells[i];
        let rc = rb.cells[i];
        assert_eq!(
            cc.value, rc.value,
            "{ctx}: cell[{i}].value C={} Rust={}",
            cc.value, rc.value
        );
        assert_eq!(
            cc.status, rc.status,
            "{ctx}: cell[{i}].status C={} Rust={}",
            cc.status, rc.status
        );
        assert_eq!(
            cc.timestamp, rc.timestamp,
            "{ctx}: cell[{i}].timestamp C={} Rust={}",
            cc.timestamp, rc.timestamp
        );
    }
}

/// One differential call of `perform_computation_with_history` on caller-owned
/// buffers.
fn diff_perform(a: c_int, b: c_int, op: c_int, count: c_int, tag: i32, ctx: &str) {
    let (c, r) = both();
    let (mut cb, mut rb) = pair(tag);
    let mut ch: *mut ComputationResult = cb.base();
    let mut rh: *mut ComputationResult = rb.base();
    let mut cn: c_int = count;
    let mut rn: c_int = count;

    let cv = unsafe { (c.perform_computation_with_history)(a, b, op, &mut ch, &mut cn) };
    let rv = unsafe { (r.perform_computation_with_history)(a, b, op, &mut rh, &mut rn) };

    assert_eq!(cv, rv, "{ctx}: return value C={cv} Rust={rv}");
    assert_eq!(cn, rn, "{ctx}: history_count C={cn} Rust={rn}");
    assert_eq!(
        ch,
        cb.base(),
        "{ctx}: C must not replace a non-NULL history pointer"
    );
    assert_eq!(
        rh,
        rb.base(),
        "{ctx}: Rust must not replace a non-NULL history pointer"
    );
    assert_bufs_eq(&cb, &rb, ctx);
}

// ---------------------------------------------------------------------------
// C13 / C14 / C16 — allocate_results, valid counts.
// ---------------------------------------------------------------------------

#[test]
fn c13_allocate_results_valid_counts() {
    let (c, r) = both();
    for &count in &[1i32, 2, 10, 100, 1000] {
        let cp = unsafe { (c.allocate_results)(count) };
        let rp = unsafe { (r.allocate_results)(count) };
        assert!(!cp.is_null(), "C allocate_results({count}) returned NULL");
        assert!(!rp.is_null(), "Rust allocate_results({count}) returned NULL");
        // calloc must have zeroed exactly count * sizeof(ComputationResult).
        let n = count as usize;
        unsafe {
            for i in 0..n {
                let cc = *cp.add(i);
                let rc = *rp.add(i);
                assert_eq!(cc, ComputationResult::default(), "C block not zeroed at {i}");
                assert_eq!(
                    rc,
                    ComputationResult::default(),
                    "Rust block not zeroed at {i}"
                );
            }
            libc_free(cp);
            libc_free(rp);
        }
    }
}

#[test]
fn c14_allocate_results_zero_count() {
    let (c, r) = both();
    let cp = unsafe { (c.allocate_results)(0) };
    let rp = unsafe { (r.allocate_results)(0) };
    assert_eq!(
        cp.is_null(),
        rp.is_null(),
        "allocate_results(0): C null={} Rust null={}",
        cp.is_null(),
        rp.is_null()
    );
    unsafe {
        libc_free(cp);
        libc_free(rp);
    }
}

/// C15 — negative counts: `(size_t)count` sign-extends, so `calloc` must fail
/// and both libraries must hand back NULL (no NULL check exists in the C).
#[test]
fn c15_allocate_results_negative_counts() {
    let (c, r) = both();
    let mut rng = Rng::new(0x15_0015);
    let mut counts: Vec<i32> = vec![-1, -2, -10, -1000, i32::MIN, i32::MIN + 1];
    for _ in 0..64 {
        counts.push(rng.range(i32::MIN, -1));
    }
    for &count in &counts {
        let cp = unsafe { (c.allocate_results)(count) };
        let rp = unsafe { (r.allocate_results)(count) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "allocate_results({count}): C null={} Rust null={}",
            cp.is_null(),
            rp.is_null()
        );
        assert!(cp.is_null(), "allocate_results({count}) must fail");
        unsafe {
            libc_free(cp);
            libc_free(rp);
        }
    }
}

#[test]
fn c16_computation_result_layout_matches_across_the_abi() {
    // Layout the harness assumes, and which both libraries must agree on.
    assert_eq!(std::mem::size_of::<ComputationResult>(), 24);
    assert_eq!(std::mem::align_of::<ComputationResult>(), 8);

    // Write a record with one library and read the fields back through the
    // other library's view of the same memory: a layout mismatch (e.g. a
    // packed or reordered struct) would show up as a garbled field.
    let (c, r) = both();
    let block = unsafe { (c.allocate_results)(SLOTS as i32) };
    assert!(!block.is_null());
    let mut h = block;
    let mut n: c_int = 0;
    // C fills slot 0, Rust fills slot 1, in the same C-allocated block.
    let cv = unsafe { (c.perform_computation_with_history)(11, 7, OP_ADD, &mut h, &mut n) };
    let rv = unsafe { (r.perform_computation_with_history)(11, 7, OP_ADD, &mut h, &mut n) };
    assert_eq!(cv, 18);
    assert_eq!(rv, 18);
    assert_eq!(n, 2);
    unsafe {
        let s0 = *block;
        let s1 = *block.add(1);
        assert_eq!(s0.value, 18);
        assert_eq!(s1.value, 18);
        assert_eq!(s0.status, STATUS_SUCCESS);
        assert_eq!(s1.status, STATUS_SUCCESS);
        assert_eq!(
            s0.timestamp, s1.timestamp,
            "both libraries must write the same time()>>29 value"
        );
        // Offsets: reading the struct as raw i64/i32 words must line up.
        let words = block as *const i32;
        assert_eq!(*words.add(0), 18, "value at offset 0");
        // offset 8 == word index 2, offset 16 == word index 4.
        assert_eq!(
            *(words.add(2) as *const i64),
            s0.timestamp,
            "timestamp at offset 8"
        );
        assert_eq!(*words.add(4), STATUS_SUCCESS, "status at offset 16");
        // sizeof == 24 => slot 1 starts at word index 6.
        assert_eq!(*words.add(6), 18, "slot 1 value at offset 24");
        libc_free(block);
    }
}

// ---------------------------------------------------------------------------
// C17 / C18 — the lazy-allocation path (*history == NULL).
// ---------------------------------------------------------------------------

fn diff_lazy_alloc(a: c_int, b: c_int, op: c_int, initial_count: c_int) {
    let (c, r) = both();
    let mut ch: *mut ComputationResult = std::ptr::null_mut();
    let mut rh: *mut ComputationResult = std::ptr::null_mut();
    let mut cn: c_int = initial_count;
    let mut rn: c_int = initial_count;

    let cv = unsafe { (c.perform_computation_with_history)(a, b, op, &mut ch, &mut cn) };
    let rv = unsafe { (r.perform_computation_with_history)(a, b, op, &mut rh, &mut rn) };

    assert_eq!(cv, rv, "lazy({a},{b},{op},{initial_count}): return");
    assert_eq!(cn, rn, "lazy({a},{b},{op},{initial_count}): count");
    assert_eq!(cn, 1, "count must be reset to 0 then incremented");
    assert!(!ch.is_null() && !rh.is_null(), "both must have allocated");

    unsafe {
        let cs = *ch;
        let rs = *rh;
        assert_eq!(cs.value, rs.value, "slot0.value");
        assert_eq!(cs.status, rs.status, "slot0.status");
        assert_eq!(cs.status, STATUS_SUCCESS);
        assert_eq!(cs.timestamp, rs.timestamp, "slot0.timestamp");
        // The other nine slots must still be calloc-zeroed in both.
        for i in 1..SLOTS {
            assert_eq!(*ch.add(i), ComputationResult::default(), "C slot {i}");
            assert_eq!(*rh.add(i), ComputationResult::default(), "Rust slot {i}");
        }
        libc_free(ch);
        libc_free(rh);
    }
}

#[test]
fn c17_history_null_lazy_allocation() {
    let mut rng = Rng::new(0x17_0000);
    for op in [OP_ADD, OP_MULTIPLY, OP_SUBTRACT, OP_DIVIDE, OP_MODULO] {
        for _ in 0..32 {
            let a = rng.spicy_i32();
            let mut b = rng.spicy_i32();
            if (op == OP_DIVIDE || op == OP_MODULO) && is_idiv_ub(a, b) {
                b = 3;
            }
            diff_lazy_alloc(a, b, op, 0);
        }
    }
}

#[test]
fn c18_history_null_resets_nonzero_count() {
    for count in [1i32, 7, 9, 10, 11, 100, -4] {
        diff_lazy_alloc(42, 5, OP_ADD, count);
    }
}

// ---------------------------------------------------------------------------
// C19 / C20 / C21 / C22 — caller-owned buffer, fill-level sweep.
// ---------------------------------------------------------------------------

#[test]
fn c19_caller_buffer_count_sweep() {
    let mut rng = Rng::new(0x19_0000);
    for count in 0..=9 {
        for op in [OP_ADD, OP_MULTIPLY, OP_SUBTRACT, OP_DIVIDE, OP_MODULO] {
            for k in 0..12 {
                let a = rng.spicy_i32();
                let mut b = rng.spicy_i32();
                if (op == OP_DIVIDE || op == OP_MODULO) && is_idiv_ub(a, b) {
                    b = 7;
                }
                diff_perform(
                    a,
                    b,
                    op,
                    count,
                    count * 10 + k,
                    &format!("C19 count={count} op={op} a={a} b={b}"),
                );
            }
        }
    }
}

#[test]
fn c20_last_writable_slot() {
    let (c, r) = both();
    let (mut cb, mut rb) = pair(20);
    let mut ch = cb.base();
    let mut rh = rb.base();
    let mut cn: c_int = 9;
    let mut rn: c_int = 9;
    let cv = unsafe { (c.perform_computation_with_history)(6, 7, OP_MULTIPLY, &mut ch, &mut cn) };
    let rv = unsafe { (r.perform_computation_with_history)(6, 7, OP_MULTIPLY, &mut rh, &mut rn) };
    assert_eq!(cv, 42);
    assert_eq!(rv, 42);
    assert_eq!(cn, 10);
    assert_eq!(rn, 10);
    assert_eq!(cb.cells[PAD + 9].value, 42);
    assert_eq!(rb.cells[PAD + 9].value, 42);
    assert_bufs_eq(&cb, &rb, "C20");
}

#[test]
fn c21_history_full_no_write() {
    let mut rng = Rng::new(0x21_0000);
    for count in [10i32, 11, 12, 100, i32::MAX] {
        for op in [OP_ADD, OP_MULTIPLY, OP_SUBTRACT, OP_DIVIDE, OP_MODULO, 0, 99] {
            let a = rng.spicy_i32();
            let mut b = rng.spicy_i32();
            if (op == OP_DIVIDE || op == OP_MODULO) && is_idiv_ub(a, b) {
                b = 5;
            }
            // Buffer must come back untouched and the count unchanged.
            let (c, r) = both();
            let (mut cb, mut rb) = pair(21);
            let pristine = Buf::new(21);
            let mut ch = cb.base();
            let mut rh = rb.base();
            let mut cn = count;
            let mut rn = count;
            let cv = unsafe { (c.perform_computation_with_history)(a, b, op, &mut ch, &mut cn) };
            let rv = unsafe { (r.perform_computation_with_history)(a, b, op, &mut rh, &mut rn) };
            assert_eq!(cv, rv, "C21 count={count} op={op}");
            assert_eq!(cn, count, "C count must be unchanged when full");
            assert_eq!(rn, count, "Rust count must be unchanged when full");
            assert_bufs_eq(&cb, &pristine, "C21 C buffer must be untouched");
            assert_bufs_eq(&rb, &pristine, "C21 Rust buffer must be untouched");
        }
    }
}

#[test]
fn c22_negative_count_writes_out_of_range() {
    // `*history_count < 0` passes the `< 10` check, so the C code writes at a
    // negative index. The padded buffer makes that land in owned memory, which
    // lets the (identical) behaviour be observed instead of crashing.
    let mut rng = Rng::new(0x22_0000);
    for count in [-1i32, -2, -3, -8] {
        for op in [OP_ADD, OP_MULTIPLY, OP_SUBTRACT, OP_DIVIDE, OP_MODULO, -7] {
            let a = rng.spicy_i32();
            let mut b = rng.spicy_i32();
            if (op == OP_DIVIDE || op == OP_MODULO) && is_idiv_ub(a, b) {
                b = 11;
            }
            let (c, r) = both();
            let (mut cb, mut rb) = pair(22);
            let mut ch = cb.base();
            let mut rh = rb.base();
            let mut cn = count;
            let mut rn = count;
            let cv = unsafe { (c.perform_computation_with_history)(a, b, op, &mut ch, &mut cn) };
            let rv = unsafe { (r.perform_computation_with_history)(a, b, op, &mut rh, &mut rn) };
            assert_eq!(cv, rv, "C22 count={count} op={op}");
            assert_eq!(cn, count + 1, "C count increments toward zero");
            assert_eq!(rn, count + 1, "Rust count increments toward zero");
            let idx = (PAD as i32 + count) as usize;
            assert_eq!(cb.cells[idx].value, cv, "C wrote at negative index");
            assert_eq!(rb.cells[idx].value, rv, "Rust wrote at negative index");
            assert_bufs_eq(&cb, &rb, "C22");
        }
    }
}

// ---------------------------------------------------------------------------
// C23 / C24 — out-of-range op, and div/mod by zero, through the recorder.
// ---------------------------------------------------------------------------

#[test]
fn c23_out_of_range_op_through_recorder() {
    let mut rng = Rng::new(0x23_0000);
    let ops = [0i32, 6, -1, -5, 7, 100, i32::MIN, i32::MAX];
    for &op in &ops {
        for k in 0..24 {
            let a = rng.spicy_i32();
            let b = rng.spicy_i32();
            diff_perform(a, b, op, k % 10, 23, &format!("C23 op={op} a={a} b={b}"));
        }
    }
}

#[test]
fn c24_div_mod_by_zero_through_recorder() {
    let mut rng = Rng::new(0x24_0000);
    for op in [OP_DIVIDE, OP_MODULO] {
        for count in 0..10 {
            let a = rng.spicy_i32();
            diff_perform(a, 0, op, count, 24, &format!("C24 op={op} a={a}"));
        }
    }
    // And the recorded value must actually be the guard's 0.
    let (c, r) = both();
    let (mut cb, mut rb) = pair(24);
    let mut ch = cb.base();
    let mut rh = rb.base();
    let mut cn = 0;
    let mut rn = 0;
    let cv = unsafe { (c.perform_computation_with_history)(i32::MIN, 0, OP_DIVIDE, &mut ch, &mut cn) };
    let rv = unsafe { (r.perform_computation_with_history)(i32::MIN, 0, OP_DIVIDE, &mut rh, &mut rn) };
    assert_eq!(cv, 0);
    assert_eq!(rv, 0);
    assert_eq!(cb.cells[PAD].value, 0);
    assert_eq!(rb.cells[PAD].value, 0);
    assert_eq!(cb.cells[PAD].status, STATUS_SUCCESS);
    assert_eq!(rb.cells[PAD].status, STATUS_SUCCESS);
}

// ---------------------------------------------------------------------------
// C25 — long randomized sequences over one buffer, crossing the 10-slot limit.
// ---------------------------------------------------------------------------

#[test]
fn c25_long_sequence_over_one_buffer() {
    let (c, r) = both();
    for round in 0..24u64 {
        let mut rng = Rng::new(0x25_0000 + round);
        let (mut cb, mut rb) = pair(25);
        let mut ch = cb.base();
        let mut rh = rb.base();
        let mut cn: c_int = 0;
        let mut rn: c_int = 0;

        for step in 0..14 {
            let op = rng.range(-2, 8);
            let a = rng.spicy_i32();
            let mut b = rng.spicy_i32();
            if (op == OP_DIVIDE || op == OP_MODULO) && is_idiv_ub(a, b) {
                b = 13;
            }
            let cv = unsafe { (c.perform_computation_with_history)(a, b, op, &mut ch, &mut cn) };
            let rv = unsafe { (r.perform_computation_with_history)(a, b, op, &mut rh, &mut rn) };
            assert_eq!(cv, rv, "C25 round={round} step={step} op={op} ({a},{b})");
            assert_eq!(cn, rn, "C25 round={round} step={step}: count");
            assert_bufs_eq(&cb, &rb, &format!("C25 round={round} step={step}"));
        }
        assert_eq!(cn, 10, "the sequence must saturate the history");
    }
}

// ---------------------------------------------------------------------------
// C26 — cross-ABI interop: one library's block driven by the other's code.
// ---------------------------------------------------------------------------

#[test]
fn c26_cross_library_buffer_interop() {
    let (c, r) = both();
    let mut rng = Rng::new(0x26_0000);

    for (allocator, driver) in [(c, r), (r, c)] {
        let block = unsafe { (allocator.allocate_results)(SLOTS as i32) };
        assert!(!block.is_null());
        let mut h = block;
        let mut n: c_int = 0;
        let mut expected: Vec<i32> = Vec::new();
        for _ in 0..SLOTS {
            let op = rng.range(1, 5);
            let a = rng.range(-1000, 1000);
            let b = rng.range(1, 1000);
            let v = unsafe { (driver.perform_computation_with_history)(a, b, op, &mut h, &mut n) };
            expected.push(v);
        }
        assert_eq!(n, SLOTS as c_int);
        unsafe {
            for i in 0..SLOTS {
                let s = *block.add(i);
                assert_eq!(
                    s.value, expected[i],
                    "{} block driven by {}: slot {i}",
                    allocator.name, driver.name
                );
                assert_eq!(s.status, STATUS_SUCCESS);
            }
            libc_free(block);
        }
    }
}
