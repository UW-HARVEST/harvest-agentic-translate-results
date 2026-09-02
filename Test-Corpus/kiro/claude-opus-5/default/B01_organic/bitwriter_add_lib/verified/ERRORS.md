# ERRORS.md — Phase A error-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep evidence

```
$ grep -n 'return' src/lib.c include/lib.h
src/lib.c:23:    return 0;

$ grep -nE 'assert|RETURN_ERROR|NULL|errno|abort|exit\(|-1' src/lib.c include/lib.h
(none)

$ grep -nE '#if|#ifdef|switch|case ' src/lib.c include/lib.h
(none)

$ grep -nE 'if|while|for|\?' src/lib.c
11:    while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100) {
13:        b = b > bits ? bits : b;

$ grep -noE '[0-9]{2,}' src/lib.c
3:32   5:18446744073709551615   6:32   11:100
```

**Finding: the C code has NO error surface.** There is exactly one `return`
statement and it is the unconditional `return 0` at the end of the function.
There are no error-return macros, no error enums, no `assert`, no null checks,
no explicit range checks, and no documented min/max validation constants. The
function is total on its declared parameter types: for *every* `bits` in
`[0, 2^32)` and *every* `val` in `[0, 2^64)` it mutates `*bw` and returns `0`.

Consequently the rows below are the complete set of "rejection" rows that the
C code actually has, plus the generic C-API boundaries the task requires us to
cover even when the table is empty. Every row states the *observed* C result,
not an invented one, and each has a differential test asserting Rust matches.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `bitwriter_add` | any input at all — the sole `return` is unconditional `return 0` (src/lib.c:23); no branch can produce a different return value | returns `0`; `*bw` mutated | `e1_return_value_is_always_zero` | [x] |
| E2 | `bitwriter_add` | `bits == 0` — degenerate zero length. `val <<= (64-0)`; shift count 64 is out of range for `uint64_t` (C UB), realised by the emitted `shlq %cl` as count masked to 6 bits → shift by 0 | returns `0`; no error; `val` left unshifted, `tot += 0` | `e2_zero_bits` | [x] |
| E3 | `bitwriter_add` | `bits == 64` — exactly the full word width, the largest "documented-sane" value. `val <<= (64-64)` = shift by 0; loop is entered for any `bw->bits` | returns `0`; no error | `e3_bits_equals_word_width` | [x] |
| E4 | `bitwriter_add` | `bits == 65` — one step past the valid range `[0,64]`. `64-65` wraps; the emitted 32-bit `sub` gives `0xFFFFFFFF`, `%cl`-masked to a shift of 63 | returns `0`; no error; silently corrupt-but-defined value | `e4_bits_one_past_word_width` | [x] |
| E5 | `bitwriter_add` | `bits` oversized: `0xFFFFFFFF`, `0x80000000`, `100`, `1000`, `0x10000000` etc. — no upper bound is ever checked | returns `0`; no error; the `i < 100` guard is what terminates the loop | `e5_oversized_bits` | [x] |
| E6 | `bitwriter_add` | `bw->bits == 63` on entry with `bits >= 1`: `b = (u32)(63-63) = 0`, so `bits -= 0` never progresses — the loop only terminates via the `i < 100` cap, executing exactly 100 iterations | returns `0`; no hang, no error; `bw->bits` ends at 63 + trailing `bits` | `e6_b_zero_hits_iteration_cap` | [x] |
| E7 | `bitwriter_add` | `bw->bits > 63` on entry (out-of-range state, e.g. 64, 65, 100, 0xFFFFFFFF): `b = (u32)(63 - bw->bits)` underflows to a huge `u32`, then clamps to `bits`; `val >> bw->bits` has an out-of-range shift count | returns `0`; no error; loop caps at 100 iterations | `e7_out_of_range_bw_bits` | [x] |
| E8 | `bitwriter_add` | `bw->bits + bits` overflows `u32` so the loop condition `(u32)(bw->bits+bits) >= 64` is **false** despite both operands being huge (e.g. `bw->bits=0xFFFFFFFF`, `bits=0x41`) — the wrap skips the loop entirely | returns `0`; loop body never runs; `bw->val` OR'd once | `e8_loop_condition_u32_wrap` | [x] |
| E9 | `bitwriter_add` | `bw->tot` arithmetic overflow: `bw->tot += bits` on `u32` with `tot` near `0xFFFFFFFF`; unsigned wrap-around is unchecked | returns `0`; `tot` wraps mod 2^32 | `e9_tot_wraps` | [x] |
| E10 | `bitwriter_add` | `bw->bits += b` / `bw->bits += bits` arithmetic overflow: final `bw->bits` addition on `u32` with no check | returns `0`; `bits` field wraps mod 2^32 | `e10_bw_bits_wraps` | [x] |
| E11 | `bitwriter_add` | out-of-range "enum"-style value across FFI: `bits` is a `tflac_u32` with no valid-variant restriction, so every one of the 2^32 ints is a real input. Sweep the whole low range plus each power-of-two boundary and each `64k±1` multiple | returns `0` for all; identical `*bw` for all | `e11_exhaustive_bits_sweep` | [x] |
| E12 | `bitwriter_add` | `bw->buffer` null / dangling, and `bw->pos`/`bw->len` inconsistent (e.g. `pos > len`) — the function never dereferences `buffer` nor reads `pos`/`len`, so no check exists and none is needed | returns `0`; `buffer`, `pos`, `len` left byte-identical | `e12_buffer_pos_len_untouched` | [x] |
| E13 | `bitwriter_add` | `bw == NULL` — dereferenced unconditionally at `bw->tot += bits` with no null check | SIGSEGV (both C and Rust); *not* an error return | `e13_null_bw_documented_only` (documented, executed under an opt-in subprocess check) | [x] |

Row E13 is the one row that cannot be asserted as an equal *return* value,
because the C code has no null check: it faults. The test documents this and
verifies both libraries agree that it faults rather than returning a value, by
running each call in a forked child process and comparing the wait status.

## Phase C result

All 13 rows pass: `cargo test --test phase_c_errors` → **13 passed, 0 failed**
against the release cdylib (12 passed against the debug cdylib, with E13 skipped
for the reason below), and against the C `.so` rebuilt at `-O2` and `-O3`.

Each row asserts the *same* outcome, not merely "both did something": the same
`int` return value **and** a byte-identical 32-byte struct image.

### One real divergence was found and fixed by this phase

Row **E13** (`bw == NULL`) initially failed:

```
c:    code=None signal=Some(11)   # SIGSEGV
rust: code=None signal=Some(6)    # SIGABRT
```

Cause: the Rust translation opened with

```rust
let bw: &mut tflac_bitwriter = unsafe { &mut *bw };
```

Forming a Rust reference from the caller's raw pointer asserts non-null and
aligned. With `debug_assertions` on, rustc's `ub_checks` turn a NULL `bw` into
the panic "null pointer dereference occurred", which — escaping an
`extern "C"` fn — aborts with `SIGABRT`. The C has no null check and simply
faults with `SIGSEGV`.

Fix (in `src/lib.rs`, never in the C): access the fields through raw place
expressions, `(*bw).tot`, `(*bw).bits`, `(*bw).val`, exactly as the C does. This
also stops the translation from claiming the `noalias` guarantee that the C
pointer does not carry. Both `.so`s now fault identically. Note that rustc's
`ub_checks` also fire on raw place access in debug builds, so the release
artifact — the shipped one — is what E13 is asserted against; `SYMBOLS.md`
records the profile caveat.

### Coverage beyond the table

Also exercised, as required, even though the C checks none of them:

* null pointer (E13), zero length (E2), oversized lengths (E5, up to
  `0xFFFFFFFF`), one step past the valid range (E4 `bits == 65`, E7
  `bw->bits == 64`);
* out-of-range "enum"-style values across the FFI boundary (E11): `bits` is a
  bare `tflac_u32` with no valid-variant set, so the whole `0..=512` range, every
  power-of-two boundary ±1, and every multiple of 64 ±1 up to 4096 are swept
  against several incoming states;
* unsigned wrap-around on both accumulators (E9 `tot`, E10 `bits`) including a
  200-value sweep straddling `0xFFFFFFFF`;
* inconsistent/garbage untouched fields (E12): `pos > len`, null and bogus
  non-null `buffer`, all asserted to come back unchanged.
