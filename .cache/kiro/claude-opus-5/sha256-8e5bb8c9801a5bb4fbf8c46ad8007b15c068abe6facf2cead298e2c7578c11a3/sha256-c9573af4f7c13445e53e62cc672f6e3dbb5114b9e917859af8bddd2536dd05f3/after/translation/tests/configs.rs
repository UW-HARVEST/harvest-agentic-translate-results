//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the bytes they write. Nothing calls into the Rust crate
//! directly.
//!
//! Randomized rows use `common::Rng` (SplitMix64) with a fixed seed per row, so
//! a failure is reproducible.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 — printIntPtrLine, stack-local int, randomized values
// ---------------------------------------------------------------------------
fn row01_pipl_stack_random() {
    let (c, r) = (load_c(), load_rust());
    let mut rng = Rng::new(0x5EED_1234);
    for _ in 0..4096 {
        assert_same_in_process(&c, &r, &format!("pipl:{}", rng.next_i32()));
    }
}

// ---------------------------------------------------------------------------
// Row 2 — printIntPtrLine, boundary value set
// ---------------------------------------------------------------------------
fn row02_pipl_stack_boundaries() {
    let (c, r) = (load_c(), load_rust());
    let vals = boundary_i32s();
    assert!(vals.len() > 100, "boundary set unexpectedly small");
    for v in vals {
        assert_same_in_process(&c, &r, &format!("pipl:{v}"));
    }
}

// ---------------------------------------------------------------------------
// Row 3 — printIntPtrLine, heap-allocated int
// ---------------------------------------------------------------------------
fn row03_pipl_heap_random() {
    let (c, r) = (load_c(), load_rust());
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..1024 {
        assert_same_in_process(&c, &r, &format!("pipl_heap:{}", rng.next_i32()));
    }
    for v in boundary_i32s() {
        assert_same_in_process(&c, &r, &format!("pipl_heap:{v}"));
    }
}

// ---------------------------------------------------------------------------
// Row 4 — printIntPtrLine, pointer into static storage
// ---------------------------------------------------------------------------
fn row04_pipl_static_random() {
    let (c, r) = (load_c(), load_rust());
    let mut rng = Rng::new(0x5EED_0004);
    for _ in 0..1024 {
        assert_same_in_process(&c, &r, &format!("pipl_static:{}", rng.next_i32()));
    }
    for v in boundary_i32s() {
        assert_same_in_process(&c, &r, &format!("pipl_static:{v}"));
    }
}

// ---------------------------------------------------------------------------
// Row 5 — printIntPtrLine, element 0 / middle / last of an array
// ---------------------------------------------------------------------------
fn row05_pipl_array_index() {
    let (c, r) = (load_c(), load_rust());
    let mut rng = Rng::new(0x5EED_0005);

    // Fixed shapes: first, middle and last element of several array sizes.
    for n in [1usize, 2, 3, 8, 64, 1000] {
        for i in [0usize, n / 2, n - 1] {
            assert_same_in_process(&c, &r, &format!("pipl_idx:{}:{n}:{i}", rng.next_i32()));
        }
    }
    // Randomized shape and index.
    for _ in 0..512 {
        let n = 1 + rng.below(256);
        let i = rng.below(n);
        assert_same_in_process(&c, &r, &format!("pipl_idx:{}:{n}:{i}", rng.next_i32()));
    }
}

// ---------------------------------------------------------------------------
// Row 6 — printIntPtrLine, misaligned but readable address
// ---------------------------------------------------------------------------
fn row06_pipl_unaligned() {
    let (c, r) = (load_c(), load_rust());
    let mut rng = Rng::new(0x5EED_0006);
    for _ in 0..512 {
        let word = rng.next_u64() as u32;
        for off in 0..4usize {
            assert_same_in_process(&c, &r, &format!("pipl_unaligned:{word}:{off}"));
        }
    }
    // Byte patterns that make the misalignment visible in the decimal output.
    for word in [0u32, u32::MAX, 0x00FF_00FF, 0xFF00_FF00, 0x8000_0000, 1] {
        for off in 0..4usize {
            assert_same_in_process(&c, &r, &format!("pipl_unaligned:{word}:{off}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 — printIntPtrLine called back to back in a tight loop
// ---------------------------------------------------------------------------
fn row07_pipl_repeated_loop() {
    let (c, r) = (load_c(), load_rust());
    let mut rng = Rng::new(0x5EED_0007);
    // 1024 calls inside ONE capture, so stdio buffering, line ordering and any
    // per-call state leakage are all part of the comparison.
    let vals: Vec<i32> = (0..1024).map(|_| rng.next_i32()).collect();

    let run = |api: &Api| -> Vec<u8> {
        capture(&mut || {
            for v in &vals {
                unsafe { (api.print_int_ptr_line)(v as *const i32) };
            }
        })
    };
    let out_c = run(&c);
    let out_r = run(&r);
    assert_eq!(out_c, out_r, "1024-call loop output diverged");

    // Sanity: the capture really did collect 1024 lines.
    assert_eq!(
        out_c.iter().filter(|&&b| b == b'\n').count(),
        1024,
        "expected 1024 lines from 1024 calls"
    );
    let expected: String = vals.iter().map(|v| format!("{v}\n")).collect();
    assert_eq!(
        out_c,
        expected.as_bytes(),
        "C output did not match printf(\"%d\\n\") semantics"
    );
}

// ---------------------------------------------------------------------------
// Row 8 — good(), the single fixed configuration
// ---------------------------------------------------------------------------
fn row08_good() {
    let (c, r) = (load_c(), load_rust());
    assert_same_in_process(&c, &r, "good");
    let (out_c, _) = run_in_process(&c, &r, "good");
    assert_eq!(out_c, b"5\n", "good() must print 5");
}

// ---------------------------------------------------------------------------
// Row 9 — good() repeated and interleaved with printIntPtrLine
// ---------------------------------------------------------------------------
fn row09_good_interleaved() {
    let (c, r) = (load_c(), load_rust());
    let mut rng = Rng::new(0x5EED_0009);

    // Build a randomized program: 0 => good(), 1 => printIntPtrLine(&v).
    let program: Vec<(u8, i32)> = (0..512)
        .map(|_| {
            let kind = (rng.next_u64() & 1) as u8;
            (kind, rng.next_i32())
        })
        .collect();

    let run = |api: &Api| -> Vec<u8> {
        capture(&mut || {
            for (kind, v) in &program {
                unsafe {
                    if *kind == 0 {
                        (api.good)();
                    } else {
                        (api.print_int_ptr_line)(v as *const i32);
                    }
                }
            }
        })
    };
    assert_eq!(run(&c), run(&r), "interleaved good/pipl output diverged");

    // Also the plain repetition case.
    let run_n = |api: &Api| -> Vec<u8> {
        capture(&mut || {
            for _ in 0..256 {
                unsafe { (api.good)() };
            }
        })
    };
    let out_c = run_n(&c);
    assert_eq!(out_c, run_n(&r), "repeated good() output diverged");
    assert_eq!(out_c, "5\n".repeat(256).as_bytes());
}

// ---------------------------------------------------------------------------
// Row 10 — driver(1)
// ---------------------------------------------------------------------------
fn row10_driver_one() {
    let (c, r) = (load_c(), load_rust());
    assert_same_in_process(&c, &r, "driver:1");
    let (out_c, _) = run_in_process(&c, &r, "driver:1");
    assert_eq!(out_c, b"5\n", "driver(1) must take the good arm");
}

// ---------------------------------------------------------------------------
// Row 11 — driver with randomized NON-ZERO useGood (C truthiness)
// ---------------------------------------------------------------------------
fn row11_driver_nonzero_random() {
    let (c, r) = (load_c(), load_rust());
    let mut rng = Rng::new(0x5EED_0011);

    let mut checked = 0usize;
    for _ in 0..4096 {
        let v = rng.next_i32();
        if v == 0 {
            continue; // zero is row 12's territory
        }
        assert_same_in_process(&c, &r, &format!("driver:{v}"));
        checked += 1;
    }
    assert!(checked > 4000, "too few non-zero samples: {checked}");

    // Every non-zero boundary value, plus single-bit and high-bit-only patterns.
    let mut vals: Vec<i32> = boundary_i32s().into_iter().filter(|&v| v != 0).collect();
    vals.extend([i32::MIN, i32::MAX, -1, 1, 0x4000_0000, -0x4000_0000]);
    for v in vals {
        let (out_c, out_r) = run_in_process(&c, &r, &format!("driver:{v}"));
        assert_eq!(out_c, out_r, "driver({v}) diverged");
        assert_eq!(
            out_c, b"5\n",
            "driver({v}) must take the good arm: any non-zero int is true in C"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 12 — driver(0): the uninitialised-read arm, via the wrapper
// ---------------------------------------------------------------------------
fn row12_driver_zero_structural() {
    // Unspecified output (see ERRORS.md rows 6/7): the C library prints a
    // different leaked stack address on every run, so byte equality is not a
    // property the C itself has. Compare what IS specified.
    let (oc, or) = assert_same_termination("driver:0");
    if !oc.crashed() {
        assert!(
            is_one_int_line(&oc.stdout),
            "C driver(0) should emit one printf(\"%d\\n\") line, got {:?}",
            oc.text()
        );
        assert!(
            is_one_int_line(&or.stdout),
            "Rust driver(0) should emit one printf(\"%d\\n\") line, got {:?}",
            or.text()
        );
    }
    // Repeat a few times: the two must keep agreeing on whether they survive.
    for _ in 0..5 {
        assert_same_termination("driver:0");
    }
}

// ---------------------------------------------------------------------------
// Row 13 — bad() called directly (one frame shallower than row 12)
// ---------------------------------------------------------------------------
fn row13_bad_direct_structural() {
    let (oc, or) = assert_same_termination("bad");
    if !oc.crashed() {
        assert!(
            is_one_int_line(&oc.stdout),
            "C bad() output shape: {:?}",
            oc.text()
        );
        assert!(
            is_one_int_line(&or.stdout),
            "Rust bad() output shape: {:?}",
            or.text()
        );
    }
    // bad() twice in a row: the second call reads the slot the first one wrote,
    // which IS deterministic, so this compares bytes exactly.
    assert_same_isolated("bad_bad");
}

// ---------------------------------------------------------------------------
// Row 14 — bad() after good(): controlled residue, deterministic, byte-exact
// ---------------------------------------------------------------------------
fn row14_bad_after_good_exact() {
    // good() stores `5` at [rbp-12] and `&5` at [rbp-8]. A following bad() at
    // the same stack depth gets the same rbp, so it reads exactly that pointer
    // and must print 5. This is the configuration that pins down bad()'s frame
    // layout, and it fails loudly if the frame size or the call sequence in the
    // translation differs from the C.
    assert_same_isolated("good_bad");
    let (c, _r) = run_isolated_both("good_bad");
    assert_eq!(
        c.stdout, b"5\n5\n",
        "C good();bad() should print 5 twice (bad re-reads good's slot)"
    );

    // Same thing in-process, where it is also safe as long as the frame layout
    // matches. Guarded so that a mistranslation is reported rather than taking
    // the runner down with a SIGSEGV.
    let (lc, lr) = (load_c(), load_rust());
    assert_same_in_process_guarded(&lc, &lr, "good_bad");
}

// ---------------------------------------------------------------------------
// Row 15 — bad() after printIntPtrLine(&v): value-dependent and byte-exact
// ---------------------------------------------------------------------------
fn row15_bad_after_pipl_exact() {
    // printIntPtrLine spills its argument to [rbp-8]; bad() at the same depth
    // reads that very slot, so it prints the SAME value again. This makes the
    // uninitialised read observable as a function of the input, and it is the
    // strongest available check that the two frame layouts agree.
    let (lc, lr) = (load_c(), load_rust());
    let mut rng = Rng::new(0x5EED_0015);

    // One guarded probe first: if the frame layout is wrong this reports cleanly
    // instead of faulting the runner on the first iteration.
    assert_same_in_process_guarded(&lc, &lr, "pipl_bad:0");

    for _ in 0..256 {
        let v = rng.next_i32();
        let spec = format!("pipl_bad:{v}");
        let (out_c, out_r) = run_in_process(&lc, &lr, &spec);
        assert_eq!(out_c, out_r, "{spec} diverged");
        assert_eq!(
            out_c,
            format!("{v}\n{v}\n").as_bytes(),
            "{spec}: bad() should re-read printIntPtrLine's spilled pointer"
        );
    }
    for v in boundary_i32s() {
        assert_same_in_process_guarded(&lc, &lr, &format!("pipl_bad:{v}"));
    }
    // And once through a fresh process, to confirm it does not depend on the
    // in-process warm-up state.
    assert_same_isolated("pipl_bad:-12345");
    assert_same_isolated("pipl_bad:2147483647");
}

// ---------------------------------------------------------------------------
// Row 16 — alternating driver(1)/driver(0) sequences (composed pipeline)
// ---------------------------------------------------------------------------
fn row16_driver_sequences() {
    let mut rng = Rng::new(0x5EED_0016);

    // Sequences containing a 0 reach the unspecified arm, so compare
    // termination plus the line COUNT and the shape of every line: one
    // printf("%d\n") per driver() call, `5` for each 1.
    for _ in 0..12 {
        let len = 1 + rng.below(8);
        let bits: String = (0..len)
            .map(|_| if rng.next_u64() & 1 == 0 { '0' } else { '1' })
            .collect();
        let spec = format!("driver_seq:{bits}");
        let (oc, or) = assert_same_termination(&spec);
        if !oc.crashed() {
            for (out, who) in [(&oc, "C"), (&or, "Rust")] {
                let text = out.text();
                let lines: Vec<&str> = text.lines().collect();
                assert_eq!(
                    lines.len(),
                    len,
                    "{who} {spec}: expected one line per driver() call, got {text:?}"
                );
                for (ch, line) in bits.chars().zip(&lines) {
                    assert!(
                        line.parse::<i64>().is_ok(),
                        "{who} {spec}: line {line:?} is not a decimal int"
                    );
                    if ch == '1' {
                        assert_eq!(*line, "5", "{who} {spec}: driver(1) must print 5");
                    }
                }
            }
        }
    }

    // All-ones sequences are fully specified, so compare bytes exactly.
    for len in [1usize, 2, 3, 7, 32] {
        let bits = "1".repeat(len);
        assert_same_isolated(&format!("driver_seq:{bits}"));
        let (oc, _) = run_isolated_both(&format!("driver_seq:{bits}"));
        assert_eq!(oc.stdout, "5\n".repeat(len).as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 17 — driver with garbage in the upper 32 bits of rdi
// ---------------------------------------------------------------------------
fn row17_driver_wide_rdi() {
    let (c, r) = (load_c(), load_rust());

    // Low half non-zero => good arm, regardless of the upper half.
    for hi in [0u64, 1, 0xDEAD_BEEF, u32::MAX as u64] {
        for lo in [1u32, 5, 0xFFFF_FFFF, 0x8000_0000] {
            let v = (hi << 32) | lo as u64;
            let spec = format!("driver_wide:{v}");
            let (out_c, out_r) = run_in_process(&c, &r, &spec);
            assert_eq!(out_c, out_r, "{spec} diverged");
            assert_eq!(
                out_c, b"5\n",
                "{spec}: only edi is tested, low half is non-zero"
            );
        }
    }

    // Low half zero => bad arm even though rdi as a whole is non-zero. This is
    // the interesting FFI case; output is unspecified, so compare termination.
    for hi in [1u64, 0xDEAD_BEEF, u32::MAX as u64] {
        let v = hi << 32;
        assert_same_termination(&format!("driver_wide:{v}"));
    }
}

// ---------------------------------------------------------------------------
// Row 18 — codegen parity of all four exported functions
// ---------------------------------------------------------------------------
fn row18_codegen_parity() {
    // The residue-dependent rows above only hold because the two libraries have
    // the same frame geometry. Assert that directly so a regression is reported
    // here rather than as a mysterious garbage-value mismatch.
    for func in ["printIntPtrLine", "bad", "good", "driver"] {
        let c = disasm(&c_so_path(), func);
        let r = disasm(&rust_so_path(), func);
        assert!(!c.is_empty(), "no disassembly for C {func}");
        assert_eq!(
            c, r,
            "codegen for {func} differs\n  C   : {c:?}\n  Rust: {r:?}"
        );
    }
}

/// `objdump -d` the named function and return its instructions, normalised:
/// addresses, RIP-relative displacements and PLT slot numbers dropped, so only
/// mnemonics plus register/immediate operands remain.
fn disasm(so: &std::path::Path, func: &str) -> Vec<String> {
    let out = std::process::Command::new("objdump")
        .args(["-d", "--no-show-raw-insn"])
        .arg(so)
        .output()
        .expect("run objdump");
    assert!(out.status.success(), "objdump failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);

    let mut insns = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.ends_with(&format!("<{func}>:")) {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        let body = match line.split_once(":\t") {
            Some((_addr, body)) => body.trim(),
            None => break, // blank line: end of the function
        };
        // Drop the "# 1234 <sym>" comment objdump appends to RIP-relative refs.
        let body = body.split('#').next().unwrap().trim();
        // Normalise call/jmp targets: keep the callee NAME (or `plt`), drop the
        // address, and drop RIP displacements which legitimately differ.
        let norm = normalise_operands(body);
        let is_terminator = norm.starts_with("ret");
        insns.push(norm);
        if is_terminator {
            break;
        }
    }
    insns
}

fn normalise_operands(insn: &str) -> String {
    let (mnemonic, rest) = match insn.split_once(char::is_whitespace) {
        Some((m, r)) => (m, r.trim()),
        None => return insn.to_string(),
    };
    if mnemonic.starts_with("call") || mnemonic.starts_with("j") {
        // "1173 <printIntPtrLine@plt>" -> "printIntPtrLine@plt";
        // a bare local target ("11be <driver+0x1d>") -> "local".
        let target = rest
            .split_once('<')
            .and_then(|(_, t)| t.strip_suffix('>'))
            .unwrap_or("local");
        let target = match target.split_once('+') {
            Some(_) => "local", // intra-function branch: address differs, fine
            None => target,
        };
        return format!("{mnemonic} {target}");
    }
    // Replace any RIP-relative displacement with a placeholder.
    let rest = if rest.contains("(%rip)") {
        let mut s = String::new();
        let mut chars = rest.char_indices().peekable();
        while let Some((i, ch)) = chars.next() {
            if rest[i..].starts_with("(%rip)") {
                // Strip the numeric displacement we already pushed.
                while s
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_ascii_hexdigit() || c == 'x' || c == '-')
                {
                    s.pop();
                }
                s.push_str("DISP(%rip)");
                for _ in 0..5 {
                    chars.next();
                }
                continue;
            }
            s.push(ch);
        }
        s
    } else {
        rest.to_string()
    };
    format!("{mnemonic} {rest}")
}

// ---------------------------------------------------------------------------
// Entry point (`harness = false`; see the comment in Cargo.toml)
// ---------------------------------------------------------------------------
fn main() -> ! {
    common::run_tests(driver_tests())
}

fn driver_tests() -> &'static [common::Test] {
    tests![
        row01_pipl_stack_random,
        row02_pipl_stack_boundaries,
        row03_pipl_heap_random,
        row04_pipl_static_random,
        row05_pipl_array_index,
        row06_pipl_unaligned,
        row07_pipl_repeated_loop,
        row08_good,
        row09_good_interleaved,
        row10_driver_one,
        row11_driver_nonzero_random,
        row12_driver_zero_structural,
        row13_bad_direct_structural,
        row14_bad_after_good_exact,
        row15_bad_after_pipl_exact,
        row16_driver_sequences,
        row17_driver_wide_rdi,
        row18_codegen_parity,
    ]
}
