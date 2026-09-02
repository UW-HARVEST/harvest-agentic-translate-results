// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH shared objects through
// their exported C symbols and compares the captured stdout byte-for-byte.
// Randomised rows use the fixed-seed PRNG in `common` so failures reproduce.

mod common;

use common::*;
use std::ffi::c_char;

fn p(api: Api, buf: &[u8]) {
    assert_eq!(buf.last(), Some(&0), "test bug: buffer is not NUL-terminated");
    unsafe { (api.print_line)(buf.as_ptr() as *const c_char) }
}

// --- row 1: empty string ---------------------------------------------------

#[test]
fn row01_print_line_empty_string() {
    let buf = [0u8];
    assert_same("printLine(\"\")", |api| p(api, &buf));
    // Ground truth: the null guard passes and puts writes just the newline.
    let out = capture(|| p(c_api(), &buf));
    assert_eq!(out, b"\n", "C reference produced {out:?}");
}

// --- row 2: every single-byte string 0x01..=0xFF ---------------------------

#[test]
fn row02_print_line_every_single_byte() {
    for b in 1u8..=255 {
        let buf = [b, 0];
        assert_same(&format!("printLine(single byte {b:#04x})"), |api| {
            p(api, &buf)
        });
    }
}

// --- row 3: randomised printable ASCII, 1..=64 -----------------------------

#[test]
fn row03_print_line_random_ascii() {
    let mut rng = Rng::new(SEED);
    for i in 0..256 {
        let len = 1 + rng.below(64);
        let buf = random_cstr(&mut rng, len, true);
        assert_same(&format!("printLine(random ascii #{i}, len {len})"), |api| {
            p(api, &buf)
        });
    }
}

// --- row 4: randomised arbitrary non-NUL bytes (invalid UTF-8 included) ----

#[test]
fn row04_print_line_random_arbitrary_bytes() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..256 {
        let len = 1 + rng.below(256);
        let buf = random_cstr(&mut rng, len, false);
        assert_same(&format!("printLine(random bytes #{i}, len {len})"), |api| {
            p(api, &buf)
        });
    }
}

// --- row 5: printf format specifiers must be treated as DATA --------------

#[test]
fn row05_print_line_format_specifiers() {
    let cases: &[&[u8]] = &[
        b"%s\0",
        b"%n\0",
        b"%d\0",
        b"%%\0",
        b"%1000000d\0",
        b"%p %p %p %p %p %p %p %p\0",
        b"%s%s%s%s%s%s%s%s%s%s\0",
        b"%.999999f\0",
        b"%*d\0",
        b"%hn%hhn%lln\0",
        b"AAAA%08x.%08x.%08x.%08x\0",
        b"100%% done: %s -> %d\0",
    ];
    for c in cases {
        assert_same(&format!("printLine({:?})", String::from_utf8_lossy(c)), |api| {
            p(api, c)
        });
    }
}

// --- row 6: embedded control characters / newlines -------------------------

#[test]
fn row06_print_line_embedded_control_chars() {
    let cases: &[&[u8]] = &[
        b"\n\0",
        b"\n\n\n\0",
        b"a\nb\nc\0",
        b"line1\r\nline2\r\n\0",
        b"tab\there\0",
        b"vt\x0bff\x0c\0",
        b"trailing newline\n\0",
        b"\x7f\x1b[31mansi\x1b[0m\0",
    ];
    for c in cases {
        assert_same("printLine(control chars)", |api| p(api, c));
    }
}

// --- row 7: embedded NUL terminates early ---------------------------------

#[test]
fn row07_print_line_embedded_nul_truncation() {
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..128 {
        let total = 2 + rng.below(128);
        let cut = rng.below(total); // 0 => empty string
        let mut buf = random_cstr(&mut rng, total, false);
        buf[cut] = 0;
        assert_same(
            &format!("printLine(embedded NUL #{i}, total {total}, cut {cut})"),
            |api| p(api, &buf),
        );
    }
}

// --- row 8: stdio buffer boundaries ---------------------------------------

#[test]
fn row08_print_line_stdio_buffer_boundaries() {
    let mut rng = Rng::new(SEED ^ 8);
    for len in [1023usize, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193] {
        let buf = random_cstr(&mut rng, len, true);
        assert_same(&format!("printLine(len {len})"), |api| p(api, &buf));
        // Same length but a uniform fill, in case value distribution matters.
        let solid = filled_cstr(len, b'Z');
        assert_same(&format!("printLine(solid len {len})"), |api| p(api, &solid));
    }
}

// --- row 9: large payloads ------------------------------------------------

#[test]
fn row09_print_line_large_payloads() {
    let mut rng = Rng::new(SEED ^ 9);
    for len in [65535usize, 65536, 65537, 1 << 20] {
        let buf = random_cstr(&mut rng, len, true);
        assert_same(&format!("printLine(large len {len})"), |api| p(api, &buf));
    }
}

// --- row 10: interior pointer into a larger allocation --------------------

#[test]
fn row10_print_line_interior_pointer() {
    let mut rng = Rng::new(SEED ^ 10);
    let backing = random_cstr(&mut rng, 4096, true);
    for i in 0..128 {
        let off = rng.below(4096);
        let slice = &backing[off..]; // still NUL-terminated at the end
        assert_same(&format!("printLine(&buf[{off}]) #{i}"), |api| {
            unsafe { (api.print_line)(slice.as_ptr() as *const c_char) };
        });
    }
}

// --- row 11: back-to-back sequence in one capture --------------------------

#[test]
fn row11_print_line_sequence_ordering() {
    let mut rng = Rng::new(SEED ^ 11);
    for seq in 0..64 {
        let bufs: Vec<Vec<u8>> = (0..64)
            .map(|_| {
                let len = 1 + rng.below(48);
                random_cstr(&mut rng, len, true)
            })
            .collect();
        assert_same(&format!("printLine x64 sequence #{seq}"), |api| {
            for b in &bufs {
                p(api, b);
            }
        });
    }
}

// --- row 12: valid calls interleaved with NULL ----------------------------

#[test]
fn row12_print_line_interleaved_with_null() {
    let mut rng = Rng::new(SEED ^ 12);
    for seq in 0..64 {
        let items: Vec<Option<Vec<u8>>> = (0..32)
            .map(|_| {
                if rng.next_u64() % 3 == 0 {
                    None
                } else {
                    let len = 1 + rng.below(32);
                    Some(random_cstr(&mut rng, len, true))
                }
            })
            .collect();
        assert_same(&format!("printLine null-interleaved #{seq}"), |api| {
            for it in &items {
                match it {
                    None => unsafe { (api.print_line)(std::ptr::null()) },
                    Some(b) => p(api, b),
                }
            }
        });
    }
}

// --- rows 13/14: bad() ----------------------------------------------------

#[test]
fn row13_bad_single_call() {
    assert_same("bad()", |api| unsafe { (api.bad)() });
    // Ground truth: helperBad's dead stack address is materialised as NULL by
    // gcc at every -O level, so printLine's guard rejects it and nothing prints.
    let out = capture(|| unsafe { (c_api().bad)() });
    assert!(
        out.is_empty(),
        "C bad() unexpectedly produced {} bytes: {out:?}",
        out.len()
    );
}

#[test]
fn row14_bad_repeated() {
    assert_same("bad() x1000", |api| {
        for _ in 0..1000 {
            unsafe { (api.bad)() }
        }
    });
}

// --- rows 15/16: good() ---------------------------------------------------

#[test]
fn row15_good_single_call() {
    assert_same("good()", |api| unsafe { (api.good)() });
    let out = capture(|| unsafe { (c_api().good)() });
    assert_eq!(out, b"helperGood1 string\n", "C good() reference output");
}

#[test]
fn row16_good_repeated_static_storage() {
    assert_same("good() x1000", |api| {
        for _ in 0..1000 {
            unsafe { (api.good)() }
        }
    });
    // The array has static storage duration: identical bytes every iteration.
    let out = capture(|| {
        for _ in 0..1000 {
            unsafe { (c_api().good)() }
        }
    });
    assert_eq!(out.len(), b"helperGood1 string\n".len() * 1000);
}

// --- rows 17/18/19: driver() with fixed values ---------------------------

#[test]
fn row17_driver_true() {
    assert_same("driver(1)", |api| unsafe { (api.driver)(1) });
    assert_eq!(
        capture(|| unsafe { (c_api().driver)(1) }),
        b"helperGood1 string\n"
    );
}

#[test]
fn row18_driver_false() {
    assert_same("driver(0)", |api| unsafe { (api.driver)(0) });
    assert!(capture(|| unsafe { (c_api().driver)(0) }).is_empty());
}

#[test]
fn row19_driver_non_canonical_truthy() {
    for v in [
        -1i32,
        2,
        -2,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0x0001_0000,
        0xFFFF_0000u32 as i32,
        0x0000_FF00,
        256,
        -256,
    ] {
        assert_same(&format!("driver({v})"), |api| unsafe { (api.driver)(v) });
        assert_eq!(
            capture(|| unsafe { (c_api().driver)(v) }),
            b"helperGood1 string\n",
            "C driver({v}) should take the good path"
        );
    }
}

// --- row 20: randomised driver over the full i32 range -------------------

#[test]
fn row20_driver_randomised_full_range() {
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..1024 {
        let v = rng.next_u32() as i32;
        assert_same(&format!("driver(random {v}) #{i}"), |api| unsafe {
            (api.driver)(v)
        });
    }
}

// --- row 21: randomised driver biased towards zero -----------------------

#[test]
fn row21_driver_randomised_dense_branches() {
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..512 {
        let v = (rng.next_u64() % 3) as i32 - 1; // -1, 0, 1
        assert_same(&format!("driver(dense {v}) #{i}"), |api| unsafe {
            (api.driver)(v)
        });
    }
    for i in 0..512 {
        let v = (rng.next_u32() & 1) as i32;
        assert_same(&format!("driver(bit {v}) #{i}"), |api| unsafe {
            (api.driver)(v)
        });
    }
}

// --- row 22: randomised interleaving of all four entry points ------------

#[derive(Clone)]
enum Op {
    PrintLine(Vec<u8>),
    PrintNull,
    Bad,
    Good,
    Driver(i32),
}

#[test]
fn row22_all_entry_points_interleaved() {
    let mut rng = Rng::new(SEED ^ 22);
    for seq in 0..64 {
        let ops: Vec<Op> = (0..32)
            .map(|_| match rng.next_u64() % 5 {
                0 => {
                    let len = 1 + rng.below(40);
                    Op::PrintLine(random_cstr(&mut rng, len, true))
                }
                1 => Op::PrintNull,
                2 => Op::Bad,
                3 => Op::Good,
                _ => Op::Driver(if rng.next_u64() % 4 == 0 {
                    0
                } else {
                    rng.next_u32() as i32
                }),
            })
            .collect();
        assert_same(&format!("interleaved pipeline #{seq}"), |api| {
            for op in &ops {
                unsafe {
                    match op {
                        Op::PrintLine(b) => (api.print_line)(b.as_ptr() as *const c_char),
                        Op::PrintNull => (api.print_line)(std::ptr::null()),
                        Op::Bad => (api.bad)(),
                        Op::Good => (api.good)(),
                        Op::Driver(v) => (api.driver)(*v),
                    }
                }
            }
        });
    }
}

// --- row 23: no per-call init, no cross-call state divergence ------------

#[test]
fn row23_repeated_use_of_same_handles() {
    // 200 rounds hitting every entry point through the very same resolved
    // pointers, all inside one capture, to catch any first-call-only behaviour
    // or accumulating state.
    assert_same("all entry points, 200 rounds, same handles", |api| unsafe {
        let s = b"round\0";
        for _ in 0..200 {
            (api.print_line)(s.as_ptr() as *const c_char);
            (api.print_line)(std::ptr::null());
            (api.bad)();
            (api.good)();
            (api.driver)(0);
            (api.driver)(1);
        }
    });
}

// --- row 24: load order / no cross-object interposition -----------------

#[test]
fn row24_load_order_independence() {
    use libloading::Library;

    // Copy each .so to a unique path so dlopen really maps a fresh object
    // instead of returning the already-open handle.
    fn copy_to_tmp(src: &std::path::Path, tag: &str) -> std::path::PathBuf {
        let dst = std::env::temp_dir().join(format!(
            "driver_order_{}_{}_{}.so",
            std::process::id(),
            tag,
            src.file_stem().unwrap().to_string_lossy()
        ));
        std::fs::copy(src, &dst).expect("copy .so");
        dst
    }

    let run = |tag: &str, c_first: bool| -> (Vec<u8>, Vec<u8>) {
        let cp = copy_to_tmp(&c_so_path(), tag);
        let rp = copy_to_tmp(&rust_so_path(), tag);
        let (clib, rlib) = if c_first {
            let c = unsafe { Library::new(&cp) }.expect("dlopen C");
            let r = unsafe { Library::new(&rp) }.expect("dlopen Rust");
            (c, r)
        } else {
            let r = unsafe { Library::new(&rp) }.expect("dlopen Rust");
            let c = unsafe { Library::new(&cp) }.expect("dlopen C");
            (c, r)
        };
        let ca = resolve(&clib, "C");
        let ra = resolve(&rlib, "Rust");
        let body = |api: Api| unsafe {
            let s = b"order check\0";
            (api.print_line)(s.as_ptr() as *const c_char);
            (api.print_line)(std::ptr::null());
            (api.bad)();
            (api.good)();
            (api.driver)(0);
            (api.driver)(7);
        };
        let out = (capture(|| body(ca)), capture(|| body(ra)));
        drop(clib);
        drop(rlib);
        let _ = std::fs::remove_file(cp);
        let _ = std::fs::remove_file(rp);
        out
    };

    let (c1, r1) = run("cfirst", true);
    assert_eq!(c1, r1, "divergence when the C object is loaded first");
    let (c2, r2) = run("rfirst", false);
    assert_eq!(c2, r2, "divergence when the Rust object is loaded first");
    // And load order must not change either implementation's own output.
    assert_eq!(c1, c2, "C output depends on load order");
    assert_eq!(r1, r2, "Rust output depends on load order");
}
