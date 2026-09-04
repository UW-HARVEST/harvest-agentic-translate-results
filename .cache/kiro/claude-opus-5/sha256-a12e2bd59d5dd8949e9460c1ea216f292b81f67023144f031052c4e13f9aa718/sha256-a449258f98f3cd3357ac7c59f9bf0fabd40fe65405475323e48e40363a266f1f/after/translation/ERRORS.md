# Differential verification of `translation/` against `c_src/`

Ground truth: `c_src/src/main.c`. The Rust program must produce byte-identical
stdout, byte-identical stderr, and the same exit status for every input.

## How it was verified

| Command | Purpose |
| --- | --- |
| `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | builds the reference, `c_src/build/driver` |
| `cd translation && cargo build --release` | builds `translation/target/release/driver` |
| `cd translation && cargo test` | runs the differential suite (both profiles pass) |

Both programs are driven as subprocesses over a pipe — never linked as a
library — because that is how the comparison is made.

Evidence gathered:

* **47 tests** in `tests/differential.rs` (39, enumerated branches) and
  `tests/fuzz_differential.rs` (8, randomized + process-level). Every test
  asserts all three of stdout, stderr and exit status. None is `#[ignore]`d,
  skipped or disabled.
* **~17,000 distinct inputs** total: exhaustive sweeps over `(length, rotate
  amount)`, `(length, split position)`, `(len1, len2)` merge/interleave pairs,
  all 256 byte values, all lengths `0..=256`, plus ~4,800 randomized inputs
  (structured, unstructured, and raw byte soup including NUL and controls).
* **Branch coverage of the C.** The corpus was replayed through a
  `gcc --coverage` build of `main.c` (compiled from a copy in `/tmp`; `c_src`
  was not touched). Result: **0 unexecuted lines and 0 one-sided branches in
  reachable code.** Every line gcov reports as unexecuted is provably
  unreachable from `main` — see the inventory at the bottom.

---

## Mismatch found

### 1. Exit status on a closed stdout (SIGPIPE disposition)

**Symptom.** With a reader that closes the pipe before the program finishes
writing, the two programs disagreed on how they terminated:

```
$ ./c_src/build/driver < big_input | (read -r _; exit 0)   # C
141                       # killed by SIGPIPE (128 + 13)

$ ./translation/target/release/driver < big_input | (read -r _; exit 0)
0                         # exited normally
```

Reproducing this needs an input whose stdout exceeds the pipe capacity, e.g.
100 buffers of 256 bytes (`op=1`, ~91.8 KB of output).

**Cause.** Not a translation error in the logic — a difference in process
startup. The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs, so a write to a closed pipe returns `EPIPE` instead of raising a signal.
`Out::flush` discards write errors (`let _ = ...`), exactly as C's implicit
`exit`-time flush does, so the failure vanished and the process exited with the
normal status. A C program starts with `SIGPIPE` at `SIG_DFL`, so the same write
kills it with signal 13.

**Fix.** `restore_default_sigpipe()` in `src/main.rs`, called first thing in
`main`, resets the handler to `SIG_DFL` via a bare `extern "C" { fn signal(...) }`
declaration (no new dependency). It is a no-op whenever stdout stays open, so
nothing else changes. Regression-tested by
`closed_stdout_matches_sigpipe_disposition`.

---

## Behaviors that look like bugs and were confirmed faithful

These were the highest-risk spots. Each is reproduced deliberately, was
suspected of being wrong, and was confirmed identical by direct comparison. They
are recorded so the next reader does not "fix" them.

### `scanf("%d")` saturates as `long`, then truncates to `int`

glibc converts the digit run as a `long`, clamping to `LONG_MAX`/`LONG_MIN` on
overflow, and only then stores the low 32 bits. The visible results are
surprising and both programs agree on all of them:

| Input | Parsed as `int` | Observable effect |
| --- | --- | --- |
| `1 99999999999999999999` | `-1` | `Error: Invalid buffer count -1` |
| `1 -99999999999999999999` | `0` | `Error: Invalid buffer count 0` |
| `1 2147483648` | `-2147483648` | `Error: Invalid buffer count -2147483648` |
| `4294967296 1 0` | `0` | runs `OP_COPY`, not "unknown operation" |
| `4294967297 1 0` | `1` | runs `OP_REVERSE` |

Boundaries `9223372036854775807/8` and `-9223372036854775808/9` were checked
individually, as were 5,000-digit runs and 5,000 leading zeros. Covered by
`scanf_overflow_saturates_then_truncates` and
`scanf_int_truncation_boundaries`.

### A negative split position becomes an enormous `size_t`

`main` reads `split_pos` as `int` and passes it to a `size_t` parameter, so the
value sign-extends *before* `buffer_split`'s `split_pos > src->length` check:

```
input:  3 1 3 1 2 3 -1
stderr: Error: Split position 18446744073709551615 exceeds length 3
input:  3 1 3 1 2 3 -2147483648
stderr: Error: Split position 18446744071562067968 exceeds length 3
```

The Rust reproduces this with `split_pos as i64 as usize` (not `as usize`, which
would widen from `i32` differently). Covered by
`op_split_negative_position_becomes_huge`.

### `buffer_rotate` checks `positions == 0` before normalizing

The early return tests the *raw* amount, so `positions == length` skips the
early return and performs a zero-length rotate — same output, different path.
C's `%` truncates toward zero, so negatives need the `+= length` correction.
`i32::MIN` is safe here only because `length` is always positive.
`op_rotate_all_amounts` sweeps every amount in `±(2·length+2)` for lengths
1, 2, 3, 5, 8 and 256, plus `INT_MIN`/`INT_MAX`.

### `OP_INTERLEAVE`'s error message has no format specifier

`Error: Interleaved length exceeds maximum` prints no length, unlike
`Error: Merged length %zu exceeds maximum` right above it. Asymmetric, and
correct. Covered by `op_interleave_length_boundary`.

### Validation happens after all input is consumed

An unknown operation is only diagnosed at the bottom of `main`, so buffer errors
surface first: `77 1 -5` reports `Error: Invalid buffer length -5`, not
`Error: Unknown operation 77`. Covered by
`unknown_operation_still_reads_all_buffers_first`.

### `%d` crosses newlines

Line structure is irrelevant to the whole program; `\n`, `\r\n`, `\t`, `\x0b`
and `\x0c` are all just separators. Verified with a full `isspace` alphabet, and
by feeding every maximum-size input twice, once space-separated and once
newline-separated (`maximum_size_inputs_for_every_operation`).

### Uninitialized destination buffers are observationally zeroed

`buffer_t temp;` / `buffer_t merged;` / `buffer_t part1, part2;` are
uninitialized in the C, and `init_buffer_array` uses `malloc`, not `calloc`.
Auditing every read confirms nothing ever touches `data[i]` for `i >= length`
(`calculate_checksum`, `write_buffer`, `validate_buffer` and all the copy
routines are bounded by `length`), so `Buffer::new()`'s zeroing cannot be
observed. Checked at every length `0..=256`.

### Trailing input is ignored

Once the required tokens are read, the rest of stdin is discarded — no
"unexpected trailing data" diagnostic exists. Covered by
`trailing_input_is_ignored`.

---

## Code gcov reports as unexecuted, and why it is unreachable

Confirming these are dead was necessary before claiming full coverage; none can
be reached through stdin, so none is observable.

* **All `NULL`-pointer guards** (`validate_buffer`, `buffer_copy`,
  `buffer_reverse`, `buffer_merge`, `buffer_split`, `buffer_interleave`,
  `buffer_rotate`, `read_buffer`, `write_buffer`). `main` only ever passes
  addresses of live objects. The Rust drops them, since a `&Buffer` cannot be
  null.
* **`validate_buffer`'s `buf->length > 256`.** `read_buffer` rejects any length
  above 256 first, so the guard can never fire.
* **`validate_buffer`'s `Warning: Checksum mismatch`.** `checksum` is always
  recomputed from `data` immediately before validation, so `expected` always
  equals it.
* **`init_buffer_array`'s `initial_capacity <= 0`.** `main` rejects
  `buffer_count <= 0` beforehand.
* **Both `malloc`-failure paths** and `main`'s `if (!buffers) return 1;`.
  Not reachable from input; a 100-element `buffer_t` array is ~26 KB.
* **`buffer_conditional_copy`, `buffer_copy_strided`, `process_buffer_array`.**
  Defined but never called — `main` implements its operations inline. Kept in
  the Rust behind `#[allow(dead_code)]` for structural parity. This is why
  `Error: Invalid stride %d`, `Error: Invalid buffer array` and
  `Error: Need at least 2 buffers for merge` never appear, even though
  `Error: Copy needs at least 2 buffers` (a separate string in `main`) does.
* **`main`'s `case OP_SPLIT` implicit else.** `if (buffer_count >= 1)` is always
  true, because `buffer_count <= 0` was already rejected.

## Status

All four phases pass: both programs build clean, every enumerated input produces
identical stdout, stderr and exit status, `cargo test` is green in debug and
release with nothing skipped, and `c_src/` is unmodified apart from the
`build/` directory the documented build command creates.
