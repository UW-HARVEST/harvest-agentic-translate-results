# ERRORS.md — Phase C error-surface table

## How this table was derived (mechanical)

Every control-flow / rejection construct in the entire C library, found by
grepping `c_src/src/driver.c` + `c_src/include/driver.h` for
`return | NULL | assert | if | else | switch | #if | error | ERROR | -1 | MAX | MIN | < | > | == | !=`:

```
src/driver.c:30:    if (line != NULL)      <-- the ONLY explicit rejection check
src/driver.c:39:    return charString;     <-- returns address of an automatic array (CWE-562)
src/driver.c:50:    return charString;     <-- returns address of a static array (well defined)
src/driver.c:60:    if (useGood)           <-- mode select, not a rejection
src/driver.c:64:    else
include/driver.h:24:#ifndef DRIVER_H_      <-- include guard
```

Findings, stated exhaustively so the blind spots are explicit:

* **0** `assert` / `static_assert`.
* **0** error-return macros (`RETURN_ERROR`, `CHECK`, `goto fail`, …).
* **0** error enums, error codes, or `errno` use.
* **0** explicit range checks, and **0** min/max constants.
* **0** allocation calls, so **0** allocation-failure paths.
* **1** null check (`driver.c:30`).
* Every public function returns `void`, so **no** function can report an error
  to its caller. The *only* observable "result" of any call is (a) the bytes
  written to `stdout` and (b) whether the call returns normally instead of
  crashing. Both tables below therefore assert on exactly those two things.
* `helperBad` (`driver.c:36-40`) returns the address of the automatic array
  `charString`, which is undefined behaviour. In the reference build the
  compiler diagnoses it (`-Wreturn-local-addr`) and emits a literal
  `mov $0x0,%eax`, i.e. `helperBad` returns **NULL**:

  ```
  000000000000115b <helperBad>:
    115f: movabs $0x61427265706c6568,%rax   # "helperBa"
    1169: movabs $0x676e697274732064,%rdx   # "d string"
    1173: mov    %rax,-0x20(%rbp)           # dead stores into the frame
    1177: mov    %rdx,-0x18(%rbp)
    117b: movb   $0x0,-0x10(%rbp)
    117f: mov    $0x0,%eax                  # <-- returns NULL
    1184: pop    %rbp
    1185: ret
  ```

  So the CWE-562 defect *feeds the null check*: `bad()` → `printLine(NULL)` →
  no output. This is the single most important row in this file, and the one a
  "helpful" translation gets wrong by printing `helperBad string`.

## Table — every distinct rejection the C code performs

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `printLine` | `line == NULL` (the `if (line != NULL)` guard at `driver.c:30` is false) | guard rejects; **zero bytes** written to stdout; returns normally (no crash, no return value) | `e1_print_line_null` | [x] |
| E2 | `bad` | no arguments — the invalid *condition* is internal: `helperBad()` hands back the dangling address of its automatic `charString`, which the reference build materialises as NULL, so `printLine` receives NULL and takes the E1 rejection path | **zero bytes** written to stdout (the text `helperBad string` is *never* printed); returns normally | `e2_bad_prints_nothing` | [x] |
| E3 | `driver` | `useGood == 0` — the false side of `if (useGood)` at `driver.c:60`, routing to `bad()` and thus to the E2/E1 rejection | **zero bytes** written to stdout; returns normally | `e3_driver_zero_prints_nothing` | [x] |

Those are all three rejection branches in the library; there is no fourth.

## Mandated generic FFI-boundary cases

Required by the task even though the C code contains no check for them. "Same
rejection" here means *identical stdout bytes and identical
returns-normally-vs-crashes behaviour* from both `.so`s, since `void` is the
only return type in the API.

| # | function | trigger | expected C result | test | ✔ |
|---|----------|---------|-------------------|------|---|
| G1 | `printLine` | null pointer (duplicate of E1, asserted again through the raw `.so` symbol) | no output | `e1_print_line_null` | [x] |
| G2 | `printLine` | **zero length**: pointer to a buffer whose first byte is `\0` (empty string) — non-NULL, so the guard passes | prints exactly one byte, `"\n"` | `g2_print_line_empty_string` | [x] |
| G3 | `printLine` | **oversized length**: 1 byte, 4095, 4096, 4097, 65536 and 1 MiB NUL-terminated payloads | full payload + `"\n"`, no truncation at any stdio buffer boundary | `g3_print_line_oversized` | [x] |
| G4 | `printLine` | interior / unaligned pointer: `p.add(k)` for odd `k` into a larger buffer | prints the tail from `k` + `"\n"` | `g4_print_line_interior_pointer` | [x] |
| G5 | `printLine` | payload containing `printf` conversion specifiers (`%s %d %n %099999d %%`) | printed **literally** — the C passes `line` as an *argument*, never as the format string | `g5_print_line_format_specifiers` | [x] |
| G6 | `printLine` | payload of arbitrary non-ASCII / **invalid UTF-8** bytes (`0x80..0xFF`, lone continuation bytes, truncated sequences) | bytes emitted verbatim; no validation, no replacement char, no panic | `g6_print_line_invalid_utf8` | [x] |
| G7 | `printLine` | payload containing embedded `\n`, `\r`, `\t`, `\x1b` | emitted verbatim, then the trailing `"\n"` | `g6_print_line_control_bytes` | [x] |
| G8 | `driver` | **out-of-range "enum" values** across the FFI boundary: `int` has no valid-variant restriction, so every one of `0, 1, -1, 2, 42, -42, i32::MIN, i32::MAX, i32::MIN+1, i32::MAX-1, 0x0001_0000, 0x7FFF_FFFE` is a real input | C truthiness: `!= 0` → `good()` → `"helperGood1 string\n"`; `== 0` → `bad()` → no output. In particular `i32::MIN` and every negative value select `good()` | `g8_driver_out_of_range_ints` | [x] |
| G9 | `driver` | one step past the "documented" range `{0,1}`: `-1` and `2` | both non-zero → `good()` | `g8_driver_out_of_range_ints` | [x] |
| G10 | `bad` / `good` / `driver` | called repeatedly (100×) and interleaved, to catch a translation that consumes/frees/mutates the `helperGood1` static on first use | every call reproduces its first-call output exactly | `g10_repeated_and_interleaved_calls` | [x] |
| G11 | `printLine` | value-dependent fuzz: 2000 randomized payloads (seeded LCG) over length 0..512 and the full byte range 0x01..0xFF | identical bytes from both `.so`s for every payload | `g11_print_line_random_fuzz` | [x] |

All rows above are checked off only because
`cargo test --test errors` passes with both `.so`s loaded via `libloading`.

## Cross-references

Rows E1-E3 and G8 are additionally re-run against the C source rebuilt at -O0,
-O1, -O2, -O3 and -Os by `tests/optlevels.rs` -- important here because E2
depends on how the C compiler resolves `helperBad`s undefined behaviour.
Row-by-row sensitivity is recorded in `FEATURES.md`; the overall gate is in
`VERIFICATION.md`.

## Deliberately not tested

`printLine` with a non-NULL but invalid pointer (freed, unmapped, or an
unterminated buffer) is undefined behaviour in the C, not a rejection: both
libraries would fault inside libc and there is no defined result to compare.
It is therefore out of scope rather than overlooked.
