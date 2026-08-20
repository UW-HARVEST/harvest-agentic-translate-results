# Verification report

Differential verification of the Rust translation against the C ground truth in
`c_src/`. See `SYMBOLS.md` (Phase A/D), `CONFIGS.md` (Phase B) and `ERRORS.md`
(Phase C) for the surface maps that gate the test suite.

## How to reproduce

Everything below is wrapped up in one script:

```bash
cd translated_rust && ./verify_all.sh
```

Step by step:

```bash
cd translated_rust

# C reference artifacts (nothing under c_src/ is modified)
(cd c_src && mkdir -p build && cd build \
   && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)
./build_c_so.sh                       # -> c_build/libcdecisions.so

# Rust artifacts + the whole differential suite, for every feature combination
./check_features.sh

# Release-profile coverage (cargo test --release is impossible: the crate sets
# [profile.release] panic = "abort", which the unwinding test harness rejects)
cargo build --release
DRIVER_RUST_SO=$PWD/target/release/libdriver.so \
DRIVER_RUST_BIN=$PWD/target/release/driver \
  cargo test --offline
```

## Test inventory

| test binary | tests | what it covers |
|---|---:|---|
| `tests/symbols.rs` | 2 | `nm -D` symbol parity; proof the harness loads two *different* `.so`s |
| `tests/differential.rs` | 40 | Phase B — `CONFIGS.md` rows C1-C38, C42 |
| `tests/error_paths.rs` | 20 | Phase C — `ERRORS.md` rows E1-E20 |
| `tests/executable.rs` | 15 | `CONFIGS.md` C39-C41, C43-C45 and `ERRORS.md` E21-E26 (`driver` binary, end to end) |
| **total** | **77** | all passing, debug and release |

Every library test calls `process_decisions` **only** through `dlopen` +
`dlsym` on the two shared objects (`libloading`), so the `#[no_mangle]`
`extern "C"` wrapper and the ABI are under test, not just the inner Rust
functions. Each call compares the returned `int` **and** the complete post-call
buffer (plus 16 guard bytes past the region the C may touch).

## C-compiler robustness cross-check

`validate_sequence`'s `(bool*)sequence` cast is a strict-aliasing violation, so
the whole suite was re-run against C builds at every optimisation level and with
a second compiler, using the `DRIVER_C_SO` override:

```bash
for opt in O0 O1 O2 O3 Os; do
  gcc  -$opt -shared -fPIC -o /tmp/gcc$opt.so   c_src/src/lib.c
  DRIVER_C_SO=/tmp/gcc$opt.so cargo test --offline --test differential --test error_paths
done
for opt in O0 O2; do
  clang -$opt -shared -fPIC -o /tmp/clang$opt.so c_src/src/lib.c
  DRIVER_C_SO=/tmp/clang$opt.so cargo test --offline --test differential --test error_paths
done
```

| C build | differential | error paths |
|---|---|---|
| `gcc` (cmake default, no `-O`) | 39/39 | 20/20 |
| `gcc -O1` / `-O2` / `-O3` / `-Os` | 39/39 each | 20/20 each |
| `clang -O0` / `-O2` | 39/39 each | 20/20 each |

The Rust translation matches all seven C builds, so the behaviour being
replicated is not an artefact of one particular compilation.

## Divergences found and fixed

### 1. `validate_sequence` did not rewrite the caller's buffer

**`validate_sequence` did not rewrite the caller's buffer.** `c_src/src/lib.c`
lines 322-326 do

```c
bool *bools = (bool*)sequence;  /* Reuse buffer */
for (size_t i = 0; i < len; i++) {
    bool val = parse_bool(sequence[i]);
    bools[i] = val;
}
```

which aliases the caller's `char` buffer with a one-byte `_Bool` pointer and
overwrites every byte in `[0, len)` with `0` or `1`. `main` never looks at the
buffer again, so the mutation is invisible from the executable — but it is fully
observable by any FFI caller of the exported `process_decisions`, and it happens
even on the early-return rule-violation paths (`-10`, `-11`, `-12`).

The original translation built a private `Vec<bool>` and left the caller's bytes
untouched. Fixed in `src/decisions.rs` by performing the in-place map and
reading the parsed values back out of the buffer, exactly as C does. `src/lib.rs`
therefore exposes the parameter as `*mut c_char`, and `src/main.rs` passes a
mutable buffer. Regression coverage: `CONFIGS.md` row C35 plus the
whole-buffer comparison in every single `assert_same` call.

A negative-control run (deliberately reverting the fix) makes 25+ tests fail
with `post-call buffer mismatch`, confirming the harness detects it.

### 2. `SIGPIPE` disposition and panicking stderr writes

A C `main` inherits the process's **default** `SIGPIPE` disposition, so
`printf`/`fprintf` to a pipe with no reader terminates the process with signal
13. The Rust runtime installs `SIG_IGN` for `SIGPIPE` *before* `main` runs, and
`eprint!` panics when the write fails. Measured before the fix:

| condition | C | Rust (before) |
|---|---|---|
| stdout is a broken pipe, stdin `"0\n0\nyyn\n"` | killed by signal 13 | exit **0** (EPIPE swallowed) |
| stderr is a broken pipe, stdin at EOF | killed by signal 13 | exit **101** (`eprint!` panicked) |

Fixed in `src/main.rs` by (a) restoring `SIG_DFL` for `SIGPIPE` as the first
statement of `main` — via a direct `extern "C" { fn signal(...) }` declaration, so
no new dependency is introduced — and (b) replacing `eprint!` with a
`write_stderr` helper that ignores write errors the way `fprintf` does. Both
conditions now report signal 13 from the Rust binary too. Regression coverage:
`CONFIGS.md` rows C43-C45, which compare the raw wait status (exit code *and*
terminating signal), and which assert the C really is dying from SIGPIPE so the
tests cannot silently lose their teeth.

### 3. (not a bug, but worth recording) over-claimed `length`

The C reads only indices 0..2 for operations 0/1 and only `min(length, 32)`
bytes for operation 2, so a caller may legitimately pass an enormous `length`
with a tiny buffer. The `extern "C"` wrapper therefore computes the exact access
window instead of building a `length`-sized slice up front; with
`length == usize::MAX` the naive version would be unsound and would trip a std
debug assertion. Covered by `CONFIGS.md` row C42.

## Structural changes made to enable verification

* `Cargo.toml`: added a `[lib] crate-type = ["cdylib", "rlib"]` target so an
  external caller can `dlopen` the Rust code, an explicit empty
  `[features] default = []`, and `libloading = "0.8"` under `[dev-dependencies]`.
* `src/lib.rs` (new): `pub mod decisions;` plus the `#[no_mangle] extern "C"`
  wrapper. It performs the C's NULL check *before* touching memory and derives
  the exact number of bytes the selected operation may access, so it never forms
  a reference over memory the C would not have read either.
* `src/main.rs`: now consumes the library crate (`use driver::decisions;`),
  mirrors C's `input_buffer[len - 1] = '\0'` newline strip on a mutable buffer,
  restores the default `SIGPIPE` disposition, and writes to stderr without
  panicking on failure.
* `build_c_so.sh` (new): builds `c_src/src/lib.c` into a shared object with the
  same (unoptimised) flags cmake uses, without touching `c_src/`.
* `check_features.sh` (new): extracts the feature list from `Cargo.toml`,
  enumerates its power set, and runs `cargo check --all-targets`, `cargo build`
  and `cargo test` for each combination.

## Behaviours deliberately preserved (not "fixed")

* `parse_bool` maps every unrecognised byte — including `'\0'` and `0x80..0xFF`
  — to `false` instead of reporting an error.
* `configure_flags` computes the `flags` bitmask and never uses it.
* `apply_permissions`' `else if (read && write)` block falls through to
  `return 0` when `permission_value != 6`, which is unreachable.
* `validate_sequence`'s `if (transitions < 3) return 40;` is unreachable (rule 3
  already forces at least 3 transitions once `len >= 11`).
* `main` returns `0` even when `process_decisions` reports an error code.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` diff (C exports minus Rust exports) is **empty**;
      0 undefined non-libc/non-runtime symbols in the Rust `.so`.
- [x] Phase B: all 45 `CONFIGS.md` rows pass across randomized/exhaustive inputs.
- [x] Phase C: all 26 `ERRORS.md` rows have a passing error-path differential test.
- [x] Holds under **every** feature combination (`./check_features.sh`: F1
      `--no-default-features` and F2 default) and under both the `dev` and
      `release` profiles.
