//! Hostile-FFI differential tests: inputs a C caller can legally construct but
//! which the shaped `CONFIGS.md` rows cannot express — aliased out-parameters,
//! non-NULL-but-unreadable `content` combined with a zero-length scan, and
//! misaligned struct pointers. These probe assumptions the Rust translation
//! makes (e.g. holding a `&mut parse_buffer` across the `*mut cJSON` writes)
//! that the C does not make.

mod common;

use common::*;
use std::ffi::{c_int, c_uchar};

/// `item` and `input_buffer` overlapping in memory.
///
/// The C writes `item->valuedouble`, `item->valueint`, `item->type`, and only
/// THEN does `input_buffer->offset += ...`, re-reading `offset` after the `item`
/// writes may have clobbered it. Any Rust translation that caches `offset`
/// across those writes diverges here.
#[test]
fn h1_aliased_item_and_buffer() {
    // Struct layouts we rely on (same in C and Rust, repr(C)).
    assert_eq!(std::mem::size_of::<parse_buffer>(), 32);
    assert_eq!(std::mem::size_of::<cJSON>(), 16);
    assert_eq!(std::mem::align_of::<parse_buffer>(), 8);
    assert_eq!(std::mem::align_of::<cJSON>(), 8);

    let texts: [&[u8]; 10] = [
        b"1\0", b"12\0", b"123456\0", b"-1.5e2\0", b"0\0", b"2147483648\0", b"-2147483649\0",
        b"1e999\0", b"1.2.3\0", b"7 ",
    ];
    // Byte offset of the cJSON inside the shared 64-byte arena, relative to the
    // parse_buffer which always sits at arena offset 0.
    let item_offsets: [usize; 5] = [0, 8, 16, 24, 32];

    for text in texts {
        for &io in &item_offsets {
            for offset in [0usize, 1] {
                if offset >= text.len() {
                    continue;
                }
                let c = run_aliased(c_parse_number(), text, io, offset);
                let r = run_aliased(rust_parse_number(), text, io, offset);
                assert_eq!(
                    c,
                    r,
                    "[H1] aliased out-params diverged: text={:?} item_offset={io} offset={offset}",
                    String::from_utf8_lossy(text)
                );
            }
        }
    }
}

/// Result of an aliased call: return value plus the whole arena, byte for byte.
fn run_aliased(f: ParseNumberFn, text: &[u8], item_off: usize, offset: usize) -> (c_int, Vec<u8>) {
    // 8-byte-aligned arena big enough for a parse_buffer at 0 and a cJSON at
    // item_off (max 32) — 64 bytes covers every case.
    #[repr(align(8))]
    struct Arena([u8; 64]);
    let mut arena = Arena([0u8; 64]);
    let mut data = text.to_vec();

    let base = arena.0.as_mut_ptr();
    let buf = base as *mut parse_buffer;
    let item = unsafe { base.add(item_off) } as *mut cJSON;

    unsafe {
        std::ptr::write(
            buf,
            parse_buffer {
                content: data.as_mut_ptr(),
                length: text.len(),
                offset,
                depth: 0xDEAD_BEEF,
            },
        );
        // Seed the cJSON AFTER the buffer, so the overlap direction is
        // deterministic and identical for both implementations.
        if item_off >= 32 {
            std::ptr::write(
                item,
                cJSON {
                    type_: POISON_TYPE,
                    valueint: POISON_VALUEINT,
                    valuedouble: f64::from_bits(POISON_DOUBLE_BITS),
                },
            );
        }
        let ret = f(item, buf);
        // Report the arena verbatim; `content` is a per-call heap address, so
        // blank out the pointer field before comparing.
        let mut snapshot = arena.0.to_vec();
        for b in &mut snapshot[0..8] {
            *b = 0;
        }
        (ret, snapshot)
    }
}

/// `content` is non-NULL but points nowhere readable, while the bound check
/// guarantees a zero-length scan (`offset >= length`). The C computes
/// `content + offset` and `memcpy(dst, that, 0)` without dereferencing it, so
/// it must return `false` rather than crash — and so must the Rust.
#[test]
fn h2_bogus_nonnull_content_with_zero_length_scan() {
    let bogus: [*const c_uchar; 6] = [
        1usize as *const c_uchar,
        0xDEADusize as *const c_uchar,
        usize::MAX as *const c_uchar,
        (usize::MAX - 7) as *const c_uchar,
        0x1000usize as *const c_uchar,
        (1usize << 47) as *const c_uchar,
    ];
    for &content in &bogus {
        for (length, offset) in [
            (0usize, 0usize),
            (0, 1),
            (0, usize::MAX),
            (1, 1),
            (5, 5),
            (5, 9),
            (usize::MAX, usize::MAX),
        ] {
            if offset < length {
                continue; // would actually dereference — not a safe comparison
            }
            let mk = || parse_buffer {
                content,
                length,
                offset,
                depth: 42,
            };
            let mk_item = || cJSON {
                type_: POISON_TYPE,
                valueint: POISON_VALUEINT,
                valuedouble: f64::from_bits(POISON_DOUBLE_BITS),
            };
            let (mut cb, mut ci) = (mk(), mk_item());
            let (mut rb, mut ri) = (mk(), mk_item());
            let cr = unsafe { c_parse_number()(&mut ci, &mut cb) };
            let rr = unsafe { rust_parse_number()(&mut ri, &mut rb) };
            assert_eq!(
                (
                    cr,
                    ci.type_,
                    ci.valueint,
                    ci.valuedouble.to_bits(),
                    cb.offset,
                    cb.length,
                    cb.depth
                ),
                (
                    rr,
                    ri.type_,
                    ri.valueint,
                    ri.valuedouble.to_bits(),
                    rb.offset,
                    rb.length,
                    rb.depth
                ),
                "[H2] content={content:?} length={length} offset={offset}"
            );
            assert_eq!(cr, 0, "[H2] must reject without dereferencing");
        }
    }
}

/// Misaligned `cJSON` / `parse_buffer` pointers. A C caller can produce these
/// (e.g. structs inside a packed buffer); x86-64 tolerates them. Both
/// implementations must behave identically.
#[test]
fn h3_misaligned_struct_pointers() {
    #[repr(align(8))]
    struct Arena([u8; 128]);

    let texts: [&[u8]; 6] = [b"1\0", b"-2.5e1\0", b"2147483648\0", b"1e999\0", b".\0", b"x\0"];

    for text in texts {
        for skew in 1usize..8 {
            let mut out = Vec::new();
            for f in [c_parse_number(), rust_parse_number()] {
                let mut arena = Arena([0u8; 128]);
                let mut data = text.to_vec();
                let base = unsafe { arena.0.as_mut_ptr().add(skew) };
                let buf = base as *mut parse_buffer;
                let item = unsafe { base.add(32) } as *mut cJSON;
                unsafe {
                    std::ptr::write_unaligned(
                        buf,
                        parse_buffer {
                            content: data.as_mut_ptr(),
                            length: text.len(),
                            offset: 0,
                            depth: 9,
                        },
                    );
                    std::ptr::write_unaligned(
                        item,
                        cJSON {
                            type_: POISON_TYPE,
                            valueint: POISON_VALUEINT,
                            valuedouble: f64::from_bits(POISON_DOUBLE_BITS),
                        },
                    );
                    let ret = f(item, buf);
                    let b = std::ptr::read_unaligned(buf);
                    let i = std::ptr::read_unaligned(item);
                    out.push((
                        ret,
                        b.length,
                        b.offset,
                        b.depth,
                        i.type_,
                        i.valueint,
                        i.valuedouble.to_bits(),
                    ));
                }
            }
            assert_eq!(
                out[0],
                out[1],
                "[H3] misaligned (skew={skew}) diverged for {:?}",
                String::from_utf8_lossy(text)
            );
        }
    }
}

/// Repeated calls with the SAME `item` reused across success and failure,
/// verifying that a failure never partially overwrites a previous success.
#[test]
fn h4_item_reuse_across_success_and_failure() {
    let seq = ["123", "x", "4.5", "", "-6e2", ".", "1e999", "-", "0"];
    let cf = c_parse_number();
    let rf = rust_parse_number();
    let mut c_item = cJSON {
        type_: POISON_TYPE,
        valueint: POISON_VALUEINT,
        valuedouble: f64::from_bits(POISON_DOUBLE_BITS),
    };
    let mut r_item = c_item;
    for s in seq {
        let mut cd = {
            let mut v = s.as_bytes().to_vec();
            v.push(0);
            v
        };
        let mut rd = cd.clone();
        let mut cb = parse_buffer {
            content: cd.as_mut_ptr(),
            length: cd.len(),
            offset: 0,
            depth: 0,
        };
        let mut rb = parse_buffer {
            content: rd.as_mut_ptr(),
            length: rd.len(),
            offset: 0,
            depth: 0,
        };
        let cr = unsafe { cf(&mut c_item, &mut cb) };
        let rr = unsafe { rf(&mut r_item, &mut rb) };
        assert_eq!(
            (cr, c_item.type_, c_item.valueint, c_item.valuedouble.to_bits(), cb.offset),
            (rr, r_item.type_, r_item.valueint, r_item.valuedouble.to_bits(), rb.offset),
            "[H4] divergence after {s:?}"
        );
    }
}

/// C29 — locale surface. `strtod(3)` is locale-sensitive, and the C source's
/// `decimal_point` variable exists precisely because of that (it is hard-coded
/// to `'.'`, so the "localise" loop is a no-op — a quirk that is preserved, not
/// fixed). Under a comma-decimal locale such as `de_DE.utf8`, `strtod` stops at
/// `'.'`, so `"1.5"` parses as `1` and only one byte is consumed. Both
/// implementations must agree in every locale.
///
/// `setlocale` is process-global, so each locale is exercised in a forked child
/// that returns 0 only if every comparison in that locale matched.
#[test]
fn h5_locale_dependent_strtod() {
    let locales: [&[u8]; 7] = [
        b"C\0",
        b"POSIX\0",
        b"C.utf8\0",
        b"de_DE.utf8\0",
        b"de_DE\0",
        b"fr_FR.utf8\0",
        b"ru_RU.utf8\0",
    ];
    for loc in locales {
        let name = String::from_utf8_lossy(&loc[..loc.len() - 1]).to_string();
        let status = fork_locale_child(loc);
        let exited = (status & 0x7f) == 0;
        let code = (status >> 8) & 0xff;
        assert!(
            exited,
            "[H5] child for locale {name} died on a signal (status {status:#x})"
        );
        assert!(
            code == 0 || code == 77,
            "[H5] divergence in locale {name} (child exit code {code})"
        );
        if code == 77 {
            eprintln!("[H5] locale {name} unavailable on this host — skipped");
        }
    }
}

fn fork_locale_child(loc: &[u8]) -> i32 {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let ok = libc::setlocale(libc::LC_NUMERIC, loc.as_ptr() as *const libc::c_char);
            if ok.is_null() {
                libc::_exit(77); // locale not installed
            }
            let cf = c_parse_number();
            let rf = rust_parse_number();
            let cases: [&str; 24] = [
                "1.5", "1,5", "-1.5", "-1,5", "0.0", "0,0", "1.5e2", "1,5e2", "12.", "12,",
                ".5", ",5", "3.14159", "3,14159", "2147483647.5", "2147483647,5",
                "-2147483648.5", "-2147483648,5", "1e999", "1.7976931348623157e308",
                "1,7976931348623157e308", "0.1", "0,1", "1.2.3",
            ];
            let mut bad = 0;
            for s in cases {
                for extra in [true, false] {
                    let mut base = s.as_bytes().to_vec();
                    if extra {
                        base.push(0);
                    } else {
                        base.extend_from_slice(b" 99");
                    }
                    let len = if extra { base.len() } else { s.len() };
                    let mut cd = base.clone();
                    let mut rd = base.clone();
                    let mut cb = parse_buffer {
                        content: cd.as_mut_ptr(),
                        length: len,
                        offset: 0,
                        depth: 0,
                    };
                    let mut rb = parse_buffer {
                        content: rd.as_mut_ptr(),
                        length: len,
                        offset: 0,
                        depth: 0,
                    };
                    let mut ci = cJSON {
                        type_: POISON_TYPE,
                        valueint: POISON_VALUEINT,
                        valuedouble: f64::from_bits(POISON_DOUBLE_BITS),
                    };
                    let mut ri = ci;
                    let cr = cf(&mut ci, &mut cb);
                    let rr = rf(&mut ri, &mut rb);
                    if (
                        cr,
                        ci.type_,
                        ci.valueint,
                        ci.valuedouble.to_bits(),
                        cb.offset,
                    ) != (
                        rr,
                        ri.type_,
                        ri.valueint,
                        ri.valuedouble.to_bits(),
                        rb.offset,
                    ) {
                        bad += 1;
                    }
                    if cd != base || rd != base {
                        bad += 1; // input buffer was mutated
                    }
                }
            }
            libc::_exit(if bad == 0 { 0 } else { 1 });
        }
        let mut status: c_int = 0;
        libc::waitpid(pid, &mut status, 0);
        status
    }
}

/// Keep `c_uchar` referenced (used in the H2 pointer casts).
const _: () = {
    let _ = std::mem::size_of::<c_uchar>();
};
