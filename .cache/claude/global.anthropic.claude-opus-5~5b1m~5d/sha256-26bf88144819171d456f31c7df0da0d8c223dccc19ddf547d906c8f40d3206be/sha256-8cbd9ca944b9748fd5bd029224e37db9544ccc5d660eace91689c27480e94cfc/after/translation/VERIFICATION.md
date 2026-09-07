# Verification report — C `Sieve` vs. Rust translation

## Scope of the library

The entire C library is one function:

```c
void sieve(int val) {
    while (1) {
        printf("%d\n", val);
        if (val % 10 == 9) break;
        val++;
    }
}
```

One header, one `.c` file, one exported symbol, one branch, zero error returns.
`SYMBOLS.md`, `ERRORS.md` and `CONFIGS.md` were derived mechanically from that
source (grep transcripts included in each file).

## Result: PASS — the Rust `.so` matches the C `.so` byte-for-byte

No divergence was found; `translation/src/lib.rs` required **no behavioural
changes**. `cargo check` was clean on the first run. Everything below was
verified through `libloading`: both `.so`s are `dlopen`ed and their exported
`sieve` symbol is called, so the `#[no_mangle] extern "C"` wrapper is under test
too. The Rust functions are never called directly.

## How outputs are compared

`sieve` returns `void`; its only observable effect is the byte stream it writes
to stdout through `printf`. The harness (`tests/common/mod.rs`) therefore:

* redirects fd 1 to a temp file around each call and compares the resulting
  bytes exactly (`assert_same_bytes`);
* for the inputs whose C output is effectively unbounded (`INT_MIN`, and the
  8-value signed-overflow window at `INT_MAX`, ≈2^32 lines) runs the call in a
  **forked child** and compares a bounded 256 KiB stdout prefix, asserting the
  prefix is actually full so the comparison cannot pass vacuously;
* additionally compares behaviour when stdout is a **pipe** and when stdout is a
  **closed fd** (comparing child exit status, since every `printf` then fails and
  the C code ignores the return value);
* runs a **watchdog** that aborts with an attributed message if any single
  in-process call fails to return within 25 s, so a mistranslation that turns a
  terminating C loop into an infinite Rust loop is reported as a failure instead
  of silently hanging the run.

## Gate status

| gate | status | evidence |
|------|--------|----------|
| `SYMBOLS.md`: 0 missing / 0 undefined non-libc symbols in Rust | **PASS** | `diff` of `nm -D --defined-only` on both `.so`s is empty (debug and release); `ldd -r` reports no undefined symbols |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | **PASS** | 22/22 rows, `tests/valid_paths.rs`, fixed SplitMix64 seed |
| Binary/driver stdout comparison | **N/A** | neither project builds an executable (`CMakeLists.txt` has only `add_library`; `Cargo.toml` has only `[lib] crate-type=["cdylib"]`). The library's whole stdout is compared instead |
| Phase C: every `ERRORS.md` row has a passing error-path test | **PASS** | 14/14 tests, `tests/error_paths.rs` |
| Holds under every feature combination | **PASS** | `./check_features.sh` — no `[features]` table exists, so default / `--no-default-features` / `--all-features` were all built, symbol-diffed and tested |

Totals: **38 tests, 0 failures** (22 Phase B + 14 Phase C + 2 harness smoke).
Also re-run green against the **release** `.so` (optimized, `panic = "abort"`)
via `RUST_SIEVE_SO=target/release/libSieve.so`.

## Behaviours of the C code that the translation reproduces deliberately

These are the places a naive translation would drift; each has a dedicated test.

1. **C truncated modulo, not Euclidean.** `val % 10` for negative `val` yields
   `0..-9`, never `9`, so the loop's exit test is *unreachable while negative*.
   `sieve(-9)` prints `-9 … 9` (19 lines), **not** one line. Using
   `rem_euclid` or `abs()` breaks 10 and 6 tests respectively.
2. **Signed overflow at `INT_MAX`.** `2147483647 % 10 == 7`, so C executes
   `val++` on `INT_MAX` — UB in C, and the compiled `-O0` C wraps to `INT_MIN`
   and keeps counting. Rust uses `wrapping_add(1)` to match; the test pins the
   observed C stream `2147483647\n-2147483648\n-2147483647\n…`. A plain `val + 1`
   panics and a `saturating_add` hangs — both are caught.
3. **The exit boundary is sharp at `INT_MAX-8`.** `2147483639` terminates in one
   line; `2147483640` never terminates. Both sides of the boundary are tested.
4. **Platform `printf`, not Rust formatting**, so field width, `%d` semantics and
   stdout buffering produce an identical byte stream.

## Test-suite power (mutation check)

Passing tests only mean something if they would also fail on a wrong
translation, so `./mutation_check.sh` injects 12 plausible mistranslations into
a pristine `src/lib.rs`, one at a time, and requires the suite to fail on each.
Result: **12/12 detected** (`ALL MUTANTS DETECTED`), covering Euclidean modulo,
`abs()` before the modulo, panicking / saturating / by-two / i64 increments,
off-by-one in the comparison and in the start value, testing the incremented
value, `% 100`, a missing newline, and `%u` instead of `%d`. The script restores
`src/lib.rs` on exit.

## Reproducing

```bash
# 1. C reference library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust library + differential suite
cd translation && cargo build && cargo test -- --test-threads=1

# 3. Every feature combination, with per-combination symbol parity
./check_features.sh

# 4. Confirm the suite can actually detect divergence
./mutation_check.sh
```

`--test-threads=1` is required: fd 1 is process-global, and the harness
redirects it. (The harness also takes an internal lock, so parallel runs are
serialized rather than corrupt, but single-threaded keeps the output readable.)
