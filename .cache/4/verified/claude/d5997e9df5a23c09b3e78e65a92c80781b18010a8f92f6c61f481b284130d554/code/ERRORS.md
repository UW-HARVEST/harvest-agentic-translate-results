# ERRORS.md — Phase C error-surface table

Derived **mechanically** from `c_src/src/lib.c` (35 lines) and
`c_src/include/lib.h` (7 lines).

## Mechanical grep results

```
grep -nE "return|NULL|assert|abort|exit|errno|-1|if *\(|while|for" \
     c_src/src/lib.c c_src/include/lib.h
```

⇒ **zero** matches that are error paths. The only hits are the three `-` signs
inside floating-point expressions (lines 6, 20, 21), not `return -1`.

The C library therefore has:

* no `return` statements at all (every function is `void`),
* no error enum / error code / sentinel value,
* no `assert`, `abort`, `exit`, `errno` use,
* no explicit range check, no null check, no min/max constant,
* no `RETURN_ERROR`-style macro,
* no `#ifdef` / preprocessor conditional (only `#include "lib.h"`).

The **entire** rejection surface is consequently *implicit*, and there are
exactly two mechanisms:

1. **`switch (Impairment)` in `colourblind` has no `default:` label**
   (`c_src/src/lib.c:25-34`). Any value that is not `cbProtanopia`(0),
   `cbDeuteranopia`(1) or `cbTritanopia`(2) falls straight through the `switch`
   and the function returns having touched nothing. `*R`, `*G`, `*B` keep their
   caller-supplied values; the pointers are **never dereferenced**.
   Confirmed in codegen — GCC emits an **unsigned** comparison, so the C
   `cb_impairment` enum's underlying type is `unsigned int` and *every* negative
   `int` also lands in the fall-through path:
   ```
   13e9: cmpl $0x2,-0x4(%rbp)
   13ed: je   1435          # case 2 -> Tritanopia
   13ef: cmpl $0x2,-0x4(%rbp)
   13f3: ja   144e          # UNSIGNED >2  -> fall through to ret
   13f5: cmpl $0x0,-0x4(%rbp) ; je 1403   # case 0 -> Protanopia
   13fb: cmpl $0x1,-0x4(%rbp) ; je 141c   # case 1 -> Deuteranopia
   1401: jmp  144e          # -> ret
   ```
2. **The three `static` helpers dereference their pointers unconditionally**
   (`*Red`, `*Green`, `*Blue` — `c_src/src/lib.c:4,11,18`) with no null guard.
   For a *valid* impairment a null pointer is therefore an unconditional
   invalid access ⇒ `SIGSEGV`.

## ERROR-SURFACE TABLE

One row per distinct rejection / invalid-input condition the C actually
distinguishes. "no-op" means: function returns normally, `*R`/`*G`/`*B`
bit-identical to their pre-call values, no pointer dereferenced.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `colourblind` | `Impairment == 3` — first value one step past the last valid enumerator `cbTritanopia`(2) | no-op; `*R`,`*G`,`*B` unchanged | `err_e1_impairment_3` | [x] |
| E2 | `colourblind` | `Impairment == 4` (second value past range) | no-op | `err_e2_impairment_4` | [x] |
| E3 | `colourblind` | `Impairment == -1` — negative; `ja` treats it as `0xFFFFFFFF` | no-op | `err_e3_impairment_neg1` | [x] |
| E4 | `colourblind` | `Impairment == INT_MIN` (`-2147483648`, `0x80000000`) | no-op | `err_e4_impairment_int_min` | [x] |
| E5 | `colourblind` | `Impairment == INT_MAX` (`2147483647`, `0x7FFFFFFF`) | no-op | `err_e5_impairment_int_max` | [x] |
| E6 | `colourblind` | `Impairment` = every other out-of-range value: exhaustive `3..=4096`, all `-4096..=-1`, plus randomized 32-bit values | no-op for all | `err_e6_impairment_out_of_range_sweep` | [x] |
| E7 | `colourblind` | out-of-range enum **and** `R == G == B == NULL` (0x0) simultaneously — the switch falls through *before* any deref, so this must NOT crash | returns normally, no crash, no deref | `err_e7_null_ptrs_with_invalid_impairment` | [x] |
| E8 | `colourblind` | out-of-range enum with wild/unmapped non-null pointers (`0x1`, `0xdeadbeef`, `usize::MAX`) — again no deref | returns normally, no crash | `err_e8_wild_ptrs_with_invalid_impairment` | [x] |
| E9 | `colourblind` | `Impairment == 0` (`cbProtanopia`, valid) with `R == NULL` | `SIGSEGV` (unconditional `*Red` read at `lib.c:4`) | `err_e9_null_r_segv_matches` (subprocess) | [x] |
| E10 | `colourblind` | `Impairment == 1` (`cbDeuteranopia`, valid) with `G == NULL` | `SIGSEGV` (`*Green` at `lib.c:11`) | `err_e10_null_g_segv_matches` (subprocess) | [x] |
| E11 | `colourblind` | `Impairment == 2` (`cbTritanopia`, valid) with `B == NULL` | `SIGSEGV` (`*Blue` at `lib.c:18`) | `err_e11_null_b_segv_matches` (subprocess) | [x] |
| E12 | `colourblind` | valid enum, all three pointers NULL | `SIGSEGV` | `err_e12_all_null_segv_matches` (subprocess) | [x] |

### Notes on rows E9–E12

Null dereference is undefined behaviour in **both** languages, so the contract
verified is the *observable* one: for the same input the C `.so` and the Rust
`.so` must fail the same way. These rows are executed in a forked child process
(`Command`-based re-exec of the test binary) and the test asserts the child was
killed by the **same signal** (`SIGSEGV`, 11) for both libraries — not merely
"both failed somehow".

### Generic boundaries also covered (not distinct C branches, but mandated)

| condition | where |
|-----------|-------|
| zero and negative-zero inputs | Phase B `CONFIGS.md` rows C10–C15 |
| subnormal / denormal inputs | Phase B rows C16–C18 |
| oversized magnitudes (`±FLT_MAX`, sums overflowing to `±inf`) and `FLT_MIN` | Phase B rows C19–C24 |
| `inf` / `-inf` inputs (incl. `inf - inf` ⇒ NaN) | Phase B rows C25–C27, C31–C33 |
| `NaN` inputs — sign, payload and signalling-NaN quieting | Phase B rows C28–C30, C64 |
| every other `f32` bit pattern (arbitrary `u32` reinterpreted) | Phase B rows C37–C39, C65 |
| aliasing (same pointer passed as two or three args) | Phase B rows C49–C60 |
| unaligned `float*` | Phase B row C61 |
| out-of-range enum values crossing FFI | E1–E8 above |

Note there is no length/size parameter anywhere in this API (`colourblind`
takes three scalar `float *`, not a buffer plus count), so the "zero and
oversized lengths" boundary has no counterpart here; the corresponding scalar
boundaries are the magnitude rows listed above.

## Verification status

All 12 rows have a passing differential test, plus an `E0` control row that
proves the child-process mechanism itself works (a valid call exits 0).

Run:
```
cargo test --no-default-features --test phase_c_errors            # dev
cargo test --no-default-features --release --test phase_c_errors  # release
```

Result: **13 passed, 0 failed, 1 ignored** (the ignored test is
`zz_child_worker`, the internal child-process worker driven by E7–E12) in both
profiles.

Efficacy check: adding a `default:` arm to the Rust `switch` makes rows E1–E6
fail immediately, confirming these tests are not vacuous (see the mutation table
in `CONFIGS.md`).

### Signal parity for rows E9–E12

| profile | C child | Rust child | verdict |
|---------|---------|------------|---------|
| `release` (shipped artifact) | `SIGSEGV` (11) | `SIGSEGV` (11) | exact match |
| `dev` (`debug_assertions` on) | `SIGSEGV` (11) | `SIGABRT` (6) | accepted: `core::ptr::read_unaligned`'s "dereferenceable and non-null" precondition check traps the UB *before* the faulting access. This is a `rustc` diagnostic, not a difference in the library, and the test asserts the C side really did segfault before allowing it. |
