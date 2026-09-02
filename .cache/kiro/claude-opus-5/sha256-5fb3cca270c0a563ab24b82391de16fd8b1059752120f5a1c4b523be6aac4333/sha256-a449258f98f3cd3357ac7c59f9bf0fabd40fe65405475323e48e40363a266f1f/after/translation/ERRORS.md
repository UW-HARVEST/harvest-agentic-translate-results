# ERRORS.md — differential verification of `c_src/src/main.c` vs `translation/src/main.rs`

## Outcome

**No behavioural mismatch was found between the C program and the Rust
translation.** Every input class enumerated from the C source, plus 8,791
randomised fuzz cases, produced byte-identical stdout, byte-identical stderr and
an identical exit status.

Because this file exists to record what was *checked*, not merely what was
broken, the sections below list every place a mismatch was plausible, what the C
actually does there, and the evidence that the Rust matches it. An empty
"mismatches found" list is only meaningful alongside the list of things looked
for.

## How it was verified

- C binary: `c_src/build/driver`, built with
  `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  (no `CMAKE_BUILD_TYPE`, so no optimisation flags — this matters for the
  signed-overflow behaviour below).
- Rust binary: `cd translation && cargo build --release` →
  `translation/target/release/driver`.
- Test suite: `translation/tests/differential.rs`, 29 tests, each spawning both
  executables as subprocesses and comparing all three observables. No test is
  `#[ignore]`d, skipped or disabled. Passes under both `cargo test` and
  `cargo test --release`, and with `c_src/build/` either present or absent (the
  suite builds the C program itself if needed).
- Additional out-of-band fuzzing: exhaustive product of a 46-element argument
  pool plus 4,000 random-arity and 3,000 random-byte invocations
  (8,791 executions, 0 mismatches).
- `c_src/` was read only. The single filesystem addition under it is the
  `c_src/build/` CMake output directory, created by the build command the task
  prescribes; no tracked source file was touched
  (`c_src/src/main.c` md5 `bc307974bca17e9618bdc55bf57c50fe`,
  `c_src/CMakeLists.txt` md5 `88b0836e1b60d97bef2e41ef476e5044`).

## Risk areas examined, and why the Rust already matches

### 1. `int *` aliasing across a `static` local

`static_alias` returns either `&inner` (a function-scope `static`) or its own
`outer` argument, so the `int *running_sum` in `main` alternates between
pointing at `main`'s `initial_value` and at the static `inner`. The Rust models
the *pointee identity* with a `Target` enum plus a `Vars` struct instead of raw
pointers.

This is the one place a translation could silently diverge — e.g. by copying the
value instead of aliasing it, which would make `printf("%d", *running_sum)`
print the wrong object. Verified correct on both branches and across the
transition: `-10 20` and the `else_to_then_transition_boundary` sweep
(initial ∈ {-3,-2,-1,0} × iterations 1..7) exercise the iteration where the
pointer flips from `&initial_value` to `&inner`.

Note the `else` arm is only ever reached with `outer == Target::Initial`: once
the pointer aliases `inner`, `*outer >= inner` is `inner >= inner`, trivially
true. The Rust still writes the arm generically, which is harmless.

### 2. Signed integer overflow on `inner += *outer`

Once the pointer aliases `inner`, the value doubles every iteration and
overflows `int` within ~31 iterations. This is UB in C, but at `-O0` gcc emits a
plain `addl` and it wraps two's-complement. The Rust uses `wrapping_add`, which
reproduces that and — importantly — also prevents a debug-profile overflow
panic, so `cargo test` (debug) and `cargo test --release` agree.

Confirmed by `1 40`, `1 70`, `1073741824 6`, `715827883 8`, `1431655765 8`, and
`2147483647 5` (overflow on the very first addition). Also confirmed that once
`inner` wraps to `0` it stays `0` forever, since `0 >= 0` keeps taking the
then-branch (`2147483647 20`).

### 3. `strtol` accepted-input surface

The C only rejects input when `end == argv[n]`, i.e. when *no* conversion
happened. Everything else is accepted, including things that look like errors:

| input | C behaviour | covered by |
|---|---|---|
| `5abc`, `12.9`, `"5 5"` | trailing garbage ignored, prefix used | `trailing_garbage_is_accepted` |
| `0x10` | base 10, so parses `0` and stops at `x` | `base_ten_means_hex_prefix_is_a_zero` |
| `"  7  "`, `"\t\n 12"`, `"\x0b\x0c9"` | leading `isspace` skipped | `leading_whitespace_is_skipped` |
| `+5`, `-0`, `007` | sign and leading zeros accepted | `sign_and_leading_zero_forms` |
| `""`, `" "`, `"+"`, `"-"`, `"--5"`, `".5"` | no conversion → error path | `first_arg_not_an_integer` |

The Rust reimplements `strtol` rather than using `str::parse`, which would have
rejected every row in the top half of that table. `is_c_space` covers all six C
whitespace characters including the rarely-tested `\x0b` and `\x0c`.

### 4. `strtol` saturation, then implicit `long` → `int` truncation

`strtol` clamps out-of-range input to `LONG_MAX`/`LONG_MIN`; assigning to `int`
then truncates to the low 32 bits. The composition produces distinctly
non-obvious results that a naive translation gets wrong:

- `99999999999999999999` → `LONG_MAX` → `initial_value == -1`
- `-99999999999999999999` → `LONG_MIN` → `initial_value == 0`
- `4294967296` → `0`; `4294967297` → `1`; `4294967295` → `-1`
- `2147483648` → `INT_MIN`; `-2147483649` → `INT_MAX`

The last row is the sharpest: as an *iterations* argument, `-2147483649` looks
negative (loop should not run) but truncates to `INT_MAX`, so the C loops ~2^31
times. Covered by `out_of_range_saturates_then_truncates_to_int`,
`long_to_int_truncation_of_in_range_longs`, and — since the full output is
~4 GB — `iterations_truncating_to_int_max`, which compares a bounded 1 MiB
stdout prefix from both programs.

### 5. Order of validation

The C checks `argc`, then argument 1, then argument 2, returning immediately on
each failure. So `driver abc def` must print only the *first-argument* message.
Covered by `first_arg_check_happens_before_second`; a translation that validated
both arguments before reporting would fail it.

### 6. Output stream and exit status

All three messages go to **stdout** via `printf`, not stderr, and the error
paths `return 1` while success `return 0`. Every test asserts stderr is empty
and the status matches; `errors_go_to_stdout_not_stderr` additionally asserts
directly against the C that stderr stays empty, so the suite would notice if a
future change moved diagnostics to stderr on both sides at once.

### 7. Non-UTF-8 `argv`

`char **argv` is bytes, not text. The Rust reads `args_os` as raw bytes on Unix
rather than going through `String`, so `\xff\xfe` reaches the error path and
`5\xff` is accepted as `5` with trailing garbage, exactly as in C. Covered by
`non_utf8_arguments` and by the random-byte fuzz alphabet.

### 8. Loop bound and empty output

`for (int i = 0; i < iterations; i++)` produces nothing for zero or negative
iteration counts, and the program still exits 0 with completely empty stdout.
Covered by `zero_iterations_produces_no_output` and
`negative_iterations_produces_no_output`.

## Mismatches found

### In the translation

None.

### In the verification harness (recorded for the next reader)

My first fuzz driver capped iteration counts using Python's `int()` on the
second argument, then skipped anything above the cap. That model is wrong for
`-2147483649`: `int()` yields a negative number, so the case was not skipped,
but the C program's `long`→`int` truncation turns it into `INT_MAX` iterations.
The harness tried to buffer the resulting multi-gigabyte stdout and died with
`MemoryError`. The fix was to model `strtol` saturation *and* the 32-bit
truncation in the harness's cap computation, and to compare that input class via
a bounded prefix instead of a full capture.

This was a bug in my test scaffolding, not in either program, but it is exactly
the kind of input class that is easy to leave untested — the harness failure is
what surfaced it.

#### Harness bug 2: CMake race across parallel tests

`c_bin()` originally built the C program on demand from whichever test needed it
first. Integration tests run in parallel threads inside a single process, so on
a checkout where `c_src/build/` does not yet exist, all 29 tests invoked
`cmake` concurrently in the same directory and clobbered each other's temporary
files — 27 of 29 tests failed with a CMake configure error, and a couple with
"C build reported success but .../driver does not exist".

Note the shape of this failure: the suite passed whenever `c_src/build/` already
existed, so it looked green for the entire session and only broke on a clean
tree. Fixed by funnelling the build through a `std::sync::OnceLock`, so exactly
one thread builds and the rest wait for its result. Verified across all four
combinations of {clean, pre-built} × {`cargo test`, `cargo test --release`}.
