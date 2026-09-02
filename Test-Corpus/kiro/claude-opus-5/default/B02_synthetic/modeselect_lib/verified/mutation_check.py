#!/usr/bin/env python3
"""Mutation check for the C-to-Rust differential suite.

The suite is only meaningful if it FAILS when the Rust diverges from the C.
Each mutant below injects one deliberate behavioural bug into
`translation/src/lib.rs`; the suite must reject it. `src/lib.rs` is restored
unconditionally on exit.

Run: python3 mutation_check.py
"""
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "src", "lib.rs")

# (name, find, replace) -- literal string replacement, no regex quoting hazards.
MUTANTS = [
    # --- d2i: the (int)double cast semantics -------------------------------
    (
        "d2i-saturating-positive",
        "    if t >= -2147483648.0 && t <= 2147483647.0 {\n        t as i32\n    } else {\n        i32::MIN\n    }",
        "    if t >= -2147483648.0 && t <= 2147483647.0 {\n        t as i32\n    } else if t > 0.0 {\n        i32::MAX\n    } else {\n        i32::MIN\n    }",
    ),
    (
        "d2i-nan-returns-zero",
        "    if x.is_nan() {\n        return i32::MIN;\n    }",
        "    if x.is_nan() {\n        return 0;\n    }",
    ),
    (
        "d2i-rust-as-cast",
        "    let t = x.trunc();\n    if t >= -2147483648.0 && t <= 2147483647.0 {\n        t as i32\n    } else {\n        i32::MIN\n    }",
        "    let t = x.trunc();\n    t as i32",
    ),
    (
        "d2i-boundary-off-by-one",
        "if t >= -2147483648.0 && t <= 2147483647.0 {",
        "if t >= -2147483648.0 && t <= 2147483646.0 {",
    ),
    # --- apply_multiplier: the fall-through switch --------------------------
    (
        "multiplier-level4-drops-ff",
        "        4 => {\n            result = result.wrapping_add(0xFF);\n",
        "        4 => {\n",
    ),
    (
        "multiplier-no-fallthrough",
        "        3 => {\n            result = result.wrapping_add(0xAB);\n            result = result.wrapping_add(0x7E);\n            result = result.wrapping_add(0x1C);\n            result = result.wrapping_add(0x05);\n        }",
        "        3 => {\n            result = result.wrapping_add(0xAB);\n        }",
    ),
    (
        "multiplier-default-sentinel",
        "            result = 0xDEAD;",
        "            result = 0xDEAF;",
    ),
    (
        "multiplier-default-keeps-base",
        "            result = 0xDEAD;",
        "            result = result.wrapping_add(0xDEAD);",
    ),
    (
        "multiplier-level-range",
        "        0 => {\n            result = result.wrapping_add(0x05);\n        }",
        "        0 | 5 => {\n            result = result.wrapping_add(0x05);\n        }",
    ),
    # --- hash_time_value ---------------------------------------------------
    ("hash-seed", "let mut hash: u32 = 0x5A5A_5A5A;", "let mut hash: u32 = 0x5A5A_5A5B;"),
    ("hash-multiplier", "hash.wrapping_mul(0x1F)", "hash.wrapping_mul(0x1D)"),
    ("hash-mask", "(hash & 0x7FFF_FFFF) as c_int", "(hash & 0xFFFF_FFFF) as c_int"),
    ("hash-byte-order", "let bytes = t.to_ne_bytes();", "let bytes = t.to_be_bytes();"),
    ("hash-xor-to-add", "hash ^= (bytes[i] as u32)", "hash = hash.wrapping_add(bytes[i] as u32)"),
    (
        "hash-signed-byte",
        "hash ^= (bytes[i] as u32) << ((i % 4) * 8);",
        "hash ^= ((bytes[i] as i8) as i32 as u32) << ((i % 4) * 8);",
    ),
    (
        "hash-loop-length",
        "for i in 0..std::mem::size_of::<time_t>() {",
        "for i in 0..4usize {",
    ),
    # --- get_modified_time -------------------------------------------------
    (
        "gmt-64bit-math",
        "    let offset_i32: c_int = offset_days\n        .wrapping_mul(86400)\n        .wrapping_add(offset_hours.wrapping_mul(3600));\n    let offset: time_t = offset_i32 as time_t;",
        "    let offset: time_t = (offset_days as i64)\n        .wrapping_mul(86400)\n        .wrapping_add((offset_hours as i64).wrapping_mul(3600));",
    ),
    ("gmt-shift-amount", "current >>= 29;", "current >>= 28;"),
    ("gmt-seconds-per-day", ".wrapping_mul(86400)", ".wrapping_mul(86401)"),
    ("gmt-unsigned-extend", "let offset: time_t = offset_i32 as time_t;", "let offset: time_t = offset_i32 as u32 as time_t;"),
    # --- classify_mode -----------------------------------------------------
    ("classify-standard-value", "        if cstr_eq(mode, b\"standard\") {\n            0x10", "        if cstr_eq(mode, b\"standard\") {\n            0x20"),
    ("classify-fallback-value", "        } else {\n            0x00\n        }", "        } else {\n            0x01\n        }"),
    ("classify-prefix-match", "        *p.add(s.len()) as u8 == 0", "        true"),
    ("classify-case-insensitive", "if *p.add(i) as u8 != b {", "if (*p.add(i) as u8).to_ascii_lowercase() != b.to_ascii_lowercase() {"),
    # --- convert_* scaling -------------------------------------------------
    ("ctf-scale", "let scaled: f64 = factor * 1e12;", "let scaled: f64 = factor * 1e11;"),
    ("cno-scale-sign", "let extreme: f64 = value * -1e15;", "let extreme: f64 = value * 1e15;"),
    # --- modeselect composition and printf output --------------------------
    ("printf-precision", "Converting double %.2e to int (may overflow)", "Converting double %.3e to int (may overflow)"),
    # `Result 1`/`Result 2` only ever print 0 or 80000000 (no hex letters), so
    # %X vs %x is unobservable there -- see the reachable-value invariant test.
    # These two target the %X sites that DO print letters.
    ("printf-hex-case-multiplier", "Complexity level: %d, Multiplier: 0x%X", "Complexity level: %d, Multiplier: 0x%x"),
    ("printf-hex-case-final", "\\nFinal result: %d (0x%X)", "\\nFinal result: %d (0x%x)"),
    ("printf-hex-case-hash", ", Hash: 0x%X", ", Hash: 0x%x"),
    ("printf-missing-newline", "\\nFinal result: %d (0x%X)\\n\\0", "Final result: %d (0x%X)\\n\\0"),
    ("printf-mode-line", "Selected mode: %s (0x%X)\\n\\0", "Selected mode: %s (0x%X)\\n\\n\\0"),
    ("modeselect-final-const", ".wrapping_mul(0x10).wrapping_add(0xBEEF)", ".wrapping_mul(0x10).wrapping_add(0xBEEE)"),
    ("modeselect-final-shift", ".wrapping_mul(0x10).wrapping_add(0xBEEF)", ".wrapping_mul(0x20).wrapping_add(0xBEEF)"),
    ("modeselect-complexity-mod", "let complexity_level: c_int = complexity % 5;", "let complexity_level: c_int = complexity.rem_euclid(5);"),
    # NOTE: `mode_selector % 4` -> `.rem_euclid(4)` is deliberately NOT a mutant.
    # The two differ only for negative selectors, and every negative
    # non-multiple of 4 makes the C SIGSEGV (ERRORS.md E29), so the difference is
    # unobservable by construction. Recorded as a known equivalent mutant.
    ("modeselect-base", "apply_multiplier(0xA0, complexity_level)", "apply_multiplier(0xA1, complexity_level)"),
    ("modeselect-hash-mod", "result.wrapping_add(time_hash % 0x1000)", "result.wrapping_add(time_hash % 0x1001)"),
    # NOTE: `result1 & 0xFF` and `result2 & 0xFF00` mask widths are likewise
    # equivalent mutants: both operands are always 0 or 0x80000000 inside
    # modeselect, so every low-bit mask yields 0. Proven by
    # `invariant_modeselect_cast_results_are_only_zero_or_int_min`. Mutating the
    # XOR *operation* instead, which IS observable:
    ("modeselect-xor-to-add", "result ^= result1 & 0xFF;", "result = result.wrapping_add(1);"),
    ("modeselect-drop-xor2", "result ^= result2 & 0xFF00;", "result ^= 1;"),
    ("modeselect-seed-hours", "get_modified_time(time_offset, seed % 24)", "get_modified_time(time_offset, seed % 25)"),
    ("modeselect-factor1", "let factor1: f64 = (seed as f64) * 1e8;", "let factor1: f64 = (seed as f64) * 1e7;"),
    ("modeselect-factor2", "let factor2: f64 = (time_offset as f64) * -1e7;", "let factor2: f64 = (time_offset as f64) * -1e8;"),
    ("modeselect-mode-order", "b\"enhanced\\0\",\n        b\"turbo\\0\",", "b\"turbo\\0\",\n        b\"enhanced\\0\","),
]


def run(cmd, timeout=600):
    return subprocess.run(
        cmd, cwd=HERE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=timeout
    ).returncode


def main():
    with open(SRC, "r") as f:
        original = f.read()
    backup = tempfile.NamedTemporaryFile("w", delete=False, suffix=".rs")
    backup.write(original)
    backup.close()

    caught, missed, skipped = [], [], []
    try:
        for name, find, repl in MUTANTS:
            if original.count(find) == 0:
                skipped.append((name, "pattern absent"))
                print(f"SKIP    {name}  (pattern absent)", flush=True)
                continue
            mutated = original.replace(find, repl, 1)
            if mutated == original:
                skipped.append((name, "no-op"))
                print(f"SKIP    {name}  (no-op)", flush=True)
                continue
            with open(SRC, "w") as f:
                f.write(mutated)
            if run(["cargo", "build", "--release"]) != 0:
                skipped.append((name, "does not compile"))
                print(f"SKIP    {name}  (does not compile)", flush=True)
                continue
            rc = run(["cargo", "test", "--release"])
            if rc == 0:
                missed.append(name)
                print(f"MISSED  {name}   <-- suite accepted a broken translation", flush=True)
            else:
                caught.append(name)
                print(f"CAUGHT  {name}", flush=True)
    finally:
        with open(SRC, "w") as f:
            f.write(original)
        run(["cargo", "build", "--release"])
        os.unlink(backup.name)

    total = len(caught) + len(missed)
    print()
    print(f"caught {len(caught)}/{total} behavioural mutants "
          f"({len(skipped)} skipped: {[s[0] for s in skipped]})")
    if missed:
        print("MISSED:", missed)
        return 1
    print("Every behavioural mutant was rejected by the suite.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
