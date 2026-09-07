# CONFIGS.md — Configuration-surface table

## Derivation

Enumerated the axes the C actually distinguishes, from the source only:

* **Runtime options / modes / flags:** none. `c_src/include/lib.h` exposes no
  setter, no context/handle struct, no global state, no init/teardown call.
  `grep -c 'if\|switch\|#if' c_src/src/lib.c` == 0, so there is no flag for the
  code to branch on. The library is stateless and re-entrant.
* **Compile-time configuration:** none. No `#ifdef` in `c_src/`, and
  `CMakeLists.txt` defines no `target_compile_definitions`.
* **Full set of public entry points:** exactly one — `rev16`. It *is* the
  lowest-level entry point; there is no convenience wrapper layered over
  anything, so "exercise the low-level API, not just the wrappers" is satisfied
  by calling `rev16` directly through the `.so`.
* **Input shapes the code special-cases:** `rev16` is branch-free, so it
  special-cases nothing *control-flow*-wise. The meaningful axes are therefore
  the **data shapes the bit-manipulation treats differently**, which is where
  a value-dependent translation bug would hide:
  * *width* — payload confined to the low 16 bits vs. spilling into the high 16
    bits (the masks are only 16 bits wide, so the first statement **discards**
    bits 16..=31; a translation that used 32-bit masks, or that used a rotate or
    `swap_bytes`/`reverse_bits` shortcut, would diverge exactly here);
  * *bit density* — empty (0), one bit, few bits, many bits, all bits;
  * *symmetry* — bit-palindromes (input where reversal is the identity) vs.
    asymmetric values, since a palindrome masks an incorrect bit order;
  * *nibble/byte alignment* — values aligned to the 1/2/4/8 shift boundaries of
    the four statements, to catch a wrong shift amount or a swapped mask pair;
  * *signedness / promotion* — values with bit 31 or bit 15 set, which is where
    a `u32`-vs-`i32` or a sign-extension mistake would surface.

Rows below are the cross-product of those axes, pruned to the combinations that
are genuinely distinguishable in the output.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `rev16` | no options (stateless); input `0` — empty, zero bit density | [x] |
| 2 | `rev16` | no options; every single-bit input `1u32 << k` for k in 0..=15 — minimum density, in-mask, each maps to bit `15-k` | [x] |
| 3 | `rev16` | no options; every single-bit input `1u32 << k` for k in 16..=31 — minimum density, **out-of-mask** (result must be 0) | [x] |
| 4 | `rev16` | no options; `0xFFFF` — all in-mask bits set, maximum in-mask density, bit-palindrome | [x] |
| 5 | `rev16` | no options; `0xFFFFFFFF` — all 32 bits set, high half must be discarded (result `0xFFFF`, not `0xFFFFFFFF`) | [x] |
| 6 | `rev16` | no options; `0xFFFF0000` — payload entirely in the discarded high half (result must be `0`) | [x] |
| 7 | `rev16` | no options; high-half garbage + low-half payload (`(hi << 16) \| lo`, randomized) — proves high bits never leak into the result | [x] |
| 8 | `rev16` | no options; byte-aligned shapes `0x00FF`, `0xFF00`, `0xAA00`, `0x00AA` — exercises the `>>8`/`<<8` statement boundary | [x] |
| 9 | `rev16` | no options; nibble-aligned shapes `0x0F0F`, `0xF0F0`, `0x0F00`, `0x00F0` — exercises the `>>4`/`<<4` statement | [x] |
| 10 | `rev16` | no options; pair/adjacent-bit shapes `0x3333`, `0xCCCC`, `0x5555`, `0xAAAA` — exercises the `>>1`/`<<1` and `>>2`/`<<2` statements and the exact mask constants | [x] |
| 11 | `rev16` | no options; bit-palindromic inputs (`x` such that `rev16(x) == x`, randomized) — reversal is the identity | [x] |
| 12 | `rev16` | no options; sign-bit-adjacent shapes — bit 15 set (`0x8000`), bit 31 set (`0x80000000`), both (`0x80008000`) — catches signed-vs-unsigned promotion | [x] |
| 13 | `rev16` | no options; involution property — `rev16(rev16(x) ) == rev16(x) & 0xFFFF` reversed twice, randomized (composed pipeline, calling the entry point repeatedly) | [x] |
| 14 | `rev16` | no options; uniformly random full-range `u32` inputs, fixed seed, many iterations (mixed density/symmetry/width) | [x] |
| 15 | `rev16` | no options; random inputs biased to sparse and to dense bit patterns (`rand & rand`, `rand \| rand`), fixed seed | [x] |
| 16 | `rev16` | no options; exhaustive sweep of `0..=0x1FFFF` (all 16-bit values plus one bit past, proving the whole in-mask domain) | [x] |

All 16 rows are checked off; each was verified to pass in
`translation/tests/differential.rs` with both libraries loaded via `libloading`.

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED ...)` — there is no
`add_executable`, and `translation/Cargo.toml` declares only `[lib]` with
`crate-type = ["cdylib"]` and has no `src/main.rs`. **No driver binary exists on
either side, so the stdout-comparison obligation is not applicable.**

## Feature combinations

`translation/Cargo.toml` has no `[features]` section and no optional
dependencies, so the feature powerset is the single empty combination. Verified
that `--no-default-features` and the default build are the same build (see
`check_feature_combos.sh`).
