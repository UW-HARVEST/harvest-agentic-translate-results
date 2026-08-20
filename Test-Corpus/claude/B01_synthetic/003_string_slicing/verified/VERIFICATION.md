# Verification summary

C ground truth: `c_src/src/main.c` (one translation unit, one function: `main`).
Rust translation: `src/imp.rs` (the logic), `src/lib.rs` (`cdylib`, exports the C
symbol `main`), `src/main.rs` (the executable).

## How to reproduce everything

```sh
./scripts/verify.sh          # builds the C + Rust artifacts and runs the full matrix
```

The script

1. builds the C executable via `c_src/CMakeLists.txt` (default flags, i.e. `-O0`)
   **and** an optimized CMake build tree outside `c_src/`, plus the C shared
   library (`gcc -shared -fPIC -O2`);
2. enumerates every valid Cargo feature combination (there are no `[features]`,
   so the complete set is the single default combination) and `cargo check`s each
   with `--no-default-features --all-targets`;
3. diffs `nm -D --defined-only` between the C and the Rust `.so`;
4. runs all differential tests for the 4-way matrix
   `{rust dev, rust release} x {C -O0, C -O2}`.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` exports exactly `main`; the Rust
      `.so` exports `main` too. Symbol diff is empty; the Rust `.so` has 0
      undefined non-libc symbols (`ldd -r` clean). Nothing is stubbed: the whole
      C translation unit is translated.
- [x] **Phase B** — every one of the 30 rows of `CONFIGS.md` has a differential
      test that passes with randomized inputs from a fixed seed
      (`tests/differential_so.rs`, `tests/differential_cli.rs`).
- [x] **Phase C** — every row of `ERRORS.md` (E1-E7 plus the 15 generic boundary
      rows B1-B15) has a differential test that asserts the *same* status and the
      *same* message bytes (`tests/errors_so.rs`, `tests/crash_paths.rs`);
      B9 is the single row that is argued in code instead of executed, because
      triggering it needs a >2 GiB argument and >2 GiB of output.
- [x] **All configurations** — the whole suite passes for
      `{dev, release} x {C -O0, C -O2}`; 54 tests per configuration.

## Test layout

| file | what it drives | phase |
|---|---|---|
| `tests/common/mod.rs` | harness: `dlopen`s both `.so`s, builds `argv` vectors (contiguous / separate / aliasing), captures fd 1, forks for crash cases, fixed-seed PRNG, input generators | — |
| `tests/symbols.rs` | `nm -D` parity, `ldd -r`, both `.so`s callable | D |
| `tests/differential_so.rs` | `CONFIGS.md` C1-C20, C25, C26 through the exported `main` | B |
| `tests/differential_cli.rs` | `CONFIGS.md` C21-C23, C27-C29 through `execve` | B |
| `tests/errors_so.rs` | `ERRORS.md` E1-E7, B1, B2, B6-B8, B10 | C |
| `tests/crash_paths.rs` | `ERRORS.md` B3-B5, B11-B13 (NULL pointers, guard pages) | C |

Both libraries are always driven through their **exported** `main` symbol loaded
with `libloading`; no Rust function is ever called directly, so the
`#[no_mangle] extern "C"` wrapper is part of what is tested.

## Divergences found and fixed

| # | divergence | fix |
|---|-----------|-----|
| 1 | The first translation rebuilt `argv` from `std::env::args_os()`, so it could not reproduce `argv[1]` for `argc == 0`, and the `end == argv[3]` pointer comparison was hard-coded as "never taken" instead of actually being performed. The C *can* take that branch (it prints `"Third argument must be an integer!"`) when the caller's `argv[3]` points into `argv[2]`. | `src/imp.rs` now works on the raw `(argc, argv)` pair, so the comparison is the very same pointer comparison; `src/main.rs` captures the real process vector through a `.init_array` entry. Covered by `cfg_c19_layout_alias`, `err_e5_third_arg_alias`, `boundary_b1_argc0_ffi`. |
| 2 | The first translation called `std::process::exit()` from the middle of the program, so it could not be exercised through the FFI boundary and behaved differently from a C `main` that merely `return`s. | `imp::c_main` returns the status; the `cdylib` returns it to the caller, the binary passes it to `exit`. |
| 3 | `SIGSEGV` vs `SIGABRT` on the NULL-pointer paths: a plain `*ptr` deref is preceded by a Rust-inserted null check in builds with debug assertions, which aborts (signal 6); the C faults (signal 11). | `imp::load()` performs the byte load with `read_volatile`, which faults exactly like C in every profile. Covered by `boundary_b3_b4_b5_null_pointers`, `boundary_b11_null_argv`, `boundary_b13_unterminated_string_faults_identically`. |
| 4 | Broken pipe: the Rust runtime sets `SIGPIPE` to `SIG_IGN`, so `driver <long string> \| head -c 5` exited 0 while the C was killed by signal 13. | `src/main.rs` restores `SIG_DFL` for `SIGPIPE` before running. Covered by `cfg_c28_cli_broken_pipe`. |

## Behaviors of the C that are deliberately preserved (not "fixed")

* `if (start > len)` / `if (stop > len)` compare an `int` against a `size_t`, so
  **every negative `start`/`stop` is reported as "off the end of the string"**.
* `strtol` returns `long`; the assignment to `int` truncates modulo 2³², so e.g.
  `4294967296` behaves like `0` and `9223372036854775808` (which saturates to
  `LONG_MAX`) behaves like `-1`.
* `"Second argument must be an integer!"` and
  `"Third argument must be an integer!"` are printed **without** a trailing
  newline, and everything goes to **stdout**, never stderr.
* The third argument is parsed with a NULL `endptr`, so a non-numeric third
  argument silently yields `stop == 0` (and then usually trips
  `stop <= start`) instead of reporting an error.
* `stop = len` narrows `size_t` to `int`, and `printf("%.*s", ...)` treats the
  resulting negative precision as "no precision" — both reproduced in
  `imp::put_precision_str_nl`.
* With `argc == 0` the C reads `argv[1]`, one past the vector's NULL terminator.
