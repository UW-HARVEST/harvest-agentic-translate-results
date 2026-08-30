# Differential verification: `c_src` (ground truth) vs `translation`

## What is being compared

Both programs are built and driven as **subprocesses** over stdin; `stdout`,
`stderr` and the **exit status** are compared byte for byte.

| | command |
|---|---|
| C reference | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver` |

Test suite: `translation/tests/differential.rs` (24 tests, ~450 distinct
inputs). Run with `cd translation && cargo test`. The suite builds the C
reference automatically if `c_src/build/driver` is missing.

## The C program's behavior (what had to be replicated)

```c
void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len);
void driver(int *out, int len);   // fma_array(out, out, out, out, len), then prints
int  main();                      // reads up to 100 ints with scanf("%d")
```

Branch/input classes enumerated from the source:

1. `main` loop bound — `for (i = 0; i < 100; i++)`: 0, 1, 99, 100, >100 items.
2. `scanf("%d", &data[i]) != 1` → `break`. Two distinct failure modes:
   matching failure (non-numeric text) and input failure (EOF).
3. `fma_array` aliases **all four** pointers to the same array, so the
   computation is `out[i] = out[i]*out[i] + out[i]`, with signed overflow.
4. `driver`'s print loop runs `len` times with `printf("%d\n", ...)`.
5. `int data[100]` is uninitialized; only the first `i` elements are ever read,
   so the indeterminate tail is never observable.

## Outcome

**No behavioral mismatches were found between the C program and the Rust
translation.** Every enumerated input class produced identical stdout, stderr
and exit status. The sections below record the non-obvious C semantics that the
translation already handles correctly, since each one is a place where a naive
translation *would* diverge — these are the hazards this suite is guarding.

### H1 — `scanf` reads across newlines

`scanf("%d")` skips **any** leading whitespace, including newlines, so
`"1 2 3"`, `"1\n2\n3"` and `"\t1\v2\f3"` are all three items. A translation
built on `read_line`/`fgets` + `parse()` would treat lines as records and
mis-handle multiple values per line and values split across lines.
Covered by `scanf_crosses_newlines_and_mixed_whitespace`, `whitespace_only_input`.

The whitespace set must be C's `isspace`: space, `\t`, `\n`, `\r`, `\v`, `\f`.
Dropping `\r` (a plausible slip) is caught — verified by mutation M3 below.

### H2 — glibc `%d` converts with `long` semantics, then truncates to `int`

This is the subtlest part. glibc's `%d` parses the digit run into a `long`
(saturating at `LONG_MAX`/`LONG_MIN` on overflow, per `strtol`), then assigns
that `long` into an `int` object, which **truncates** to 32 bits. So:

| input | `long` value | stored `int` |
|---|---|---|
| `2147483648` | 2147483648 | `-2147483648` |
| `4294967296` | 4294967296 | `0` |
| `9223372036854775808` | saturates to `LONG_MAX` | `-1` |
| `-9223372036854775809` | saturates to `LONG_MIN` | `0` |
| `"9" * 10000` | saturates to `LONG_MAX` | `-1` |

A translation that clamps to `i32` instead of truncating, or that returns a
parse error on overflow, diverges here. `scan_int` in `src/main.rs` implements
exactly the saturate-to-`i64`-then-`as i32` sequence.
Covered by `values_beyond_int_range_truncate`,
`values_beyond_long_range_saturate_then_truncate`, `absurdly_long_digit_runs`.

### H3 — matching failure leaves the offending character unconsumed

`%d` accepts an optional sign followed by digits and stops at the first
non-digit, which must be pushed back (`ungetc`). Inputs like `0x10` → `0`,
`3.5` → `3`, `1e5` → `1`, `12abc` → `12`. A bare sign with no digit
(`-`, `+`, `- 5`, `--5`) is a matching failure that reads nothing.
Covered by `partial_numeric_forms_stop_at_first_non_digit`,
`sign_without_digits_is_a_matching_failure`, `non_numeric_input_stops_reading`.

Both `scanf` failure modes (`0` for matching failure, `EOF` for input failure)
hit the same `break` in `main`, so they are indistinguishable in the output —
the translation collapses them to one `break` for the same reason.

### H4 — the loop bound is the array bound

The loop stops at `i == 100`, **on the bound, not on a bad token**. With 101+
items the 101st is never consumed; with exactly 100 items followed by garbage,
the garbage is never even read. With 99 items followed by garbage, the 100th
`scanf` *does* run and fails, so `i == 99`. These two look similar but differ in
how many conversions are attempted.
Covered by `exactly_one_hundred_items_is_the_maximum`,
`more_than_one_hundred_items_ignores_the_excess`,
`hundred_items_followed_by_garbage`, `ninety_nine_items_followed_by_garbage`.

An off-by-one here panics in Rust (index out of bounds → SIGABRT, exit 134)
where C exits 0 — a mismatch in **exit status and stderr**, not just stdout.
This is precisely why the suite asserts all three channels; verified by M2.

### H5 — signed overflow must wrap

`out[i]*out[i] + out[i]` overflows for `|out[i]| > ~46340`. This is undefined
behavior in C, but the reference is compiled at `-O0` (the `CMakeLists.txt` sets
no optimization flags) and wraps two's-complement. The translation uses
`wrapping_mul`/`wrapping_add`. Plain `*`/`+` would panic in a debug build, and
`saturating_*` gives wrong values.
Covered by `int_boundary_values`, `overflow_wraps_the_way_c_does`,
`one_hundred_identical_extremes`, `randomized_value_sweep`.

### H6 — chunked stdin refills must not lose the pushback byte

`src/main.rs` reads stdin in 4096-byte chunks and `fill()` **clears** the buffer
on refill, so only one byte of pushback survives. Tokens straddling a chunk
boundary, and a non-digit terminator landing as the first byte after a refill,
are the risk cases. Exercised with whitespace padding of 4090/4093–4097/8190–8192
bytes plus a 10000-digit run that spans several refills.
Covered by `tokens_straddling_buffer_boundaries`,
`many_values_spanning_multiple_buffer_refills`, `absurdly_long_digit_runs`.

### H7 — the input is bytes, not UTF-8

Embedded NUL bytes, invalid UTF-8 (`\xff\xfe`), and non-ASCII digit lookalikes
(`５` U+FF10, `−` U+2212) are all just non-digit bytes to `scanf` and terminate
conversion. The translation is byte-oriented throughout, so it agrees.
Covered by `embedded_nul_bytes_terminate_conversion`, `non_utf8_and_high_bytes`.

## Suite quality check (mutation testing)

Passing tests only mean something if they can fail. Eight mutations were
injected into `src/main.rs`; each was reverted afterwards.

| # | mutation | result |
|---|---|---|
| M1 | clamp to `i32` instead of truncating (breaks H2) | **detected** |
| M2 | loop bound `100` → `101` (breaks H4) | **detected** |
| M3 | drop `\r`/`\v`/`\f` from the whitespace set (breaks H1) | **detected** |
| M4 | remove `ungetc` on the non-digit terminator (breaks H3) | **detected** |
| M5 | `v*v+v` → `v*(v+1)` | *survived — algebraically equivalent* |
| M6 | `writeln!` → `write!` (drops `\n`) | **detected** |
| M7 | reject a leading `+` sign | **detected** |
| M8 | `wrapping_*` → `saturating_*` (breaks H5) | **detected** |

7/8 detected. M5 is a true equivalent mutant: `v² + v == v·(v+1)` holds under
two's-complement wrapping for every `i32`, so no input can distinguish it. It
is not a coverage gap.

## Process errors made during verification (recorded for transparency)

- **Clobbered `translation/src/main.rs` with an unrelated file.** A mutation
  script wrote its backup to `/tmp/main.rs.bak`; that write failed
  (read-only sandbox), but a *stale* `/tmp/main.rs.bak` left over from an
  unrelated session already existed. The script's restore step then copied that
  stale file over the real translation, replacing it with a different program
  (`driver(int x) { y = 2*x; y += 300; }`). Detected by grepping for expected
  symbols (`fn fma_array`, `while i < 100`) and restored verbatim (213 lines).
  Fix: backups now go inside the writable working tree and are guarded by a
  content check before any mutation runs.
- **Race in the test harness' C build (a real bug, now fixed).** `c_bin()`
  built the C reference on demand. Cargo runs these 24 tests concurrently, so on
  a cold start (no `c_src/build`) all 24 threads invoked `cmake` in the same
  build directory at once and clobbered each other's cache, failing with
  "The C compiler identification is unknown" — all 24 tests failed for a reason
  that had nothing to do with the translation. Fixed by funnelling the build
  through a `OnceLock` so it happens exactly once per test process. Verified by
  deleting both `c_src/build` and `translation/target` and re-running cold.
- **Misread mutation results as compile errors.** The classifier grepped
  `^error`, which also matches cargo's trailing `error: test failed ...` line,
  so successful *detections* were reported as invalid mutants. Combined with a
  `head -25` truncation that cut off the failure summary, M1/M2/M4 briefly
  appeared to survive. Corrected classifier distinguishes `error[`/`error: could
  not compile` from `test result: FAILED`.

Neither affected the final result: `c_src/` was never modified, and the
translation on disk is the original, re-verified against the C reference.
