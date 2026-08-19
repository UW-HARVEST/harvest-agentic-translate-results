# CONFIGS.md — Configuration-surface table (valid inputs)

## Axes derived from the C source

The public API is the complete surface: `c_src/include/sieve.h` declares one
entry point, `void sieve(int start)`, and it *is* the lowest-level entry point —
there is no convenience wrapper and no internal helper below it, so "drive the
lowest level directly" and "drive the public API" are the same call here.

Axes the C code actually branches or depends on (from `src/sieve.c`):

| axis | values the C distinguishes | why (source evidence) |
|------|----------------------------|-----------------------|
| A. runtime options / modes / flags | **none** — no setters, no globals, no state, no `#ifdef` | the header exposes one function; the `.c` file has no file-scope variables |
| B. terminating residue `val % 10` | `== 9` → 1 iteration; `0..8` → `9 - (val%10)` more iterations; `-9..-1` and `0` (negative `val`, C truncating `%`) → **never** matches | `if (val % 10 == 9) break;` |
| C. sign of `val` | positive, zero, negative | C's `%` truncates toward zero, so the residue set differs per sign |
| D. magnitude / iteration count | 1 line, 2..10 lines, thousands of lines (negative start), ~2^31 lines (`INT_MIN`) | loop length is `9 - val` for `val < 9` |
| E. boundary values of the `int` domain | `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `9`, `10`, `INT_MAX-8` (`2147483639`, largest value ending in 9), `[INT_MAX-7, INT_MAX]` (overflow range) | domain of the single parameter |
| F. output byte format | `printf("%d\n", val)` — decimal, minus sign for negatives, `\n` terminator, no padding, no thousands separator | the only output statement |
| G. destination / buffering of `stdout` | regular file (fully buffered), pipe (fully buffered, partial writes), broken destination — set by the caller, not the library | `printf` goes through the shared libc `stdout` stream in both `.so`s |
| H. call sequencing | single call; repeated calls; C-then-Rust and Rust-then-C interleaving (statelessness) | no state exists, so this must stay true after translation |

Every row below is executed against **both** `.so` files loaded with
`libloading` (`dlopen` + `dlsym` on the exported `sieve`, never a direct Rust
call), capturing raw `stdout` bytes at the file-descriptor level, and comparing
the byte streams for equality. Rows marked *randomized* use a fixed-seed
SplitMix64 PRNG (seeds `0x5EED_00xx`, one per row) with many values per row; the
failing value is isolated and printed on divergence, so failures are
reproducible.

Comparison runs execute in a forked child writing to a pipe, bounded by a byte
cap and a wall-clock timeout (`tests/common/mod.rs`), because `sieve` can
legitimately emit ~2^31 lines and a *divergent* implementation may never
terminate; row C19 additionally drives the exported symbol in-process with a
regular-file `stdout`, and row C14 in-process through a pipe.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `sieve` | `val = 9`: positive, residue 9 → exactly one line, minimal loop | `c1_single_iteration_exact` | [x] |
| C2 | `sieve` | `val = 0`: zero, residue 0 → ten lines `0..9` | `c2_zero_start` | [x] |
| C3 | `sieve` | `val ∈ 1..=8` (all of them): positive sub-decade starts, residues 1..8 | `c3_first_decade_all` | [x] |
| C4 | `sieve` | *randomized* positive `val` with residue `9` (`val = 10k+9`, k random in `0..2·10^8`) → one line, large magnitude | `c4_random_positive_residue_nine` | [x] |
| C5 | `sieve` | *randomized* positive `val` with residue `≠ 9`, magnitude up to `INT_MAX-16` → 2..10 lines, exercises the decade carry (e.g. `…8 → …9`, `…0 → …9`) | `c5_random_positive_other_residues` | [x] |
| C6 | `sieve` | all ten residues at a fixed large positive decade (`2147483630..2147483639`) → decade boundary next to `INT_MAX` | `c6_top_decade_all_residues` | [x] |
| C7 | `sieve` | `val = 2147483639` = `INT_MAX-8`, largest input that terminates without overflow, residue 9 → one line | `c7_largest_terminating` | [x] |
| C8 | `sieve` | `val = -1`: negative, residue `-1` → 11 lines `-1..9` (floor-mod trap) | `c8_negative_one` | [x] |
| C9 | `sieve` | `val ∈ -20..=-1` (all): every negative residue class `-9..0` | `c9_negative_two_decades_all` | [x] |
| C10 | `sieve` | *randomized* negative `val` in `-2000..=-1` → long runs (up to 2010 lines), all residues, mixed digit widths (1..4 digits + sign) | `c10_random_small_negative` | [x] |
| C11 | `sieve` | *randomized* negative `val` in `-100000..=-20000` → ~100k-line runs, crosses digit-width changes (6→5→4→3→2→1 digits) and the sign change at 0, fills the libc buffer many times | `c11_random_large_negative` | [x] |
| C12 | `sieve` | digit-width transition shapes: `-10, -100, -1000, -10000, -9, -99, -999, -9999` → verify `%d` widths/sign at every decade boundary | `c12_digit_width_transitions` | [x] |
| C13 | `sieve` | *randomized* full-domain sweep restricted to bounded outputs (`val ≥ -3000`, `val ≤ INT_MAX-8`), 512 values, comparing complete byte streams | `c13_random_full_domain_bounded` | [x] |
| C14 | `sieve` | `stdout` is a **pipe** (fully buffered, 64 KiB kernel buffer, partial writes) instead of a regular file, `val = -5000` | `c14_stdout_is_pipe` | [x] |
| C15 | `sieve` | repeated / interleaved invocations: `sieve(3)` twice in a row, then C→Rust→C→Rust on the same fd (statelessness + no residual stream state) | `c15_repeated_and_interleaved` | [x] |
| C16 | `sieve` | append-to-existing-stream shape: several calls without restoring fd 1 in between, so both libraries' output is concatenated into one stream and compared as a whole | `c16_concatenated_stream` | [x] |
| C17 | `sieve` | overflow-range and `INT_MIN` configurations, compared as an output **prefix** (8 KiB) in a forked child because the full run is ~2^31 lines: `val ∈ {2147483640, 2147483647, INT_MIN, INT_MIN+1}` | `c17_unbounded_runs_prefix` | [x] |
| C18 | `sieve` | high-bit garbage in the 64-bit argument register (valid `int` after ABI truncation) — the lowest-level ABI shape | `e9_ffi_high_bits_ignored` (in `error_paths.rs`) | [x] |
| C19 | `sieve` | in-process call (no intervening `fork`) with `stdout` on a **regular file**, plus agreement between the in-process and forked capture paths; also compares the C output against an independent model of the loop, proving the capture really observes output | `c0_harness_sanity`, `d4_cargo_artifact_matches` | [x] |

## Verification evidence

```
$ ./verify_all.sh
=============== 2. feature combinations =================
Cargo.toml declares 1 feature combination(s) (empty line = no features):
  --no-default-features --features ''
=============== 3. cargo check per combination ==========
  OK   cargo check --no-default-features
  OK   cargo check --no-default-features --tests
  OK   cargo check --all-targets (default features)
=============== 4. differential suite per combination ===
  OK   cargo test --no-default-features  (33 tests passed)
  OK   cargo test --no-default-features --release  (33 tests passed)
VERIFY: ALL CHECKS PASSED
```

All 19 rows pass in both the dev and the release profile (release also changes
`opt-level` and sets `panic = "abort"` for the cdylib), single-threaded and with
the default parallel test harness.

### Why these rows are trustworthy (`./mutation_check.sh`)

Passing tests only mean something if they can fail. Ten deliberate mutations of
`src/lib.rs` were each rebuilt into the Rust `.so` and run through the suite;
every one was detected:

| mutation | detected by |
|----------|-------------|
| `rem_euclid(10) == 9` (floor-mod instead of C truncating `%`) | 9 tests |
| `saturating_add` instead of the two's-complement wrap | `c17_unbounded_runs_prefix` |
| check the residue before printing (drops the first line) | 18 tests |
| print `val + 1` instead of `val` | 18 tests |
| format string without `\n` | 18 tests |
| terminate on residue 8 instead of 9 | 17 tests |
| increment by 2 | suite timeout (non-terminating divergence) |
| `%u` instead of `%d` | 9 tests |
| `%ld` instead of `%d` | 9 tests |
| exported symbol renamed (`no_mangle` name changed) | 18 tests |

A stale-artifact trap was found and fixed while building this: `cargo test` does
**not** rebuild a `crate-type = ["cdylib"]` target, so the suite originally
compared against an outdated `.so` and a knowingly-broken Rust library passed.
The harness now uses the cargo artifact only when it is newer than `src/lib.rs`
and otherwise compiles a fresh cdylib with `rustc`; `d3_artifact_under_test_is_fresh`
guards this permanently.
