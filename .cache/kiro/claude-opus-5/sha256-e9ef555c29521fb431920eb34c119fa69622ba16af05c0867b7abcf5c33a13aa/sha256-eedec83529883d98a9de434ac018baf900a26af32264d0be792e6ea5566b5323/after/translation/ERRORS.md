# Differential verification log

C ground truth: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Rust under test: this crate's `driver` binary.

Both are driven as executables: `driver < input`, comparing stdout bytes,
stderr bytes and exit status. Nothing is loaded as a library.

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> target/release/driver
cd translation && cargo test                                            # runs the differential suite
```

`c_src/` was not modified. Only the generated, untracked `c_src/build/` tree was
added.

---

## Mismatches found

### 1. Broken pipe produced a different termination status (fixed)

**Symptom**

```
$ big_input | ./c_src/build/driver           | head -c 16   ->  wait status 141
$ big_input | ./translation/target/release/driver | head -c 16 -> wait status 134
```

**Cause.** The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs.
A write to a closed pipe therefore returns `EPIPE` instead of killing the
process, `print!` panics on the failed write, and `panic = "abort"` (set in
`[profile.release]`) turns that into `SIGABRT` — wait status 134. The C program
never touches the signal disposition, so it is killed by `SIGPIPE` itself — wait
status 141.

This is only observable when the consumer of stdout stops reading before the
program is done writing, which needs more output than one pipe buffer. It is
reachable in practice: `OP_REVERSE`/`OP_ROTATE`/`OP_CHECKSUM` with 100 buffers
of 256 bytes emit roughly 100 KB.

**Fix.** `restore_default_sigpipe()` in `src/main.rs` resets `SIGPIPE` to
`SIG_DFL` as the first statement of `main`, via a direct `extern "C" { fn signal }`
declaration so no new dependency is introduced.

**Regression test.** `stdout_reader_closes_early_yields_same_wait_status` runs
each binary in a `bash` pipeline into `head -c 16` and compares
`${PIPESTATUS[0]}`. It also pins the expected value to `141`, so the test fails
if either program stops dying from the signal. Removing the fix makes exactly
this test fail (verified).

### 2. Test-side overflow, not a translation defect (fixed in the test)

`scanf_out_of_range_saturates_then_truncates` originally built boundary values as
`(1i128 << p) + off`. For `p == 127` that is `i128::MIN`, so `+ off` and the
later negation overflowed. Release builds wrap silently and the test passed;
`cargo test` (debug) panicked with "attempt to add with overflow". The failure
was in the test's own arithmetic, so the fix was to build magnitudes in `u128`
and apply the sign textually. Recorded here because the symptom — "passes in
release, fails in debug" — otherwise looks like a translation bug.

---

## Behaviors checked and found already faithful

These are the places a translation of this program is most likely to drift. Each
was probed directly and matched; they are listed so the next reader can re-check
rather than re-derive.

**`scanf("%d")` conversion.** glibc converts through `long`, saturating at
`LONG_MAX`/`LONG_MIN`, then narrows to `int`. So `9223372036854775808` and a
400-digit run of nines both yield `-1`, and their negations both yield `0`. The
scanner in `src/scan.rs` reproduces this by accumulating the magnitude in `u128`
with a `2^80` cap (any value reaching the cap necessarily exceeds `i64::MAX`, so
capping cannot change the saturated result), saturating to `i64`, then casting to
`i32`. Verified against both `int` and `long` boundaries ±1 and against 9000-digit
tokens.

**Reading across newlines.** `%d` skips the C-locale `isspace` set
(`' ' \t \n \v \f \r`), so tokens may be separated by anything and a conversion
spans line boundaries. Verified with newline-, tab-, CRLF- and mixed-separated
input, and with 30 000-byte whitespace runs that force the scanner to refill its
8 KiB buffer. Also verified for tokens straddling offsets 8180..8200, where the
refill lands mid-token.

**Field truncation.** `buf->data[i] = (uint8_t)byte` is a plain narrowing cast:
`256 -> 0`, `257 -> 1`, `-1 -> 255`. Likewise `operation`, `buffer_count` and
`length` are `int`, so `4294967296` reads back as `0` — an out-of-range length
token can therefore become a *valid* length rather than an error. Verified.

**`int` -> `size_t` at the split call.** `buffer_split` takes `size_t split_pos`
but main reads an `int`, so `-1` arrives as `18446744073709551615` and `%zu`
prints it verbatim:
`Error: Split position 18446744073709551615 exceeds length 4`. The Rust side
reproduces this with `split_pos as isize as usize`. Verified for `-1`, `-1000`
and `INT_MIN`.

**Order of validation.** The operation number is validated *last* — after the
buffer count and after every buffer has been read. So `7 1 999` reports
`Error: Invalid buffer length 999`, not `Error: Unknown operation 7`. Verified.

**Boundary conditions on the 256-byte cap.** `length > 256` is rejected but
`length == 256` is accepted. `buffer_merge` and `buffer_interleave` reject
`len1 + len2 > 256`, so a sum of exactly 256 succeeds. All sums around the
boundary were swept.

**`buffer_rotate` normalization.** `positions % (int)buf->length`, then
`positions += buf->length` if negative — where the `+=` is evaluated in `size_t`
and converted back to `int`. Every rotation amount in `-len-2 ..= len+2` was
swept for lengths 0, 1, 2, 3, 255, 256, plus `INT_MIN` and `INT_MAX`.

**`calculate_checksum` overflow.** `sum = (sum << 3) ^ data[i]` on `uint32_t`:
bits shifted past bit 31 are discarded. Swept over every length 0..=256.

**Uninitialized C locals are not observable.** `buffer_t temp`, `merged`,
`interleaved`, `part1`, `part2` in `main` are uninitialized stack objects, but
each operation writes `length` bytes and then sets `length`, and `write_buffer`
prints only that prefix. Modelling them as zeroed buffers is therefore
observationally identical.

**Unreachable C error paths.** The `NULL`-pointer checks, the `malloc`-failure
messages, `Error: Invalid capacity`, `Error: Buffer length %zu exceeds maximum 256`
and `Warning: Checksum mismatch` cannot fire from `main`: references cannot be
null, capacity is validated before the call, `read_buffer` caps the length, and
the checksum is always recomputed from the data it describes. `process_buffer_array`,
`buffer_conditional_copy` and `buffer_copy_strided` are dead code in the C too.
Consequently no reachable code path writes to both stdout and stderr, so the
difference between C's block-buffered stdout and Rust's line-buffered stdout is
not observable even when the two streams are merged.

**stdout / stderr / exit status are compared separately and byte for byte**, so
a Rust program that printed the right bytes but exited `0` where the C exits `1`
would fail. Every error-path test asserts all three.

---

## Coverage

`tests/differential.rs`, 34 tests, no `#[ignore]`, no `#[should_panic]`, nothing
skipped. Passes under both `cargo test` and `cargo test --release`.

Enumerated input classes: empty input; whitespace-only input; unreadable
operation / buffer count / buffer length / byte / split position / rotation
amount; buffer count `<= 0`, `> 100`, and the boundaries 1 and 100; buffer length
negative, `> 256`, and the boundaries 0, 1, 255, 256; byte truncation; all seven
operations plus unknown ones; the "needs at least 2 buffers" arms of copy, merge
and interleave; merge and interleave length caps; split at 0, mid, `len`,
`len+1`, and negative; rotate by 0, `len`, `> len`, negative, `INT_MIN`,
`INT_MAX`, and on an empty buffer; `argv` present; stdin closed rather than
empty; stdout reader closing early.

Exhaustive sweeps: every split position and rotation amount over a
representative set of lengths; every buffer length 0..=256 for reverse and
checksum; every buffer count 1..=100 against all eight operation arms; two-buffer
length pairs across the cap; every byte-prefix of five valid inputs; and the
maximum workload (100 buffers x 256 bytes) for every operation.

Additionally, an out-of-band Python differential fuzzer was run against both the
release and debug Rust binaries — ~4000 randomized inputs plus fully exhaustive
sweeps of the merge/interleave length-pair space and the split/rotate position
spaces. Zero mismatches. Running the debug binary matters independently: debug
builds panic on arithmetic overflow, so a clean debug run rules out overflow in
the translation's own index and length arithmetic.

The suite was mutation-tested. Seven deliberate defects injected into the Rust
(split comparison off-by-one, merge cap off-by-one, length bound off-by-one,
dropped sign handling in the scanner, wrong checksum shift, removed `SIGPIPE`
restore, extra space in `write_buffer`) were each detected.
