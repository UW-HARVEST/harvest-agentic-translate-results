// Phase C -- error-path differential tests.
// One test per row of ERRORS.md, plus the generic FFI boundary cases.
//
// Every public function returns `void`, so "the same rejection" is asserted as
// "the same exact bytes on stdout" -- for E1 that is the empty byte string,
// which is checked explicitly rather than merely "both produced something".

mod harness;
use harness::*;

use std::ffi::{c_char, c_int};

/// E1: `printLine(NULL)` -- the library's ONE input guard (driver.c:32).
/// Both must return normally and write EXACTLY zero bytes.
#[test]
fn err_e1_print_line_null() {
    let c = capture(|| c_api().print_line(std::ptr::null()));
    let r = capture(|| rust_api().print_line(std::ptr::null()));
    assert_eq!(c, r, "printLine(NULL) diverged");
    assert!(
        c.is_empty(),
        "printLine(NULL) must produce no output, C produced {c:?}"
    );
    assert!(
        r.is_empty(),
        "printLine(NULL) must produce no output, Rust produced {r:?}"
    );

    // The guard must still hold when NULL is interleaved with valid calls, and
    // must not consume/skip a subsequent line.
    assert_same("E1 NULL interleaved with valid payloads", |api| {
        api.print_line(std::ptr::null());
        with_cstr(b"after-null", |p| api.print_line(p));
        api.print_line(std::ptr::null());
        api.print_line(std::ptr::null());
        api.print_int_line(0);
        api.print_line(std::ptr::null());
    });

    // Many NULLs in a row: still exactly zero bytes.
    let c = capture(|| {
        for _ in 0..1000 {
            c_api().print_line(std::ptr::null());
        }
    });
    let r = capture(|| {
        for _ in 0..1000 {
            rust_api().print_line(std::ptr::null());
        }
    });
    assert_eq!(c, r);
    assert!(c.is_empty() && r.is_empty(), "1000x printLine(NULL) must be silent");
}

/// E2: empty string passes the guard and still emits the newline.
#[test]
fn err_e2_print_line_empty() {
    let c = capture(|| with_cstr(b"", |p| c_api().print_line(p)));
    let r = capture(|| with_cstr(b"", |p| rust_api().print_line(p)));
    assert_eq!(c, r, "printLine(\"\") diverged");
    assert_eq!(c, b"\n", "printLine(\"\") must emit exactly one newline");
}

/// E3: payload that looks like a format string must be echoed literally --
/// a `%n` interpreted as a conversion would be a memory-write primitive.
#[test]
fn err_e3_print_line_format_specifiers() {
    for p in [
        &b"%n"[..],
        b"%s",
        b"%d%d%d%d%d%d%d%d%d%d",
        b"%n%n%n%n%n%n%n%n",
        b"%99999999d",
        b"%.2147483647f",
        b"%1$n",
        b"%%n",
        b"AAAA%08x.%08x.%08x.%08x.%n",
    ] {
        assert_same_line("E3 format specifier payload", p);
        // And confirm the bytes really are echoed verbatim, not expanded.
        let out = capture(|| with_cstr(p, |q| c_api().print_line(q)));
        let mut expect = p.to_vec();
        expect.push(b'\n');
        assert_eq!(out, expect, "C did not echo {p:?} literally");
    }
}

/// E4: control bytes, non-UTF-8 bytes, and an INTERIOR NUL (output must stop
/// at the first NUL in both implementations).
#[test]
fn err_e4_print_line_non_utf8_and_control() {
    for p in [
        &b"\x01\x02\x03\x7f"[..],
        b"\xff\xfe\xfd",
        b"tab\there",
        b"nl\nin\nmiddle",
        b"\x80\x80\x80",
    ] {
        assert_same_line("E4 control/non-utf8", p);
    }

    // Interior NUL: buffer is "abc\0def\0", so both must print only "abc\n".
    let buf: &[u8] = b"abc\0def\0";
    let c = capture(|| c_api().print_line(buf.as_ptr() as *const c_char));
    let r = capture(|| rust_api().print_line(buf.as_ptr() as *const c_char));
    assert_eq!(c, r, "interior-NUL payload diverged");
    assert_eq!(c, b"abc\n", "output must stop at the first NUL");

    // NUL as the very first byte behaves like the empty string, not like NULL.
    let buf: &[u8] = b"\0trailing\0";
    let c = capture(|| c_api().print_line(buf.as_ptr() as *const c_char));
    let r = capture(|| rust_api().print_line(buf.as_ptr() as *const c_char));
    assert_eq!(c, r);
    assert_eq!(c, b"\n");
}

/// E5: oversized payloads -- no truncation at any stdio buffer size.
#[test]
fn err_e5_print_line_oversized() {
    let mut rng = Rng::new(SEED ^ 0xE5);
    for len in [
        8191usize,
        8192,
        8193, // BUFSIZ on glibc
        65535,
        65536,
        65537,
        1 << 20,
    ] {
        let p = rng.bytes(len);
        assert_same_line(&format!("E5 oversized len {len}"), &p);
        // No truncation: C output must be len + 1 bytes.
        let out = capture(|| with_cstr(&p, |q| c_api().print_line(q)));
        assert_eq!(out.len(), len + 1, "C truncated a {len}-byte payload");
        let out = capture(|| with_cstr(&p, |q| rust_api().print_line(q)));
        assert_eq!(out.len(), len + 1, "Rust truncated a {len}-byte payload");
    }
}

/// E6: `%d` at and one step past the int boundaries, incl. INT_MIN which has
/// no positive counterpart.
#[test]
fn err_e6_print_int_line_boundaries() {
    for v in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -1, 0] {
        assert_same(&format!("E6 printIntLine({v})"), |api| api.print_int_line(v));
    }
    // Spot-check the exact rendering of the awkward one.
    let out = capture(|| c_api().print_int_line(i32::MIN));
    assert_eq!(out, b"-2147483648\n");
    let out = capture(|| rust_api().print_int_line(i32::MIN));
    assert_eq!(out, b"-2147483648\n");

    // Values whose bit pattern comes in as an unsigned 32-bit quantity from a
    // C caller: `%d` must reinterpret them as signed.
    for u in [0x8000_0000u32, 0xFFFF_FFFF, 0x7FFF_FFFF, 0xDEAD_BEEF] {
        let v = u as i32;
        assert_same(&format!("E6 printIntLine(0x{u:08x})"), |api| {
            api.print_int_line(v)
        });
    }
}

/// E7: `useGood == 0` is the only value routed to the intentionally buggy
/// `bad()` (CWE-806 `alloca(10)` under-allocation). It must not trap.
#[test]
fn err_e7_driver_zero_selects_bad() {
    let c = capture(|| c_api().driver(0));
    let r = capture(|| rust_api().driver(0));
    assert_eq!(c, r, "driver(0) diverged");
    assert_eq!(c, b"0\n", "driver(0) must print the copied data[0]");

    // Hammer the buggy path; a real trap/abort would kill the process here.
    assert_same("E7 driver(0) x500", |api| {
        for _ in 0..500 {
            api.driver(0);
        }
    });
}

/// E8: out-of-`bool`-range ints across the FFI boundary. A C `int` accepts any
/// bit pattern where a flag/enum was implied, so every non-zero value must take
/// the `good()` branch -- the Rust must test `!= 0`, never `== 1`.
#[test]
fn err_e8_driver_out_of_range_int() {
    let mut vals: Vec<i32> = vec![
        -1,
        2,
        3,
        -2,
        i32::MIN,
        i32::MAX,
        0x100,
        0xFFFF,
        0x7FFF_FFFF,
        0xFFFF_FF00u32 as i32, // no valid "flag" variant
        0x0000_0100,
        0x0001_0000,
        1 << 31,
        -(1 << 30),
    ];
    // Values whose LOW BYTE is zero but which are non-zero overall: these catch
    // a translation that truncated the flag to i8/u8 or bool.
    vals.extend_from_slice(&[0x100, 0x200, 0x1_0000, 0x0100_0000, -256, -65536]);

    for v in &vals {
        let c = capture(|| c_api().driver(*v));
        let r = capture(|| rust_api().driver(*v));
        assert_eq!(c, r, "driver({v}) diverged");
        assert_eq!(c, b"0\n", "driver({v}) must print 0");
    }

    // The output of driver(non-zero) must equal good(), and driver(0) must
    // equal bad(), for both libraries.
    for api in [c_api(), rust_api()] {
        let via_driver = capture(|| api.driver(-1));
        let via_good = capture(|| api.good());
        assert_eq!(
            via_driver, via_good,
            "[{}] driver(-1) must dispatch to good()",
            api.which
        );
        let via_driver = capture(|| api.driver(0));
        let via_bad = capture(|| api.bad());
        assert_eq!(
            via_driver, via_bad,
            "[{}] driver(0) must dispatch to bad()",
            api.which
        );
    }
}

/// E9: the out-of-bounds-write path itself, called directly.
#[test]
fn err_e9_bad_direct_no_trap() {
    let c = capture(|| c_api().bad());
    let r = capture(|| rust_api().bad());
    assert_eq!(c, r, "bad() diverged");
    assert_eq!(c, b"0\n", "bad() must print data[0] == 0");
}

/// E10: repeated / alternating calls, so a corrupted frame would surface later.
#[test]
fn err_e10_repeated_alternating_calls() {
    assert_same("E10 bad/good/driver interleaved x300", |api| {
        for i in 0..300 {
            match i % 4 {
                0 => api.bad(),
                1 => api.good(),
                2 => api.driver(0),
                _ => api.driver(1),
            }
        }
    });

    // And with printLine/printIntLine wedged in between, so a stack smash in
    // bad() would be visible as corrupted formatting of a neighbour's output.
    assert_same("E10 bad() surrounded by other calls", |api| {
        for i in 0..200 {
            api.print_int_line(i);
            api.bad();
            with_cstr(b"sentinel-payload-0123456789", |p| api.print_line(p));
            api.bad();
            api.print_int_line(-i);
        }
    });
}

// --------------------------------------------------------- generic FFI boundaries

/// Every pointer-taking entry point, given NULL. `printLine` is the only one,
/// and it is guarded; assert the guard, not a crash.
#[test]
fn boundary_null_pointer_all_entry_points() {
    assert_same("null ptr to printLine, repeated", |api| {
        for _ in 0..64 {
            api.print_line(std::ptr::null());
        }
    });

    // A misaligned-but-valid pointer into the middle of a buffer.
    let buf: Vec<u8> = b"0123456789abcdef\0".to_vec();
    for off in 0..16usize {
        let p = unsafe { buf.as_ptr().add(off) } as *const c_char;
        let c = capture(|| c_api().print_line(p));
        let r = capture(|| rust_api().print_line(p));
        assert_eq!(c, r, "printLine(buf+{off}) diverged");
    }
}

/// Zero and oversized lengths (expressed as payload sizes, the only length-like
/// quantity in this API).
#[test]
fn boundary_zero_and_oversized_lengths() {
    assert_same_line("zero length", b"");
    let mut rng = Rng::new(SEED ^ 0xB0);
    // 2 MiB: well past every stdio buffer and past a single write() on most
    // kernels, so short-write handling is exercised too.
    let big = rng.bytes(2 * 1024 * 1024);
    assert_same_line("oversized 2MiB", &big);
}

/// Out-of-range "enum" values: the API has no enum type, so the widest such
/// input is `driver`'s `int` -- sweep the whole surface systematically,
/// including every single-bit value and a randomized full-range sample.
#[test]
fn boundary_out_of_range_enum_values() {
    let mut vals: Vec<i32> = vec![0, 1, -1, 2, -2, i32::MIN, i32::MAX];
    for k in 0..32u32 {
        vals.push((1u32 << k) as i32);
        vals.push(!(1u32 << k) as i32);
    }
    let mut rng = Rng::new(SEED ^ 0xB1);
    for _ in 0..512 {
        vals.push(rng.next_i32());
    }

    assert_same_chunked("out-of-range int flags", &vals, 64, |api, v| api.driver(*v));

    // Cross-check the dispatch rule holds for every value in both libraries.
    let good_out = capture(|| c_api().good());
    let bad_out = capture(|| c_api().bad());
    for v in &vals {
        let expect: &[u8] = if *v != 0 { &good_out } else { &bad_out };
        for api in [c_api(), rust_api()] {
            let out = capture(|| api.driver(*v));
            assert_eq!(
                out, expect,
                "[{}] driver({v}) took the wrong branch",
                api.which
            );
        }
    }
}

/// The exported symbols must be individually resolvable in BOTH libraries with
/// the exact C names, and callable with the C signature.
#[test]
fn boundary_all_symbols_resolvable_and_callable() {
    for api in [c_api(), rust_api()] {
        let f: unsafe extern "C" fn(c_int) = api.driver;
        let g: unsafe extern "C" fn() = api.good;
        let b: unsafe extern "C" fn() = api.bad;
        let pi: unsafe extern "C" fn(c_int) = api.print_int_line;
        let pl: unsafe extern "C" fn(*const c_char) = api.print_line;
        let out = capture(|| unsafe {
            f(1);
            g();
            b();
            pi(42);
            pl(b"sym\0".as_ptr() as *const c_char);
            pl(std::ptr::null());
        });
        assert_eq!(
            out, b"0\n0\n0\n42\nsym\n",
            "[{}] unexpected output from the five exports",
            api.which
        );
    }
}
