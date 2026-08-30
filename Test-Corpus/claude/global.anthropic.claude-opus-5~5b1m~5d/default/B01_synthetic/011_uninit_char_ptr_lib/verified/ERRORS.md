# Phase A.2 — Error / rejection surface table

## Mechanical derivation

Every rejection mechanism in `c_src/src/driver.c`, found by grepping for all of
them rather than by reading the happy path:

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|-1|if *\(|#if' c_src/src/driver.c
30:    if (line != NULL)
39:    printLine(data);      <- passes an UNINITIALIZED `char *data`
51:    if (useGood)
```

Result of the sweep:

* **0** `return <errorcode>` statements — all four functions return `void`.
* **0** `assert`, `abort`, `exit`, `errno`, error enums, error macros
  (`RETURN_ERROR` and friends), or numeric/`NULL` sentinels.
* **1** explicit null check: `printLine`'s `if (line != NULL)`.
* **0** range / min / max / size / count constants (no arrays, no lengths).
* **0** `#if`/`#ifdef` configuration.
* **1** implicit "rejection-ish" branch: `driver`'s `if (useGood)` zero test.
* **1** latent memory-safety defect: `bad()` hands an **uninitialised** `char *`
  to `printLine` (CWE-457 / CWE-824). This is the intentional defect of the
  sample and is preserved, not fixed.

So this library's error surface is a *rejection-by-silence* surface. Each row
below therefore states the exact observable result: the bytes written to stdout
**and** how the call terminates (clean return vs. fatal signal). Rows that can
fault are asserted in a forked child so that "both crashed the same way" is
distinguished from "both returned normally" — never merely "both failed
somehow".

## The table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test (`tests/errors.rs`) | ✅ |
|----|----------|---------------------------------------------|-------------------|--------------------------|----|
| E1 | `printLine` | `line == NULL` (literal null pointer across FFI) | takes the false branch: **0 bytes**, `exit(0)` | `err_e1_print_line_null` | [x] |
| E2 | `printLine` | `line` points at a lone `'\0'` (empty string = "zero length") | non-null ⇒ `puts("")` ⇒ exactly `"\n"` | `err_e2_print_line_empty` | [x] |
| E3 | `printLine` | oversized length — 1 MiB of `'A'`; the NUL terminator is the *only* bound, there is no length cap | 1 MiB of `'A'` then `"\n"`, no truncation | `err_e3_print_line_oversized` | [x] |
| E4 | `printLine` | embedded NUL: bytes after the first NUL must be dropped | output stops at (excluding) the first NUL, then `"\n"` | `err_e4_print_line_embedded_nul` | [x] |
| E5 | `printLine` | `%`, `%s`, `%n`, `%*d` … in the *data* (it is the argument, not the format) | `%` sequences emitted literally, **no** format interpretation, no `%n` write | `err_e5_print_line_percent` | [x] |
| E6 | `printLine` | non-UTF-8 / high bytes `0x80..=0xFF` (still valid C strings) | bytes copied through verbatim | `err_e6_print_line_invalid_utf8` | [x] |
| E7 | `driver` | `useGood == 0` — selects the defective `bad()` branch | calls `bad()`, which reads an indeterminate slot; here `"\n"` + `exit(0)`, and `SIGSEGV` from a dirtied stack | `err_e7_driver_zero` | [x] |
| E8 | `driver` | `useGood` with **no valid enum/bool variant**: `-1`, `2`, `3`, `-2`, `INT_MIN`, `INT_MAX`, `0x100`, `0xFFFF`, `0xFFFFFF00u32 as i32`, … | C tests plain truthiness, so *every* non-zero int ⇒ `good()` ⇒ `"string\n"`; only exactly `0` ⇒ `bad()` | `err_e8_driver_out_of_range_enum` | [x] |
| E9 | `driver` | a value one step past the 32-bit range: the symbol called through a `fn(u64)` pointer with `0x1_0000_0000`, `0x1_0000_0001`, `0xFFFFFFFF_00000000` | the callee reads only `%edi`, so these truncate to `0`, `1`, `0` and pick `bad`, `good`, `bad` | `err_e9_driver_int_truncation` | [x] |
| E10 | `bad` | the uninitialised read itself (CWE-457), from a clean stack and from stacks pre-dirtied with 4 fill patterns × 3 recursion depths | returns normally printing the stale bytes when the slot is a readable pointer; `SIGSEGV` when it is not | `err_e10_bad_uninitialized_read` | [x] |
| E11 | `good` | no invalid input is representable (no parameters) — included so every entry point has a row | `"string\n"`, `exit(0)` | `err_e11_good_no_args` | [x] |

## Generic FFI-boundary cases (required even though not in the C source)

| #  | case | expected | test | ✅ |
|----|------|----------|------|----|
| G1 | null pointer to `printLine` | silent no-op, clean return (= E1) | `err_e1_print_line_null` | [x] |
| G2 | zero length (empty string) | `"\n"` (= E2) | `err_e2_print_line_empty` | [x] |
| G3 | oversized length (1 MiB) | full passthrough (= E3) | `err_e3_print_line_oversized` | [x] |
| G4 | out-of-range enum int to `driver` (no valid variant) | truthiness only (= E8) | `err_e8_driver_out_of_range_enum` | [x] |
| G5 | one step past the documented range (`-1` and `2`) | both non-zero ⇒ `good()` | `err_e8_driver_out_of_range_enum` | [x] |
| G6 | repeated / interleaved calls — no hidden global state may drift | the Nth call is identical to the 1st (perfectly periodic stream over 50 rounds) | `err_g6_no_hidden_state` | [x] |
| G7 | pointer shapes: pointer to a buffer's terminating NUL (off-by-one), odd/unaligned offsets, read-only static storage | all are valid C strings and must behave | `err_extra_pointer_shapes` | [x] |
| G8 | wild non-NULL pointers (`1`, `8`, `0x1000`, `0xdeadbeef`, top of the address space) — the C has no way to reject these | both must fault **identically** (same signal, same partial output) | `err_extra_wild_pointer` | [x] |

## Notes

**E10 / E7 and the limits of "identical".** The C behaviour here is Undefined
Behaviour, and it is *caller-dependent*: the value is whatever eight bytes sit
16 bytes below the callee's entry stack pointer. The Rust reproduces it exactly
by matching gcc's frame layout instruction-for-instruction, so both `.so`s read
the *same address* with the *same* stale contents. The tests confirm this is not
vacuous: for `printLine("AAAA…"); bad();` both libraries dump the same ~60 bytes
of stale machine code from the stack, and for a dirtied stack both take SIGSEGV
at the same point. See `SYMBOLS.md` for the two link/codegen properties this
depends on.

The differential harness itself has to be symmetric for these rows to be
meaningful — see the module documentation in `tests/common/mod.rs`.

**Mutation-tested.** Each of these rows was checked to actually *fail* when the
translation is wrong; four deliberate regressions were injected and all were
caught:

| injected regression | caught by |
|---------------------|-----------|
| `bad()` replaced by a "safe" deterministic empty string | `cfg_c12_bad`, `cfg_c18_repeat_no_drift`, `cfg_c19_dirty_stack_matrix`, `cfg_c19b_good_then_bad_frame_aliasing` |
| `-Wl,-z,lazy` dropped from `build.rs` (i.e. `BIND_NOW`) | `cfg_c14_driver_zero`, `cfg_c15_driver_random_i32`, `cfg_c16_driver_boundaries`, `cfg_c19_dirty_stack_matrix`, `cfg_c19b_good_then_bad_frame_aliasing`, `err_e7_driver_zero`, `err_e8_driver_out_of_range_enum`, `err_e9_driver_int_truncation`, `link_configuration_matches_c` |
| `printLine`'s NULL check removed | `cfg_c10_printline_null`, `err_e1_print_line_null` |
| `driver`'s branch inverted | `cfg_c13_driver_one`, `cfg_c14_driver_zero` |
