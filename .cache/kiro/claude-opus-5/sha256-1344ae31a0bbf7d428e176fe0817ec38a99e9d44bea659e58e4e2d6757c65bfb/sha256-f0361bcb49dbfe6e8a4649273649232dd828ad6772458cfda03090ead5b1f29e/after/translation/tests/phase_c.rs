//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI-boundary rows (G1-G9).
//! Every test asserts the SAME sentinel / error code and the SAME error text on
//! stdout, not merely "both failed".

mod common;
use common::*;

use std::ffi::{c_char, c_int};

fn cap(f: impl FnOnce() -> i64) -> (i64, Vec<u8>) {
    capture(f)
}

// ---------------------------------------------------------------------------
// Row 1 — malloc failure for the ProcessState itself.
//
// This branch is not reachable from any argument value: `sizeof(ProcessState)`
// is a fixed 24 bytes and both libraries call the identical libc `malloc`.
// What IS differentially testable, and what the row really asserts, is that
//   (a) both `.so`s carry the byte-identical diagnostic string, and
//   (b) both agree on the struct layout that the size is derived from.
// (b) is asserted by every Phase B row (fields are read back through the same
// `#[repr(C)]` mirror); (a) is asserted here against the shipped binaries.
// ---------------------------------------------------------------------------
#[test]
fn row01_state_alloc_failure_message_is_identical_in_both_binaries() {
    use std::path::PathBuf;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = {
        let build = root.join("../c_src/build");
        std::fs::read_dir(&build)
            .expect("C build dir — build the C library first")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .find(|p| p.extension().map(|x| x == "so").unwrap_or(false))
            .expect("C .so")
    };
    let r_so = ["release", "debug"]
        .iter()
        .map(|p| root.join("target").join(p).join("libconfusion_lib.so"))
        .find(|p| p.exists())
        .expect("Rust .so");

    let c_bytes = std::fs::read(&c_so).unwrap();
    let r_bytes = std::fs::read(&r_so).unwrap();

    let contains = |hay: &[u8], needle: &[u8]| hay.windows(needle.len()).any(|w| w == needle);

    for msg in [
        // Both compilers rewrite `printf("...\n")` (no conversions) into
        // `puts("...")`, so the stored literal has no trailing newline.
        &b"Error: Failed to allocate memory for state"[..],
        &b"Error: Failed to allocate buffer"[..],
        &b"Error: Null pointer in process_buffer"[..],
    ] {
        assert!(contains(&c_bytes, msg), "C .so is missing {:?}", String::from_utf8_lossy(msg));
        assert!(
            contains(&r_bytes, msg),
            "Rust .so is missing the diagnostic {:?} — the error path cannot \
             produce byte-identical output",
            String::from_utf8_lossy(msg)
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 2, 3, 4 — buffer allocation failure via the capacity argument.
// ---------------------------------------------------------------------------
fn diff_create(label: &str, initial: c_int, capacity: c_int, read_buffer: bool) {
    let p = pair();
    let (sc, oc) = capture(|| unsafe { (p.c.create_state)(initial, capacity) });
    let (sr, or) = capture(|| unsafe { (p.r.create_state)(initial, capacity) });

    assert_eq!(
        sc.is_null(),
        sr.is_null(),
        "[{label}] create_state({initial},{capacity}) NULL-ness differs: C null={} Rust null={}",
        sc.is_null(),
        sr.is_null()
    );
    assert_eq!(
        oc,
        or,
        "[{label}] create_state({initial},{capacity}) stdout differs:\n  C   = {}\n  Rust= {}",
        show(&oc),
        show(&or)
    );

    if !sc.is_null() {
        let a = unsafe { *sc };
        let b = unsafe { *sr };
        assert_eq!(decode_flags(a.flags), decode_flags(b.flags), "[{label}] flags differ");
        assert_eq!(a.data, b.data, "[{label}] data differs");
        assert_eq!(a.capacity, b.capacity, "[{label}] capacity differs");
        assert_eq!(a.buffer.is_null(), b.buffer.is_null(), "[{label}] buffer NULL-ness differs");
        if read_buffer {
            assert_eq!(
                unsafe { p.c.buffer_bytes(sc) },
                unsafe { p.r.buffer_bytes(sr) },
                "[{label}] buffer bytes differ"
            );
        }
        let _ = capture(|| unsafe { (p.c.destroy_state)(sc) });
        let _ = capture(|| unsafe { (p.r.destroy_state)(sr) });
    }
}

#[test]
fn row02_create_state_negative_capacity() {
    let mut rng = Rng::new(0xC0002);
    let mut caps: Vec<i32> = vec![-1, -2, -8, -128, -4096, -1_000_000];
    for _ in 0..200 {
        caps.push(-(1 + (rng.next_u32() % (i32::MAX as u32)) as i64) as i32);
    }
    for c in caps {
        diff_create("row2", 42, c, true);
    }
}

#[test]
fn row03_create_state_capacity_int_min() {
    for v in [0i32, 1, -1, i32::MIN, i32::MAX] {
        diff_create("row3", v, i32::MIN, true);
        diff_create("row3", v, i32::MIN + 1, true);
    }
}

#[test]
fn row04_create_state_capacity_int_max() {
    // 2 GiB request: may succeed or fail depending on overcommit, but both
    // implementations must agree.
    for v in [0i32, -1, i32::MAX, i32::MIN] {
        diff_create("row4", v, i32::MAX, true);
        diff_create("row4", v, i32::MAX - 1, true);
        diff_create("row4", v, 1 << 30, true);
    }
}

// ---------------------------------------------------------------------------
// Rows 5, 6 / G1 — process_buffer rejects NULL state and NULL buffer.
// ---------------------------------------------------------------------------
#[test]
fn row05_process_buffer_null_state() {
    let p = pair();
    for t in [0i8, 1, b'0' as i8, b':' as i8, -1, -128, 127] {
        let (rc, oc) = cap(|| unsafe { (p.c.process_buffer)(std::ptr::null_mut(), t) as i64 });
        let (rr, or) = cap(|| unsafe { (p.r.process_buffer)(std::ptr::null_mut(), t) as i64 });
        assert_eq!(rc, -1, "C did not return -1 for NULL state");
        assert_eq!(rr, rc, "row5 return differs for target {t}");
        assert_eq!(oc, or, "row5 stdout differs:\n  C   = {}\n  Rust= {}", show(&oc), show(&or));
        assert_eq!(oc, b"Error: Null pointer in process_buffer\n".to_vec());
    }
}

#[test]
fn row06_process_buffer_null_buffer_field() {
    let p = pair();
    // A caller-constructed state whose `buffer` field is NULL. Both libraries
    // must take the guard branch before dereferencing it.
    for t in [0i8, b'S' as i8, -1] {
        let mut st = ProcessState {
            flags: 0x7B05,
            data: 0,
            buffer: std::ptr::null_mut(),
            capacity: 128,
        };
        let ptr: *mut ProcessState = &mut st;
        let (rc, oc) = cap(|| unsafe { (p.c.process_buffer)(ptr, t) as i64 });
        let (rr, or) = cap(|| unsafe { (p.r.process_buffer)(ptr, t) as i64 });
        assert_eq!(rc, -1, "C did not return -1 for NULL buffer");
        assert_eq!(rr, rc, "row6 return differs for target {t}");
        assert_eq!(oc, or, "row6 stdout differs");
        assert_eq!(oc, b"Error: Null pointer in process_buffer\n".to_vec());
    }
}

// ---------------------------------------------------------------------------
// Rows 7, 8, 9 / G8 — process_buffer loop-termination paths, driven with
// caller-supplied buffer contents so the memchr loop sees arbitrary bytes.
// ---------------------------------------------------------------------------
fn diff_process_on_bytes(label: &str, content: &[u8], target: c_char) {
    let p = pair();
    // NUL-terminated, caller-owned buffer. `process_buffer` never writes to it
    // and we never hand the state to `destroy_state`, so a Rust-owned Vec is
    // fine for both libraries.
    let mut buf: Vec<u8> = content.to_vec();
    buf.push(0);

    let mut st = ProcessState {
        flags: 0x7B05,
        data: 0,
        buffer: buf.as_mut_ptr() as *mut c_char,
        capacity: buf.len() as c_int,
    };
    let ptr: *mut ProcessState = &mut st;

    let (rc, oc) = cap(|| unsafe { (p.c.process_buffer)(ptr, target) as i64 });
    let (rr, or) = cap(|| unsafe { (p.r.process_buffer)(ptr, target) as i64 });
    assert_eq!(
        rc, rr,
        "[{label}] count differs for target {target} on {:?}: C={rc} Rust={rr}",
        String::from_utf8_lossy(content)
    );
    assert_eq!(
        oc,
        or,
        "[{label}] stdout differs for target {target}:\n  C   = {}\n  Rust= {}",
        show(&oc),
        show(&or)
    );
}

#[test]
fn row07_process_buffer_target_never_found() {
    for content in [
        &b"abc"[..],
        &b"State:1:Mode:3"[..],
        &b"\x01\x02\x03"[..],
        &[0xffu8, 0xfe, 0xfd][..],
    ] {
        for t in [b'Z' as c_char, 0x7f, -2] {
            if content.contains(&(t as u8)) {
                continue;
            }
            diff_process_on_bytes("row7", content, t);
        }
    }
}

#[test]
fn row08_process_buffer_empty_buffer() {
    for t in [0i8, b'a' as i8, -1, 127, -128] {
        diff_process_on_bytes("row8", b"", t);
    }
}

#[test]
fn row09_process_buffer_nul_target_never_matches() {
    for content in [&b""[..], &b"a"[..], &b"State:0:Mode:3"[..], &[0x80u8, 0x00, 0x41][..]] {
        diff_process_on_bytes("row9", content, 0);
    }
}

#[test]
fn row_extra_process_buffer_randomized_bytes() {
    // Exercises the memchr loop with every byte value, including 0x80..0xff,
    // and with the target at the first byte, the last byte, and every position.
    let mut rng = Rng::new(0xC0009);
    for _ in 0..400 {
        let len = (rng.next_u32() % 40) as usize;
        let mut content: Vec<u8> = Vec::with_capacity(len);
        for _ in 0..len {
            // Non-NUL bytes only, so `strlen` sees the whole buffer.
            content.push(1 + (rng.next_u32() % 255) as u8);
        }
        let t = rng.range_i32(-128, 127) as c_char;
        diff_process_on_bytes("row_extra", &content, t);
    }
    // Dense small alphabet => many repeats, adjacent matches, matches at both ends.
    for _ in 0..400 {
        let len = (rng.next_u32() % 24) as usize;
        let content: Vec<u8> = (0..len).map(|_| b'a' + (rng.next_u32() % 3) as u8).collect();
        for t in [b'a' as c_char, b'b' as c_char, b'c' as c_char, b'd' as c_char] {
            diff_process_on_bytes("row_extra_dense", &content, t);
        }
    }
    // All-same buffers of every length: match at every position.
    for len in 0..40usize {
        let content: Vec<u8> = vec![b'x'; len];
        diff_process_on_bytes("row_extra_all_same", &content, b'x' as c_char);
    }
    // Every possible byte value present exactly once.
    let all: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    for t in -128..=127i32 {
        diff_process_on_bytes("row_extra_all_bytes", &all, t as c_char);
    }
}

// ---------------------------------------------------------------------------
// Row 10 / G2 — update_flags rejects NULL state silently.
// ---------------------------------------------------------------------------
#[test]
fn row10_update_flags_null_state() {
    let p = pair();
    for param in [0i32, 1, -1, i32::MIN, i32::MAX, 0x2A] {
        let (_, oc) = cap(|| unsafe {
            (p.c.update_flags)(std::ptr::null_mut(), param);
            0
        });
        let (_, or) = cap(|| unsafe {
            (p.r.update_flags)(std::ptr::null_mut(), param);
            0
        });
        assert!(oc.is_empty(), "C printed something for NULL state: {}", show(&oc));
        assert_eq!(oc, or, "row10 stdout differs for param {param}");
    }
}

// ---------------------------------------------------------------------------
// Rows 11, 12 / G3, G7 — confuse_types NULL state and out-of-range operation.
// ---------------------------------------------------------------------------
#[test]
fn row11_confuse_types_null_state() {
    let p = pair();
    for op in [0i32, 1, 2, 3, 4, -1, i32::MIN, i32::MAX] {
        let (rc, oc) = cap(|| unsafe { (p.c.confuse_types)(std::ptr::null_mut(), op) as i64 });
        let (rr, or) = cap(|| unsafe { (p.r.confuse_types)(std::ptr::null_mut(), op) as i64 });
        assert_eq!(rc, 0, "C did not return 0 for NULL state (op {op})");
        assert_eq!(rr, rc, "row11 return differs for op {op}");
        assert!(oc.is_empty(), "C printed for NULL state: {}", show(&oc));
        assert_eq!(oc, or, "row11 stdout differs for op {op}");
    }
}

#[test]
fn row12_confuse_types_operation_out_of_range() {
    // The C `switch` has no `default`, so any int outside {0,1,2,3} falls
    // straight through: result 0, no output, and `state->data` untouched.
    let p = pair();
    let mut ops: Vec<i32> = vec![4, 5, 6, 7, 8, 100, -1, -2, -3, -4, -100, i32::MIN, i32::MAX];
    let mut rng = Rng::new(0xC0012);
    for _ in 0..300 {
        let v = rng.next_i32();
        if !(0..=3).contains(&v) {
            ops.push(v);
        }
    }

    for op in ops {
        for initial in [0i32, 42, -42, i32::MIN, i32::MAX] {
            let run = |im: &Impl| {
                capture(|| unsafe {
                    let st = (im.create_state)(initial, 128);
                    let r = (im.confuse_types)(st, op) as i64;
                    let data = (*st).data;
                    let flags = (*st).flags;
                    (im.destroy_state)(st);
                    (r, data, flags)
                })
            };
            let ((rc, dc, fc), oc) = run(&p.c);
            let ((rr, dr, fr), or) = run(&p.r);
            assert_eq!(rc, 0, "C returned {rc} (expected 0) for out-of-range op {op}");
            assert_eq!(dc, initial as u32, "C mutated data for out-of-range op {op}");
            assert_eq!(rr, rc, "row12 return differs for op {op}, initial {initial}");
            assert_eq!(dr, dc, "row12 data word differs for op {op}");
            assert_eq!(fr, fc, "row12 flags word differs for op {op}");
            assert_eq!(oc, or, "row12 stdout differs for op {op}");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 13, 14 / G4 — destroy_state guards.
// ---------------------------------------------------------------------------
#[test]
fn row13_destroy_state_null() {
    let p = pair();
    let (_, oc) = cap(|| unsafe {
        (p.c.destroy_state)(std::ptr::null_mut());
        0
    });
    let (_, or) = cap(|| unsafe {
        (p.r.destroy_state)(std::ptr::null_mut());
        0
    });
    assert!(oc.is_empty(), "C printed on destroy_state(NULL): {}", show(&oc));
    assert_eq!(oc, or, "row13 stdout differs");
}

#[test]
fn row14_destroy_state_null_buffer_field() {
    // `state` was heap-allocated by the library under test but its `buffer`
    // field is NULL: only `state` may be freed, and nothing may be printed.
    // The buffer is intentionally leaked rather than freed twice.
    let p = pair();
    for im in [&p.c, &p.r] {
        let (_, out) = capture(|| unsafe {
            let st = (im.create_state)(7, 128);
            assert!(!st.is_null());
            (*st).buffer = std::ptr::null_mut(); // leaks the original buffer
            (im.destroy_state)(st);
            0
        });
        assert!(
            out.is_empty(),
            "{} printed on destroy_state with NULL buffer: {}",
            im.name,
            show(&out)
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 15, 16, 17 — confusion's own rejection-shaped paths.
// ---------------------------------------------------------------------------
fn diff_confusion(label: &str, a: i32, b: i32, c: i32, d: i32) {
    let p = pair();
    let (rc, oc) = cap(|| unsafe { (p.c.confusion)(a, b, c, d) as i64 });
    let (rr, or) = cap(|| unsafe { (p.r.confusion)(a, b, c, d) as i64 });
    assert_eq!(rc, rr, "[{label}] confusion({a},{b},{c},{d}) return differs: C={rc} Rust={rr}");
    assert_eq!(
        oc,
        or,
        "[{label}] confusion({a},{b},{c},{d}) stdout differs:\n  C   = {}\n  Rust= {}",
        show(&oc),
        show(&or)
    );
}

#[test]
fn row15_confusion_create_state_failure_is_unreachable_but_consistent() {
    // `confusion` hardcodes capacity 128, so create_state cannot fail; assert
    // that neither implementation ever takes the `-1` early return, and that
    // they agree.
    let p = pair();
    let mut rng = Rng::new(0xC0015);
    for _ in 0..200 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let (rc, oc) = cap(|| unsafe { (p.c.confusion)(a, b, c, d) as i64 });
        let (rr, or) = cap(|| unsafe { (p.r.confusion)(a, b, c, d) as i64 });
        assert!(
            !oc.windows(9).any(|w| w == b"Error: Fa"),
            "unexpected allocation failure in C for ({a},{b},{c},{d})"
        );
        assert_eq!(rc, rr, "row15 return differs");
        assert_eq!(oc, or, "row15 stdout differs");
    }
}

#[test]
fn row16_confusion_negative_param3_gives_non_digit_search_byte() {
    // param3 < 0  =>  param3 % 10 in {-9..-1}  =>  search byte in {39..47},
    // i.e. '\'' .. '/', none of which occur in "State:<n>:Mode:<m>" unless the
    // value is negative (which contributes '-' == 45).
    let mut rng = Rng::new(0xC0016);
    for p3 in -30..0i32 {
        for _ in 0..10 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let d = rng.next_i32();
            diff_confusion("row16", a, b, p3, d);
        }
    }
    // '-' (45) is hit by param3 % 10 == -3; pair it with a negative param1 so
    // the buffer actually contains a '-'.
    for p1 in [-1i32, -7, -12345, i32::MIN, -999999999] {
        for p3 in [-3i32, -13, -23, -1003] {
            for p4 in [0i32, 1, 2, 3, -1] {
                diff_confusion("row16-minus", p1, 0, p3, p4);
                diff_confusion("row16-minus", p1, 0x2A, p3, p4);
            }
        }
    }
}

#[test]
fn row17_confusion_negative_param4_hits_no_switch_arm() {
    let mut rng = Rng::new(0xC0017);
    for p4 in [-1i32, -2, -3, -5, -6, -7, -9, -10, -11, i32::MIN, i32::MIN + 1, i32::MIN + 2] {
        for _ in 0..20 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let c = rng.next_i32();
            diff_confusion("row17", a, b, c, p4);
        }
    }
}

// ---------------------------------------------------------------------------
// G5, G6, G9 — remaining generic boundaries.
// ---------------------------------------------------------------------------
#[test]
fn g05_create_state_capacity_zero() {
    // malloc(0) returns a non-NULL zero-length block and snprintf writes
    // nothing, so the buffer is left unterminated. Only NULL-ness, the flags
    // word, the data word and stdout are defined here; the buffer contents are
    // not, in EITHER language, so they are deliberately not compared.
    for v in [0i32, 1, -1, i32::MIN, i32::MAX, 424242] {
        diff_create("G5", v, 0, false);
    }
}

#[test]
fn g06_create_state_capacity_one() {
    for v in [0i32, 1, -1, i32::MIN, i32::MAX] {
        diff_create("G6", v, 1, true);
    }
}

#[test]
fn g09_create_state_extreme_initial_val() {
    for v in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -1, 1] {
        for c in [1i32, 8, 15, 16, 17, 18, 128] {
            diff_create("G9", v, c, true);
        }
    }
}

// ---------------------------------------------------------------------------
// Extra: out-of-range values across the whole public surface at once.
// ---------------------------------------------------------------------------
#[test]
fn extra_out_of_range_everything() {
    let p = pair();
    let extremes = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];
    for &a in &extremes {
        for &b in &extremes {
            for &c in &extremes {
                for &d in &extremes {
                    let (rc, oc) = cap(|| unsafe { (p.c.confusion)(a, b, c, d) as i64 });
                    let (rr, or) = cap(|| unsafe { (p.r.confusion)(a, b, c, d) as i64 });
                    assert_eq!(rc, rr, "extremes ({a},{b},{c},{d}) return differs");
                    assert_eq!(
                        oc,
                        or,
                        "extremes ({a},{b},{c},{d}) stdout differs:\n  C   = {}\n  Rust= {}",
                        show(&oc),
                        show(&or)
                    );
                }
            }
        }
    }
}
