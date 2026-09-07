# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from the C source and the public header, the same way
`ERRORS.md` is derived.

## Axes the C code actually branches on

Enumerated from every branch construct in `c_src/src/driver.c` and every
declaration reachable from `c_src/include/driver.h` + `nm -D`:

| axis | values the C distinguishes | evidence |
|------|----------------------------|----------|
| A. entry point | `driver` (header), `printLine`, `bad`, `good` (exported but not in `driver.h`) | `nm -D` shows 4 `T` symbols; `driver.h` declares only `driver`, so `printLine`/`bad`/`good` are the **low-level** entry points and `driver` is the convenience/dispatch wrapper |
| B. `useGood` truthiness | `0` (falsy → `bad`) vs any non-zero (truthy → `good`) | `if (useGood)` at driver.c:60 — the *only* runtime option/flag in the whole library |
| C. `line` nullity | `NULL` vs non-`NULL` | `if (line != NULL)` at driver.c:30 |
| D. `line` content shape | empty, 1 byte, many bytes, embedded `%` specifiers, embedded `\n`, embedded `\t`/control bytes, non-UTF-8 bytes, long (page-crossing / MiB) | C is byte-oriented via `printf("%s\n", …)`; the code special-cases nothing, so every shape must round-trip identically. Shapes chosen to break plausible mistranslations (`CStr::to_str`, `println!`, `%` re-interpretation) |
| E. storage class of the returned buffer | automatic (`helperBad`, driver.c:37) vs `static` (`helperGood1`, driver.c:48) | the two helpers differ *only* in the `static` keyword; this is the point of the library (CWE-562) |
| F. call multiplicity / ordering | 1 call, N repeated calls, interleaved `good`/`bad`/`printLine`/`driver` | `helperGood1`'s `static char charString[]` is shared mutable state with static storage duration, so repeat and interleaved calls are a distinct configuration; `bad`'s stack array can be clobbered by a preceding call's frame |
| G. build/feature configuration | default only | `grep '^\[features\]' Cargo.toml` → no match; no `#ifdef` in the C source |

There is no size/length/count parameter, no format/byte-order/element-type
option, no init/teardown handle and no global state setter anywhere in the API —
so those usual axes are genuinely absent rather than untested.

## Rows (pruned cross-product of A × B × C × D × E × F)

Every row is exercised against BOTH `.so`s via `libloading`, capturing fd 1
byte-for-byte. Rows marked "randomized" use ≥256 property-style cases from a
fixed-seed SplitMix64 PRNG.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `printLine` (low-level) | non-null, single ASCII byte (`"a"`), and all 255 single non-NUL byte values `0x01..=0xFF` | `cfg_c1_print_line_single_bytes` | [x] |
| C2 | `printLine` | non-null, plain ASCII words of length 2..64, randomized | `cfg_c2_print_line_random_ascii` | [x] |
| C3 | `printLine` | non-null, arbitrary random non-NUL bytes (`0x01..=0xFF`), length 1..512, randomized — covers invalid UTF-8 | `cfg_c3_print_line_random_bytes` | [x] |
| C4 | `printLine` | non-null, content containing `%` conversion specifiers interleaved with random bytes, randomized | `cfg_c4_print_line_random_with_percent` | [x] |
| C5 | `printLine` | non-null, content containing embedded `\n`, `\r`, `\t` and other control bytes | `cfg_c5_print_line_embedded_control_bytes` | [x] |
| C6 | `printLine` | non-null, long buffers straddling stdio buffer / page sizes: lengths 4095, 4096, 4097, 8191, 8192, 8193, 65536, 1 MiB | `cfg_c6_print_line_buffer_boundaries` | [x] |
| C7 | `printLine` | `line == NULL` (valid input for this API — the guard is a normal path) | `cfg_c7_print_line_null` | [x] |
| C8 | `good` (low-level) | no arguments; single call. Exercises `static` storage helper (axis E) | `cfg_c8_good_single` | [x] |
| C9 | `good` | repeated calls, N = 1..64 (axis F: shared `static` buffer must be stable, output must repeat verbatim) | `cfg_c9_good_repeated` | [x] |
| C10 | `bad` (low-level) | no arguments; single call. Exercises automatic-storage helper (axis E, CWE-562) | `cfg_c10_bad_single` | [x] |
| C11 | `bad` | repeated calls, N = 1..64 (axis F: stack contents differ between iterations, output must still match C) | `cfg_c11_bad_repeated` | [x] |
| C12 | `driver` (wrapper) | `useGood != 0` → `good` path; values `1`, `2`, `-1`, `INT_MAX`, `INT_MIN`, plus randomized non-zero `i32` | `cfg_c12_driver_truthy` | [x] |
| C13 | `driver` | `useGood == 0` → `bad` path | `cfg_c13_driver_falsy` | [x] |
| C14 | `driver` | randomized full-range `i32` (zero and non-zero mixed, fixed seed) — the B axis driven as a real consumer would | `cfg_c14_driver_random_i32` | [x] |
| C15 | composed pipeline | randomized interleaving of `driver(rand)`, `good()`, `bad()`, `printLine(rand bytes)`, `printLine(NULL)` — 512 ops per run, several runs; whole-session stdout compared byte-for-byte (axis F, catches state leaking between entry points) | `cfg_c15_interleaved_pipeline` | [x] |
| C16 | `printLine` after `good` | `printLine(ptr)` where `ptr` is a fresh copy of the exact bytes `good()` emits, and `printLine` called immediately after `good()`/`bad()` so the stack frame is dirty | `cfg_c16_print_line_after_helpers` | [x] |
| C17 | all four, default features | full suite under the default (only) feature configuration | whole suite | [x] |
| C18 | all four, `--no-default-features` | full suite with no default features (equivalent here; proves parity) | whole suite | [x] |

## Harness validation (proof the all-pass result is not vacuous)

Because two of this library's paths legitimately produce *empty* stdout
(`printLine(NULL)`, `bad()`), a broken capture harness would report a false
pass. Three checks rule that out:

1. Rows C8 / E2 / E3 assert exact **non-empty** expected bytes
   (`"helperGood1 string\n"`, `"\n"`, verbatim `%`-specifier echo), so fd-1
   capture is proven to work.
2. Mutation testing — three deliberate mistranslations were introduced into
   `src/lib.rs`, rebuilt, and the suite re-run; each was caught:

   | mutation | tests that failed |
   |----------|-------------------|
   | `helperBad` returns a real `"helperBad string"` pointer instead of the NULL GCC materialises | 8 failed (C10, C11, C15, C16, E4, E5, and the driver rows) |
   | `printLine` passes `line` as the `printf` *format* string | C1–C4, C15 failed, then SIGSEGV on `%n` |
   | `driver` tests `useGood == 1` instead of `!= 0` | C12, C14, C15, E6 failed |

   `src/lib.rs` was byte-restored afterwards (`diff` against the pre-mutation
   copy is empty) and the full Phase D sweep re-run green.
3. Row↔test bijection is checked mechanically: the 25 test names named in
   `ERRORS.md`/`CONFIGS.md` and the 25 names reported by
   `cargo test -- --list` are identical sets (no missing, no extra).
