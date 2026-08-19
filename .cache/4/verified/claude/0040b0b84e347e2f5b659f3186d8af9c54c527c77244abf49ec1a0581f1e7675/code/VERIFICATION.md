# Verification report

C ground truth: `c_src/src/main.c` (unmodified).
Rust translation: `src/imp.rs` (shared by the `cdylib` and the executable).

Reproduce everything with:

```sh
scripts/verify_all.sh --with-mutation
```

## Completion gate

| gate | status | evidence |
|---|---|---|
| `SYMBOLS.md`: `nm -D` shows 0 missing symbols in the Rust `.so`, 0 unresolved non-libc undefined symbols | **PASS** | `symbol diff EMPTY`; C exports `driver main`, Rust exports `driver main`; `ldd … \| grep -c "not found"` → `0`. Asserted by `symbol_parity_nm_defined_only`, `symbol_dlsym_both_libs`, `symbol_no_unresolved_in_rust_so` |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | **PASS** | 33 rows (C1–C32 incl. C26b), all `[x]`; ≈ 25 000 randomized `driver` calls and ≈ 2 700 randomized `main` invocations per run, fixed seeds |
| Phase C: every `ERRORS.md` row has a passing error-path differential test | **PASS** | 25 rows (E1–E25), each with a named `error_path_*` test comparing stdout **and** exit status/signal |
| All of the above under every feature combination | **PASS** | `[features]` is empty → 1 combination; verified for `--no-default-features`, default and `--all-features`, in **dev and release** (`scripts/check_all_features.sh` → `FEATURE MATRIX: all combinations pass`) |

`cargo test` → **54 passed, 0 failed** (+ the `harness = false` in-process test:
`20 217 driver calls compared, OK`), in both profiles.

## How the two implementations are compared

* Both are built as shared libraries and loaded with **`libloading`**; the Rust
  side is always called through its `#[no_mangle] extern "C"` exports, never
  directly. `examples/so_runner.rs` is the `libloading` host used as a
  subprocess (fresh stdin/stdout stream state per case, exactly like the C
  program, which calls `main` once per process); `tests/inprocess.rs` loads
  **both** libraries into one process and calls `driver` through the FFI
  boundary with `dup2` stdout capture.
* The two *executables* (CMake `add_executable` product vs the Rust bin) are
  compared as well, including exit status and terminating signal.
* Every child process has a 60 s watchdog, so a hang is a test failure, never a
  stalled suite.

## Divergences found and fixed (Rust changed, C never touched)

1. **Missing C ABI surface.** The translation was a binary-only crate: the C
   `.so` exports `driver` and `main`, the Rust crate exported nothing. Added a
   `cdylib` target sharing one implementation file, with
   `#[no_mangle] extern "C" fn driver(c_int)` and
   `#[no_mangle] extern "C" fn main() -> c_int`. Symbol diff is now empty.
2. **`SIGPIPE` divergence.** With a normal Rust `fn main`, `std`'s runtime sets
   `SIGPIPE` to `SIG_IGN`, so with a closed stdout pipe the C program died from
   signal 13 while the Rust one exited 0. The bin now uses
   `#![cfg_attr(not(test), no_main)]`, so the `#[no_mangle] main` *is* the ELF
   entry point — byte-for-byte and signal-for-signal identical to
   `int main()` (`config_c29_stdout_closed_pipe_sigpipe`, `error_path_e24`).
   Measured with a pipe whose read end is closed before the child runs:

   | program | result |
   |---|---|
   | C `c_src/build/driver` | killed by signal 13 |
   | Rust with a normal `fn main` | exit code 0 ← **divergence** |
   | Rust with `#![no_main]` (what we ship) | killed by signal 13 |
3. **`scanf` must not wait for EOF.** The original translation slurped stdin with
   `read_to_end`, so with a pipe that stays open after `42\n` the C program
   printed and exited while the Rust one blocked forever. `scanf_i32` now reads
   one byte at a time (matching `getc` on the C stream: digits plus the single
   lookahead byte that terminates the run) — `config_c26_main_must_not_wait_for_eof`
   fails within 3 s if this regresses.
4. **Test-harness bug that hid everything else** (worth recording): `cargo test`
   rebuilds the cdylib into `target/<profile>/deps/` but does **not** uplift it,
   so `target/<profile>/libdriver.so` can be an older build and the suite would
   have compared a stale `.so`. The harness now picks the freshest copy that
   really exports `driver`+`main` and asserts it is newer than the sources.

Behaviour deliberately **preserved** from the C, not "fixed":

* `scanf`'s return value is ignored, so *every* rejection (EOF, whitespace only,
  garbage, lone sign, read error) silently leaves `x == 0` and `main` returns `0`.
* glibc's `strtol` saturation on overflow followed by truncation into `int`:
  `99999999999999999999` → `LONG_MAX` → `ffffffff…`, and
  `-99999999999999999999` → `LONG_MIN` → `00000000…`.
* Silent truncation for values that fit a `long` but not an `int`
  (`2147483648` → `0x80000000`, `-2147483649` → `0x7fffffff`).
* `%d` is base 10 only: `0x10` yields `0`, `010` yields `10`.
* `printf` write failures are ignored (`/dev/full` → no output, exit `0`).

## Harness self-validation

Passing tests only mean something if they can fail:
`scripts/mutation_check.py` injects **21** behaviour-changing bugs into
`src/imp.rs` (wrong struct field values, big-endian images, uppercase hex,
swapped nibbles, missing newline, short output, `isspace` sets without `\v`/`\n`,
`+` rejected or treated as `-`, hex digits accepted, saturation/truncation
swaps, wrapping accumulation, slurping stdin to EOF, read errors not treated as
EOF, `main` returning 1) and requires the suite to fail for each one; it also
includes **2** deliberately equivalent mutants that must survive. Result:

```
MUTATION CHECK PASSED: 23 mutants behaved as expected
```

Each killed mutant is reported with the tests that caught it (e.g. "slurps stdin
to EOF" is caught only by `config_c26_main_must_not_wait_for_eof`, which is why
that row exists).

## Files

| file | purpose |
|---|---|
| `SYMBOLS.md` | Phase A symbol surface + parity proof |
| `ERRORS.md` | Phase C error-surface table (25 rows) + row→test mapping |
| `CONFIGS.md` | Phase B configuration surface (33 rows) + feature/profile matrix |
| `src/imp.rs` | the translation (shared by both crate targets) |
| `src/lib.rs`, `src/main.rs` | `cdylib` and executable views |
| `examples/so_runner.rs` | `libloading` host used by the differential tests |
| `tests/differential.rs` | Phase B/C/D tests (subprocess isolation) |
| `tests/inprocess.rs` | in-process `libloading` differential test (`harness = false`) |
| `tests/common/mod.rs` | artifact resolution, process running, watchdog, PRNG, assertions |
| `scripts/verify_all.sh` | one-shot Phase A→D reproduction |
| `scripts/check_all_features.sh` | feature power-set × profile matrix |
| `scripts/mutation_check.py` | harness self-validation |
