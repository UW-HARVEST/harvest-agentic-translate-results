# VERIFICATION.md — completion gate

Reference: `c_src/src/main.c` (the C is always right).
Translation: `src/imp.rs` (logic) + `src/main.rs` (bin) + `src/lib.rs` (C ABI).

## How to reproduce everything

```sh
bash scripts/verify_all.sh      # 4 build configurations, all tests, fuzz, symbol diff
```

That script builds the C reference (executable via CMake, shared library via
`gcc -shared`), then for each of the 4 configurations
(`{default, --no-default-features}` × `{dev, release}`) runs
`cargo check --all-targets`, `cargo build` and `cargo test`, then a 2000-case
randomized end-to-end fuzz (`scripts/fuzz_diff.py`) against the release
binaries, and finally diffs `nm -D` between the two shared objects.

## Gate

- [x] **`SYMBOLS.md`** — `nm -D --defined-only` on the C `.so` gives exactly
      `{main, run}`; the Rust `.so` exports both with the same name and type.
      Symbol diff is **empty**; `nm -D -u` on the Rust `.so` shows **0**
      missing/undefined non-libc symbols (all remaining imports are glibc or
      libgcc unwind symbols). Enforced by `tests/symbol_parity.rs` (3 tests) and
      by the `dlsym` lookups in `tests/differential_ffi.rs`.
- [x] **Phase B** — every one of the 29 rows in `CONFIGS.md` passes, each with
      randomized inputs from a fixed seed where a value axis exists
      (`tests/differential_process.rs`: 15 tests, ~1800 randomized process runs;
      `tests/differential_ffi.rs`: 1 test, 370+ compared `run`/`main` calls
      through the `.so` exports).
- [x] **Phase C** — every one of the 30 rows in `ERRORS.md` has a passing
      error-path differential test (`tests/error_paths.rs`: 30 tests), each
      asserting the same *specific* sentinel (the value `scanf` parked in `x`,
      the exit code, or the killing signal), plus a `harness_detects_divergence`
      negative control. Generic boundaries covered: NUL/high/invalid-UTF-8
      bytes, empty and 100 000-byte inputs, values one step past every
      documented range (`INT_MAX+1`, `INT_MIN-1`, `LONG_MAX+1`, `LONG_MIN-1`,
      2^32), closed fd 0/fd 1, `EISDIR` stdin, reader-less stdout pipe. There is
      no enum in the API (`run` takes a plain `int`, `main` takes nothing), so
      the out-of-range-enum class degenerates to the full `int` range, which
      rows 21–23 cover at both extremes.
- [x] **All feature combinations** — `Cargo.toml` declares only `default = []`,
      so the complete set is `{default}` and `{}`; both were run in `dev` and
      `release` (4 configurations), each with the full test suite. All 4 report
      `ok` with 0 warnings.

## Divergences found and fixed during verification

| # | symptom | fix |
|---|---------|-----|
| 1 | The first translation read **all** of stdin (`read_to_end`) before parsing, so `yes 5 \| driver` or `driver < /dev/zero` never terminated, while the C exits immediately. | `scanf_i32_reader` now consumes stdin incrementally through a `BufRead`, exactly like C's `stdio` (CONFIGS row 29). |
| 2 | The Rust runtime sets `SIGPIPE` to `SIG_IGN`, so with a reader-less stdout the C died from signal 13 while Rust exited 0. | `c_main` restores the default `SIGPIPE` disposition before doing anything else (ERRORS row 28). |
| 3 | For a **seekable** stdin, glibc gives its read-ahead back by seeking to the logically consumed position when the process exits, so `{ driver >/dev/null; cat; } < file` printed the rest of the file for C but nothing for Rust. | `sync_stdin_position` reproduces the fix-up: `lseek` to `start + bytes logically consumed`, where the byte that terminated the conversion is *not* counted (it is `ungetc`-ed by `scanf`) — matching all 23 hand-picked and 200 randomized cases (CONFIGS row 28). |
| 4 | `cargo test` alone does not build the `cdylib`, so the FFI test could not find `libdriver.so` in a clean tree. | `tests/common/mod.rs::ensure_rust_so` builds it with `rustc --crate-type cdylib` when missing. |
| 5 | The exported `#[no_mangle] main` collided with libtest's entry point when the lib target was compiled as a test. | `[lib] test = false` plus `#[cfg(not(test))]` on the wrapper. |

## Confirmed-equivalent behaviours (checked against the C, not assumed)

* `scanf("%d")` out-of-range handling: glibc converts with `strtol` semantics
  (saturating at `LONG_MAX`/`LONG_MIN`) and stores the value **truncated to
  `int`** — e.g. `"2147483648"` → `INT_MIN`, `"99999999999999999999"` → `-1`,
  `"-99999999999999999999"` → `0`. It does *not* clamp to `INT_MAX`/`INT_MIN`.
* Signed overflow in `bedrooms += extra` wraps identically at `-O0`, `-O2`,
  `-O3` and `-O0 -fwrapv`; the Rust uses `wrapping_add`.
* `%.1f` on `bathrooms`: the value is always exactly `n.5`, and Rust's `{:.1}`
  agrees with glibc's round-half-to-even formatting of the exact binary value.
  `nan`/`inf` spellings are handled in `format_f64_1` for completeness.
* No locale dependence: the C never calls `setlocale`, so it stays in the `"C"`
  locale and always prints `.` as the decimal point.
* stdout buffering mode (pipe vs. file vs. `/dev/null`) changes only the syscall
  granularity, never the emitted bytes.
