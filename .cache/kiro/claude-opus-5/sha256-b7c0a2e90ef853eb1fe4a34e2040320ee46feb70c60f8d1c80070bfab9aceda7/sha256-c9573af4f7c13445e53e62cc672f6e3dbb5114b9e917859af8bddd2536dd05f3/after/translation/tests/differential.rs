//! C-vs-Rust differential verification for `libdriver`.
//!
//! Custom harness (`harness = false`) because every measurement must happen in a
//! dedicated child process: the library talks on `stdout`, and `bad()` with a
//! large index deliberately corrupts its own stack frame, which no in-process
//! test harness can survive.
//!
//! * Phase B — `CONFIGS.md` rows 1..=33 (valid / defined inputs)
//! * Phase C — `ERRORS.md` rows 1..=12 and G1..=G8 (rejection & boundary paths)
//! * Phase D — symbol parity between the two `.so` files
//!
//! Run with `cargo test --release` (or `cargo test`); a non-zero exit status
//! means at least one row diverged.

mod common;

use common::*;
use std::process::Command;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

fn main() {
    // Child mode: `--child <lib> <op…>` — performs the FFI calls and exits.
    maybe_run_as_child();

    println!("C    lib: {}", c_lib().display());
    println!("Rust lib: {}", rust_lib().display());

    let mut ok = true;
    ok &= phase_d_symbols();
    ok &= phase_b_configs();
    ok &= phase_c_errors();

    println!("\n########################################");
    if ok {
        println!("ALL PHASES PASSED");
    } else {
        println!("FAILURES PRESENT — see above");
    }
    println!("########################################");
    if !ok {
        std::process::exit(1);
    }
}

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================

fn defined_dynamic_symbols(lib: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(lib)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", lib.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let (kind, name) = match (it.next(), it.next()) {
                // "<addr> T name"
                (Some(k), Some(n)) => (k, n),
                // "         w name"
                (Some(n), None) => (a, n),
                _ => return None,
            };
            // Weak toolchain hooks (`w`) are runtime glue, not API surface.
            if kind == "w" || kind == "W" || kind == "V" || kind == "v" {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn phase_d_symbols() -> bool {
    let mut rep = Report::new();
    let c = defined_dynamic_symbols(&c_lib());
    let r = defined_dynamic_symbols(&rust_lib());
    rep.note(format!("C   exports: {c:?}"));
    rep.note(format!("Rust exports: {r:?}"));

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    rep.check(
        "SYMBOLS.md: every C symbol exported by Rust",
        missing.is_empty(),
        &format!("missing from Rust .so: {missing:?}"),
    );

    // Each of the five must additionally be *resolvable* through dlsym in both
    // libraries — the child processes do exactly that on every call, but assert
    // it explicitly here so a missing export is reported as a symbol failure.
    for name in ["printLine", "printIntLine", "bad", "good", "driver"] {
        rep.check(
            &format!("SYMBOLS.md: `{name}` present in both .so"),
            c.iter().any(|s| s == name) && r.iter().any(|s| s == name),
            &format!("`{name}` not defined in both libraries"),
        );
    }

    // `goodG2B` / `goodB2G` are `static` in C: neither library may export them.
    for name in ["goodG2B", "goodB2G"] {
        rep.check(
            &format!("SYMBOLS.md: static `{name}` not exported"),
            !c.iter().any(|s| s == name) && !r.iter().any(|s| s == name),
            &format!("`{name}` unexpectedly exported"),
        );
    }

    // No undefined non-libc symbols in the Rust .so.
    let ldd = Command::new("ldd").arg("-r").arg(rust_lib()).output();
    if let Ok(o) = ldd {
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol"))
            .collect();
        rep.check(
            "SYMBOLS.md: 0 undefined symbols in Rust .so (ldd -r)",
            bad.is_empty(),
            &format!("unresolved: {bad:?}"),
        );
    }

    rep.finish("PHASE D — symbol parity")
}

// ===========================================================================
// Phase B — valid / defined-path differential tests (CONFIGS.md)
// ===========================================================================

fn phase_b_configs() -> bool {
    let mut rep = Report::new();
    let mut rng = Rng::new(SEED);

    // ---- rows 1..=7 : printLine string shapes ---------------------------
    rep.diff("CONFIGS row 1  printLine empty string", &[Op::PrintLine(vec![])]);

    rep.diff(
        "CONFIGS row 2  printLine single ASCII byte",
        &(0x20u8..=0x7e)
            .map(|b| Op::PrintLine(vec![b]))
            .collect::<Vec<_>>(),
    );

    let mut ops = Vec::new();
    for _ in 0..120 {
        let len = rng.in_range(1, 256) as usize;
        let s: Vec<u8> = (0..len).map(|_| 0x20 + (rng.byte() % 0x5f)).collect();
        ops.push(Op::PrintLine(s));
    }
    rep.diff("CONFIGS row 3  printLine random printable ASCII", &ops);

    let mut ops = Vec::new();
    for _ in 0..120 {
        let len = rng.in_range(1, 200) as usize;
        // Full 0x01..=0xFF range: high bytes, control bytes, invalid UTF-8.
        // 0x00 is excluded because it terminates a C string by definition.
        let s: Vec<u8> = (0..len)
            .map(|_| {
                let b = rng.byte();
                if b == 0 { 1 } else { b }
            })
            .collect();
        ops.push(Op::PrintLine(s));
    }
    rep.diff("CONFIGS row 4  printLine random full-byte-range (non-UTF-8)", &ops);

    rep.diff(
        "CONFIGS row 5  printLine printf specifiers are not interpreted",
        &[
            Op::PrintLine(b"%s".to_vec()),
            Op::PrintLine(b"%d".to_vec()),
            Op::PrintLine(b"%n".to_vec()),
            Op::PrintLine(b"%%".to_vec()),
            Op::PrintLine(b"%s%s%s%s%s%s%s%s".to_vec()),
            Op::PrintLine(b"%1000000d".to_vec()),
            Op::PrintLine(b"100%% done: %p %x %-20s".to_vec()),
        ],
    );

    rep.diff(
        "CONFIGS row 6  printLine 64 KiB buffer",
        &[
            Op::PrintLine(vec![b'A'; 65536]),
            Op::PrintLine(vec![b'z'; 4095]),
            Op::PrintLine(vec![b'q'; 4096]),
            Op::PrintLine(vec![b'w'; 4097]),
        ],
    );

    rep.diff(
        "CONFIGS row 7  printLine whitespace-only strings",
        &[
            Op::PrintLine(b"\n".to_vec()),
            Op::PrintLine(b"\n\n\n".to_vec()),
            Op::PrintLine(b" ".to_vec()),
            Op::PrintLine(b"\t\t".to_vec()),
            Op::PrintLine(b"\r\n".to_vec()),
            Op::PrintLine(b"  \t \r\n \x0b\x0c ".to_vec()),
        ],
    );

    // ---- rows 8..=12 : printIntLine ------------------------------------
    rep.diff("CONFIGS row 8  printIntLine(0)", &[Op::PrintIntLine(0)]);
    rep.diff(
        "CONFIGS row 9  printIntLine small positives 1..=9",
        &(1..=9).map(Op::PrintIntLine).collect::<Vec<_>>(),
    );
    rep.diff(
        "CONFIGS row 10 printIntLine small negatives -1..=-9",
        &(-9..=-1).map(Op::PrintIntLine).collect::<Vec<_>>(),
    );
    let mut ops = Vec::new();
    for _ in 0..200 {
        ops.push(Op::PrintIntLine(rng.next_i32()));
    }
    rep.diff("CONFIGS row 11 printIntLine random full-range i32", &ops);
    rep.diff(
        "CONFIGS row 12 printIntLine extremes",
        &[
            Op::PrintIntLine(i32::MIN),
            Op::PrintIntLine(i32::MIN + 1),
            Op::PrintIntLine(i32::MAX),
            Op::PrintIntLine(i32::MAX - 1),
            Op::PrintIntLine(-1),
        ],
    );

    // ---- rows 13..=19 : bad() -------------------------------------------
    rep.diff("CONFIGS row 13 bad(0) first element", &[Op::Bad(0)]);
    rep.diff(
        "CONFIGS row 14 bad(1..=8) middle elements",
        &(1..=8).map(Op::Bad).collect::<Vec<_>>(),
    );
    rep.diff("CONFIGS row 15 bad(9) last element", &[Op::Bad(9)]);
    let mut ops = Vec::new();
    for _ in 0..100 {
        ops.push(Op::Bad(rng.in_range(0, 9)));
    }
    rep.diff("CONFIGS row 16 bad() random in-bounds index", &ops);
    let mut ops = Vec::new();
    for _ in 0..100 {
        ops.push(Op::Bad(rng.in_range(i32::MIN, -1)));
    }
    rep.diff("CONFIGS row 17 bad() random negative index", &ops);
    // 10 and 11 overrun the array but land in frame slack / the dead loop
    // counter, so the C behaviour is still deterministic. Run each on its own so
    // a crash in one cannot mask the other.
    rep.diff("CONFIGS row 18 bad(10) absorbed overrun", &[Op::Bad(10)]);
    rep.diff("CONFIGS row 18 bad(11) absorbed overrun", &[Op::Bad(11)]);

    // Row 19: >= 12 is genuine undefined behaviour in C (clobbers the saved
    // frame pointer / return address). Recorded, not equality-asserted; see
    // ERRORS.md row 9. What we *do* assert is that Rust stays well-behaved.
    for data in [12, 13, 14, 20, 100, 1000] {
        let c = run_one(&c_lib(), &Op::Bad(data));
        let r = run_one(&rust_lib(), &Op::Bad(data));
        rep.note(format!(
            "row 19 bad({data}) [UB in C, not asserted]\n        C   : {}\n        Rust: {}",
            c.describe(),
            r.describe()
        ));
        if data < 1024 {
            rep.check(
                &format!("CONFIGS row 19 bad({data}): Rust absorbs the overrun cleanly"),
                r.ok() && r.stdout == b"0\n".repeat(10),
                &format!("Rust: {}", r.describe()),
            );
        }
        if c == r {
            rep.note(format!("row 19 bad({data}): C and Rust happened to agree"));
        }
    }

    // ---- rows 20..=23 : good() ------------------------------------------
    rep.diff(
        "CONFIGS row 20 good(0..=9) in bounds",
        &(0..=9).map(Op::Good).collect::<Vec<_>>(),
    );
    let mut ops = Vec::new();
    for _ in 0..100 {
        ops.push(Op::Good(rng.in_range(0, 9)));
    }
    rep.diff("CONFIGS row 21 good() random in-bounds", &ops);
    let mut ops = vec![Op::Good(10), Op::Good(11), Op::Good(12), Op::Good(i32::MAX)];
    for _ in 0..100 {
        ops.push(Op::Good(rng.in_range(10, i32::MAX)));
    }
    rep.diff("CONFIGS row 22 good() at/above the upper bound", &ops);
    let mut ops = vec![Op::Good(-1), Op::Good(i32::MIN)];
    for _ in 0..100 {
        ops.push(Op::Good(rng.in_range(i32::MIN, -1)));
    }
    rep.diff("CONFIGS row 23 good() negative", &ops);

    // ---- rows 24..=31 : driver() ----------------------------------------
    let mut ops = Vec::new();
    for g in 0..=9 {
        for b in 0..=9 {
            ops.push(Op::Driver(g, b));
        }
    }
    rep.diff("CONFIGS row 24 driver 10x10 in-bounds cross-product", &ops);

    let mut ops = Vec::new();
    for g in 0..=9 {
        ops.push(Op::Driver(g, -1 - g));
    }
    rep.diff("CONFIGS row 25 driver good in-bounds x bad negative", &ops);

    let mut ops = Vec::new();
    for b in 0..=9 {
        ops.push(Op::Driver(10 + b, b));
    }
    ops.push(Op::Driver(i32::MAX, 0));
    rep.diff("CONFIGS row 26 driver good >=10 x bad in-bounds", &ops);

    let mut ops = Vec::new();
    for b in 0..=9 {
        ops.push(Op::Driver(-1 - b, b));
    }
    ops.push(Op::Driver(i32::MIN, 9));
    rep.diff("CONFIGS row 27 driver good negative x bad in-bounds", &ops);

    rep.diff(
        "CONFIGS row 28 driver both invalid (negative x negative)",
        &[
            Op::Driver(-1, -1),
            Op::Driver(i32::MIN, -1),
            Op::Driver(-1, i32::MIN),
            Op::Driver(i32::MIN, i32::MIN),
        ],
    );

    rep.diff(
        "CONFIGS row 29 driver good >=10 x bad negative",
        &[
            Op::Driver(10, -1),
            Op::Driver(11, -2),
            Op::Driver(i32::MAX, i32::MIN),
        ],
    );

    let goods = [i32::MIN, -1, 0, 9, 10, i32::MAX];
    let bads = [i32::MIN, -1, 0, 9, 10, 11];
    let mut ops = Vec::new();
    for g in goods {
        for b in bads {
            ops.push(Op::Driver(g, b));
        }
    }
    rep.diff("CONFIGS row 30 driver boundary cross-product (6x6)", &ops);

    let mut ops = Vec::new();
    for _ in 0..200 {
        // badData is kept inside the deterministic domain INT_MIN..=11; beyond
        // that the C code's behaviour is undefined (ERRORS.md row 9).
        ops.push(Op::Driver(rng.next_i32(), rng.in_range(i32::MIN, 11)));
    }
    rep.diff("CONFIGS row 31 driver random seeded pairs", &ops);

    // ---- row 32 : interleaved composed pipeline in one process ----------
    let mut ops = vec![
        Op::PrintLine(b"--- pipeline start ---".to_vec()),
        Op::PrintIntLine(-7),
        Op::PrintLineNull,
        Op::Bad(3),
        Op::Good(4),
        Op::Driver(5, 6),
        Op::PrintLine(vec![]),
        Op::Bad(-5),
        Op::Good(10),
        Op::Driver(-1, 11),
        Op::PrintIntLine(i32::MIN),
        Op::PrintLine(b"--- pipeline end ---".to_vec()),
    ];
    for _ in 0..60 {
        match rng.next_u64() % 5 {
            0 => ops.push(Op::PrintLine(vec![b'x'; (rng.byte() % 40 + 1) as usize])),
            1 => ops.push(Op::PrintIntLine(rng.next_i32())),
            2 => ops.push(Op::Bad(rng.in_range(-20, 11))),
            3 => ops.push(Op::Good(rng.next_i32())),
            _ => ops.push(Op::Driver(rng.next_i32(), rng.in_range(-20, 11))),
        }
    }
    rep.diff("CONFIGS row 32 interleaved calls, single shared stdout", &ops);

    // ---- row 33 : goodG2B's else-branch is dead code --------------------
    let mut all_good_output_clean = true;
    let mut probes: Vec<i32> = vec![i32::MIN, -1, 0, 7, 9, 10, i32::MAX];
    for _ in 0..40 {
        probes.push(rng.next_i32());
    }
    for d in probes {
        let out = run_one(&rust_lib(), &Op::Good(d));
        let c_out = run_one(&c_lib(), &Op::Good(d));
        let needle: &[u8] = b"ERROR: Array index is negative.";
        let contains = |h: &[u8]| h.windows(needle.len()).any(|w| w == needle);
        if contains(&out.stdout) || contains(&c_out.stdout) {
            all_good_output_clean = false;
        }
    }
    rep.check(
        "CONFIGS row 33 goodG2B else-branch is unreachable (data == 7 hardcoded)",
        all_good_output_clean,
        "good() emitted the \"negative\" diagnostic, so the branch is reachable",
    );

    rep.finish("PHASE B — valid-path differential (CONFIGS.md)")
}

// ===========================================================================
// Phase C — error / rejection-path differential tests (ERRORS.md)
// ===========================================================================

fn phase_c_errors() -> bool {
    let mut rep = Report::new();
    let mut rng = Rng::new(SEED ^ 0xDEAD_BEEF);

    // ERRORS row 1 / G1 — printLine(NULL) must be silent in both.
    let c = run_one(&c_lib(), &Op::PrintLineNull);
    let r = run_one(&rust_lib(), &Op::PrintLineNull);
    rep.check(
        "ERRORS row 1  printLine(NULL) identical",
        c == r,
        &format!("C: {} / Rust: {}", c.describe(), r.describe()),
    );
    rep.check(
        "ERRORS row 1 / G1  printLine(NULL) emits zero bytes",
        c.stdout.is_empty() && r.stdout.is_empty() && c.ok() && r.ok(),
        &format!("C: {} / Rust: {}", c.describe(), r.describe()),
    );
    // NULL guard must not be defeated by surrounding traffic either.
    rep.diff(
        "ERRORS row 1  printLine(NULL) interleaved with valid calls",
        &[
            Op::PrintLine(b"before".to_vec()),
            Op::PrintLineNull,
            Op::PrintLineNull,
            Op::PrintLine(b"after".to_vec()),
        ],
    );

    // ERRORS row 2 — printIntLine has no validation: every int is accepted.
    let mut ops = vec![
        Op::PrintIntLine(i32::MIN),
        Op::PrintIntLine(i32::MAX),
        Op::PrintIntLine(0),
    ];
    for _ in 0..80 {
        ops.push(Op::PrintIntLine(rng.next_i32()));
    }
    rep.diff("ERRORS row 2  printIntLine accepts the whole i32 range", &ops);

    // ERRORS row 3 — bad(data < 0) prints the "negative" diagnostic.
    let mut ops = vec![Op::Bad(-1), Op::Bad(-2), Op::Bad(-10), Op::Bad(-1000000)];
    for _ in 0..80 {
        ops.push(Op::Bad(rng.in_range(i32::MIN, -1)));
    }
    rep.diff("ERRORS row 3  bad(negative) rejection", &ops);
    let neg = run_one(&rust_lib(), &Op::Bad(-1));
    let neg_c = run_one(&c_lib(), &Op::Bad(-1));
    rep.check(
        "ERRORS row 3  bad(-1) exact diagnostic text",
        neg.stdout == b"ERROR: Array index is negative.\n"
            && neg_c.stdout == b"ERROR: Array index is negative.\n",
        &format!("C: {} / Rust: {}", neg_c.describe(), neg.describe()),
    );

    // ERRORS row 4 — bad(INT_MIN) (no overflow/abs trap on the extreme).
    rep.diff(
        "ERRORS row 4  bad(INT_MIN) / bad(INT_MIN+1)",
        &[Op::Bad(i32::MIN), Op::Bad(i32::MIN + 1)],
    );

    // ERRORS row 5 — good() -> goodB2G(data < 0).
    let mut ops = vec![Op::Good(-1), Op::Good(-2), Op::Good(-9999)];
    for _ in 0..80 {
        ops.push(Op::Good(rng.in_range(i32::MIN, -1)));
    }
    rep.diff("ERRORS row 5  good(negative) rejection", &ops);

    // ERRORS row 6 — good(INT_MIN).
    rep.diff(
        "ERRORS row 6  good(INT_MIN) / good(INT_MIN+1)",
        &[Op::Good(i32::MIN), Op::Good(i32::MIN + 1)],
    );

    // ERRORS row 7 — good() -> goodB2G(data >= 10).
    let mut ops = vec![
        Op::Good(10),
        Op::Good(11),
        Op::Good(100),
        Op::Good(i32::MAX),
        Op::Good(i32::MAX - 1),
    ];
    for _ in 0..80 {
        ops.push(Op::Good(rng.in_range(10, i32::MAX)));
    }
    rep.diff("ERRORS row 7  good(>= 10) rejection", &ops);
    let g = run_one(&rust_lib(), &Op::Good(10));
    let gc = run_one(&c_lib(), &Op::Good(10));
    let expect_oob = b"ERROR: Array index is out-of-bounds\n";
    rep.check(
        "ERRORS row 7  good(10) exact diagnostic text (no trailing period)",
        g.stdout.ends_with(expect_oob) && gc.stdout.ends_with(expect_oob),
        &format!("C: {} / Rust: {}", gc.describe(), g.describe()),
    );

    // ERRORS row 8 — bad(10) / bad(11): the missing upper-bound check does NOT
    // reject; both must print ten zeroes.
    for data in [10, 11] {
        let c = run_one(&c_lib(), &Op::Bad(data));
        let r = run_one(&rust_lib(), &Op::Bad(data));
        rep.check(
            &format!("ERRORS row 8  bad({data}) identical (overrun absorbed, not rejected)"),
            c == r,
            &format!("C: {} / Rust: {}", c.describe(), r.describe()),
        );
        rep.check(
            &format!("ERRORS row 8  bad({data}) prints ten zeroes and no diagnostic"),
            c.stdout == b"0\n".repeat(10) && r.stdout == b"0\n".repeat(10),
            &format!("C: {} / Rust: {}", c.describe(), r.describe()),
        );
    }

    // ERRORS row 9 — bad(>= 12): undefined behaviour in C. Recorded.
    for data in [12, 13, 14, 15, 32, 64, 500] {
        let c = run_one(&c_lib(), &Op::Bad(data));
        let r = run_one(&rust_lib(), &Op::Bad(data));
        rep.note(format!(
            "row 9 bad({data}) [C is UB — informational]\n        C   : {}\n        Rust: {}",
            c.describe(),
            r.describe()
        ));
        rep.check(
            &format!("ERRORS row 9  bad({data}): Rust neither crashes nor prints a diagnostic"),
            r.ok() && r.stdout == b"0\n".repeat(10),
            &format!("Rust: {}", r.describe()),
        );
    }

    // ERRORS row 10 — driver(_, badData < 0).
    let mut ops = vec![Op::Driver(0, -1), Op::Driver(7, i32::MIN)];
    for _ in 0..60 {
        ops.push(Op::Driver(rng.in_range(0, 9), rng.in_range(i32::MIN, -1)));
    }
    rep.diff("ERRORS row 10 driver with negative badData", &ops);

    // ERRORS row 11 — driver(goodData invalid, _).
    let mut ops = vec![
        Op::Driver(-1, 0),
        Op::Driver(10, 0),
        Op::Driver(i32::MIN, 5),
        Op::Driver(i32::MAX, 5),
    ];
    for _ in 0..60 {
        let g = if rng.next_u64() % 2 == 0 {
            rng.in_range(i32::MIN, -1)
        } else {
            rng.in_range(10, i32::MAX)
        };
        ops.push(Op::Driver(g, rng.in_range(0, 9)));
    }
    rep.diff("ERRORS row 11 driver with out-of-range goodData", &ops);

    // ERRORS row 12 — goodG2B's else branch is dead code.
    let needle: &[u8] = b"ERROR: Array index is negative.";
    let contains = |h: &[u8]| h.windows(needle.len()).any(|w| w == needle);
    let mut clean = true;
    let mut probes = vec![i32::MIN, -1, 0, 7, 9, 10, i32::MAX];
    for _ in 0..30 {
        probes.push(rng.next_i32());
    }
    for d in probes {
        if contains(&run_one(&c_lib(), &Op::Good(d)).stdout)
            || contains(&run_one(&rust_lib(), &Op::Good(d)).stdout)
        {
            clean = false;
        }
    }
    rep.check(
        "ERRORS row 12 goodG2B negative-diagnostic branch is unreachable",
        clean,
        "good() produced the goodG2B \"negative\" diagnostic",
    );

    // ---- generic FFI boundary cases G2..=G8 -----------------------------
    rep.diff("ERRORS G2  printLine(\"\") zero-length", &[Op::PrintLine(vec![])]);
    rep.diff(
        "ERRORS G3  printLine oversized payloads",
        &[
            Op::PrintLine(vec![b'A'; 65535]),
            Op::PrintLine(vec![b'B'; 65536]),
            Op::PrintLine(vec![b'C'; 100_000]),
        ],
    );
    rep.diff(
        "ERRORS G4  printLine format-specifier payloads",
        &[
            Op::PrintLine(b"%n%n%n".to_vec()),
            Op::PrintLine(b"%s".to_vec()),
            Op::PrintLine(b"%99999999d".to_vec()),
            Op::PrintLine(b"%".to_vec()),
        ],
    );
    let mut ops = Vec::new();
    for _ in 0..60 {
        let len = rng.in_range(1, 64) as usize;
        ops.push(Op::PrintLine(
            (0..len).map(|_| 0x80 | (rng.byte() & 0x7f)).collect(),
        ));
    }
    rep.diff("ERRORS G5  printLine high-bit / non-UTF-8 bytes", &ops);
    rep.diff(
        "ERRORS G6  printIntLine sign extremes",
        &[
            Op::PrintIntLine(i32::MIN),
            Op::PrintIntLine(i32::MAX),
            Op::PrintIntLine(0),
            Op::PrintIntLine(-0),
        ],
    );
    // G7: there is no enum in this API — every parameter is a plain `int`, so
    // the "out-of-range enum value" class is covered by feeding values with no
    // meaningful interpretation across the whole 32-bit range to each entry
    // point that takes an int.
    let wild = [
        i32::MIN,
        i32::MIN + 1,
        -2_147_000_000,
        -65536,
        -256,
        -2,
        -1,
        0,
        1,
        9,
        10,
        11,
        255,
        256,
        65535,
        65536,
        1_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    rep.diff(
        "ERRORS G7  good() with arbitrary out-of-domain int values",
        &wild.iter().copied().map(Op::Good).collect::<Vec<_>>(),
    );
    rep.diff(
        "ERRORS G7  printIntLine with arbitrary out-of-domain int values",
        &wild.iter().copied().map(Op::PrintIntLine).collect::<Vec<_>>(),
    );
    rep.diff(
        "ERRORS G7  driver() with arbitrary out-of-domain goodData",
        &wild
            .iter()
            .copied()
            .map(|g| Op::Driver(g, 0))
            .collect::<Vec<_>>(),
    );
    rep.diff(
        "ERRORS G7  bad() with arbitrary out-of-domain int values (deterministic subset)",
        &wild
            .iter()
            .copied()
            .filter(|&v| v <= 11)
            .map(Op::Bad)
            .collect::<Vec<_>>(),
    );
    // G8: one step past every documented boundary.
    rep.diff(
        "ERRORS G8  bad() boundary steps -1/0/9/10",
        &[Op::Bad(-1), Op::Bad(0), Op::Bad(9), Op::Bad(10)],
    );
    rep.diff(
        "ERRORS G8  good() boundary steps -1/0/9/10",
        &[Op::Good(-1), Op::Good(0), Op::Good(9), Op::Good(10)],
    );
    rep.diff(
        "ERRORS G8  driver() boundary steps on both parameters",
        &[
            Op::Driver(-1, -1),
            Op::Driver(0, 0),
            Op::Driver(9, 9),
            Op::Driver(10, 10),
            Op::Driver(9, 10),
            Op::Driver(10, 9),
        ],
    );

    rep.finish("PHASE C — error-path differential (ERRORS.md)")
}
