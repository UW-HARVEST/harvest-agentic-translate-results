# Verification report — StaticLoop C → Rust

Differential verification of `translation/` against `c_src/` as ground truth.
Both libraries are loaded as shared objects with `libloading` and driven only
through their exported C symbols, so the `#[no_mangle]`/`extern "C"` wrappers
are themselves under test. The Rust implementation is never called directly as
a Rust function.

## Result: PASS — no divergence found, and no change to `src/lib.rs` was needed.

54 differential tests, all passing, under every feature combination, both cargo
profiles, and against the C library compiled at five optimisation levels.

## How to reproduce

```sh
# 1. C reference library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust cdylib + the suite  (cargo test alone does NOT emit a cdylib)
cd translation && cargo build && cargo test

# 3. Phase D gates
./check_symbols.sh release     # symbol parity
./check_features.sh            # every feature combo x {debug, release}
./check_optlevels.sh           # C at -O0/-O1/-O2/-O3/-Os
```

## Phase A — artifacts

| artifact | content |
|---|---|
| `SYMBOLS.md` | `nm -D` inventory for both `.so`s; 2 symbols each, 0 missing |
| `ERRORS.md` | 17-row error/boundary-surface table, each mapped to a test |
| `CONFIGS.md` | 34-row configuration-surface table, each mapped to a test |

## Phase B — valid paths (`tests/phase_b_valid.rs`)

34 tests, one per `CONFIGS.md` row, `row01_*` … `row34_*`. Rows admitting a
value range are driven with randomized inputs from a fixed-seed SplitMix64
(`Rng::BASE_SEED = 0x5EED_1234_ABCD_0001`, salted per row) rather than one
hand-picked value. Both observable channels are compared: `static_sum`'s `int`
return value, and `driver`'s stdout **bytes**.

## Phase C — error paths (`tests/phase_c_errors.rs`)

17 `err*` tests, one per `ERRORS.md` row, plus 3 `generic_*` tests for the
generic FFI boundaries. Every assertion compares the concrete value or bytes
returned by each side — never merely "both failed".

## Phase D — parity, features, profiles

* **Symbol parity: empty diff.** C exports `driver` and `static_sum`; Rust
  exports exactly those two names. All of the Rust `.so`'s undefined symbols
  resolve against libc/libgcc.
* **Feature combinations: 1.** `Cargo.toml` declares no `[features]` section, so
  the default (empty) feature set is the only configuration.
  `check_features.sh` derives this mechanically from `Cargo.toml` rather than
  assuming it, and re-runs the suite for whatever it finds, under both `debug`
  and `release`. `release` matters here because it sets `panic = "abort"` and
  turns off the debug overflow checks.

## Notable findings

### 1. The library has no explicit error surface (documented, not assumed)

Grepping `staticloop.c` for error-return macros, sentinels, `assert`, `errno`,
range checks, null checks, and allocation yields **zero hits**. Both entry
points take one `int` by value and cannot fail; every `int` is a valid
successful return of `static_sum`, so there is no reserved sentinel. Phase C
therefore targets the *implicit* surface — the signed-overflow boundaries — and
`ERRORS.md` records the grep results so the claim is auditable rather than a
happy-path assumption.

### 2. Signed-overflow UB: verified against five optimisation levels

`sum += update` and `i * stride` are signed-overflow UB in C, and most
`ERRORS.md` rows sit exactly on those boundaries. The Rust uses
`wrapping_add`/`wrapping_mul`. Agreeing with the default CMake build would prove
little, because that build passes no `-O` flag (`-O0`), where a compiler has no
reason to exploit the UB. `check_optlevels.sh` builds the C source out-of-tree
at `-O0`, `-O1`, `-O2`, `-O3` and `-Os` and re-runs all 54 tests against each
(via the `STATICLOOP_C_SO` override). **All five agree with the Rust**, so the
wrapping choice is correct rather than an artifact of an unoptimised build.

### 3. The static accumulator required a per-test fresh-instance mechanism

`static_sum`'s accumulator is a function-scope `static int`, i.e. per-loaded-
object mutable state that persists across calls. glibc deduplicates `dlopen` by
`(st_dev, st_ino)`, so loading the same path twice returns the *same* object
with its accumulator already mutated — tests would have been order-dependent and
mutually contaminating. The harness copies each `.so` to a uniquely named
temporary file (a real copy, hence a distinct inode) and loads that, giving each
test a genuine `sum == 0` instance. State-carrying rows are then exercised
deliberately (rows 14–17, 29–32) rather than accidentally.

### 4. `driver`'s output is on fd 1, and capturing it demanded serialisation

`driver` writes via C `printf`, so its observable output is on file descriptor
1, not Rust's `std::io::stdout`. The harness captures it by redirecting fd 1 to
a scratch file. Two real hazards surfaced during bring-up, both of which
initially produced a spurious failure:

* libtest also writes progress to fd 1 from the main thread, so a concurrent
  test's `ok` was captured *inside* `driver`'s output. Fixed by forcing
  single-threaded execution (`.cargo/config.toml` sets `RUST_TEST_THREADS=1`) and
  by *enforcing* it in `require_serial_execution()` — a contaminated capture
  could otherwise hide a genuine divergence.
* Rust's stdout is a `LineWriter` holding partial lines (`test row32 ... `) with
  no trailing newline; those bytes were flushed into the capture. Fixed by
  flushing both Rust's stdout and C stdio before installing the redirect.

The Rust translation's decision to call C's `printf` (rather than
`std::io::stdout`) is what makes byte-identical, correctly-interleaved output
possible, and is validated by row 33's byte-shape assertions.

### 5. The suite was mutation-tested to prove it is not vacuous

An all-passing differential suite is worthless if it cannot fail. Five mutants
were injected into `src/lib.rs` and every one was caught:

| mutant | tests failed |
|---|---|
| `driver` loop bound `10` → `9` | 27 |
| `static_sum` `wrapping_add` → `saturating_add` | 37 |
| `driver` product `i*stride` → `i*stride + 1` | 37 |
| `printf` format `%d` → `%u` | 37 |
| initial `sum` `0` → `1` | 54 |

`src/lib.rs` was then restored to its original state and re-verified.

## Caveats

* **Thread safety is intentionally absent, matching the C.** The C
  `static int sum` is unsynchronised, so concurrent calls are a data race in C;
  the Rust mirrors this exactly (`UnsafeCell` + `unsafe impl Sync`, no locking).
  This is faithful to the ground truth but is not a differential-testable
  property, so no test asserts it.
* **`cargo test` alone does not build the cdylib.** The crate declares only
  `crate-type = ["cdylib"]`, and cargo does not emit it for a test build, so
  `cargo build` must run first. The harness fails with an explicit message
  rather than silently skipping.
* Verification covers `x86_64` System V. The argument-truncation rows (9, 15,
  18) are ABI-specific in the sense that they rely on the callee reading only
  the low 32 bits of the argument register; the *conclusion* (both sides agree)
  is what is asserted, not a particular truncation rule.
