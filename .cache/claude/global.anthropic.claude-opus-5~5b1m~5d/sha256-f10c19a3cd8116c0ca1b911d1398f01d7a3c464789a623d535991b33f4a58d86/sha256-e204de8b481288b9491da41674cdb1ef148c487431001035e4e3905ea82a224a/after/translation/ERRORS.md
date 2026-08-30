# Differential verification log — `c_src/src/main.c` vs. `translation/`

Ground truth: `c_src/src/main.c`, built exactly as `c_src/CMakeLists.txt`
prescribes (no `CMAKE_BUILD_TYPE`, i.e. no optimisation flags):

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                  # -> translation/target/release/driver
```

Comparison method: `translation/tests/differential.rs` execs **both binaries**
as subprocesses with identical arguments and asserts byte-identical stdout,
byte-identical stderr and an identical exit status (including death by signal).
Nothing is loaded as a library.

---

## 1. Mismatches found and fixed

### 1.1 `SIGPIPE` was ignored by the Rust program (real behavioural mismatch)

* **Symptom.** With a stderr (or stdout) pipe whose reader has already gone
  away, the C program is killed by `SIGPIPE` — `waitpid` reports
  *terminated by signal 13*, i.e. shell exit status 141. The Rust translation
  instead completed its `write_all`, ignored the `EPIPE`, and **exited 1**
  (or 0 on the success path).
* **Cause.** The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main`
  runs; a C program inherits the default `SIG_DFL`. The translation therefore
  differed in exit status on every broken-pipe input.
* **Reproduction (before the fix).**

  ```
  C   : socketpair, close peer, exec driver with fd2 -> killed by signal 13
  Rust: same setup                                   -> exit code 1
  ```
* **Fix.** `restore_default_sigpipe()` in `src/main.rs` calls
  `signal(SIGPIPE, SIG_DFL)` as the first statement of `main`.
* **Regression test.** `broken_stderr_pipe_terminates_both_the_same_way`.

### 1.2 `argv[0] == NULL` fallback (defensive; unreachable on Linux)

* While auditing the `argc != 2` branch I found the translation printed an
  empty program name if `args_os()` were empty, whereas C's
  `fprintf(stderr, "Usage: %s ...", argv[0])` with a NULL `argv[0]` makes glibc
  print `(null)`.
* Investigated with a raw `execv(path, {NULL})`: modern Linux rewrites an empty
  `argv` into a single empty string, so both programs actually observe
  `argc == 1` and `argv[0] == ""` and already agreed. The fallback was still
  changed to `"(null)"` so the two remain identical on a kernel that permits
  `argc == 0`.
* **Test.** `empty_argv_usage_message_matches` (raw fork/`execv` with an empty
  `argv`, stderr captured to a file). Both produce `Usage:  <seed>\n`.

---

## 2. Test-harness pitfalls that produced false mismatches

### 2.1 The usage message embeds `argv[0]`

`Usage: %s <seed>` echoes `argv[0]`, and the two executables necessarily live at
different paths, so a naive `Command::new(bin)` comparison reports a stderr
mismatch that is not a translation defect. The harness pins
`CommandExt::arg0("driver")` for **both** processes so stderr is directly
comparable. (This is the only normalisation applied anywhere; stdout, stderr and
status are otherwise compared raw.)

### 2.2 `cargo test -- --skip valid_` also skipped `invalid_*`

`--skip` matches substrings, and `invalid_` contains `valid_`. The
accept-and-compute tests were renamed to `fullrun_*` so the fast error-path
tests can be selected independently.

### 2.3 My own test initially mis-classified `-18446744073709551615`

I put it in the "invalid" list; the test then took ~9 minutes and still passed,
which is how the real behaviour surfaced: `strtoul` negates modulo 2^64, so
`-18446744073709551615` converts to **1**, glibc does *not* set `ERANGE`, and
the C program accepts it as seed 1. Likewise `-18446744069414584321` is accepted
as seed `UINT_MAX`. Both are now `fullrun_*` tests. No translation change was
needed — `libc_compat::strtoul_base10` already used `wrapping_neg`.

---

## 3. Non-behavioural changes (needed to make the suite runnable)

### 3.1 `[profile.dev] opt-level = 3`

`cargo test` drives the binary built with the **dev** profile
(`CARGO_BIN_EXE_driver` -> `target/debug/driver`). The workload is
2000 x 262144 x 100 ≈ 5.2·10^10 integer operations, so an unoptimised build made
the suite unusable. `opt-level = 3` was added to `[profile.dev]`. Overflow
checks stay enabled and every wrapping operation in the program is written
explicitly (`wrapping_mul`, `wrapping_add`, `wrapping_sub`, `wrapping_neg`), so
this changes speed only. `x / 2` and `x % 7` can never trap (only
`i32::MIN / -1` would).

### 3.2 Loop interchange in `perform_expensive_operations` (LANES = 8)

Each array element is transformed independently of every other, so the element
loop and the 100-step loop were interchanged over blocks of 8 elements to let
LLVM vectorise. Verified equivalent: seed 1 produces `42032659` with the
original scalar Rust, the blocked Rust, the `-O0` C build and the `-O3` C build.
Runtime for one full run dropped from 5 min 11 s to ~57 s (C at `-O0` takes
7 min 51 s).

---

## 4. Behaviours audited and confirmed already correct

None of these required a change; they are recorded because each one is a place a
translation typically goes wrong, and each is now covered by a test.

| C detail | Status |
|---|---|
| glibc `srand`/`rand` (TYPE_3 additive feedback, deg 31, sep 3, seed 0 -> 1, 310 discarded outputs) | matches for 133 seeds (boundaries + random), 262144 draws each |
| `(val >> 1) & 0x7fffffff` — arithmetic shift in Rust vs. logical in C | provably identical after the mask; verified empirically |
| `state[0] = seed` truncated to `int32_t` for seeds > 2^31 | covered by `fullrun_seed_above_int_max` and the seed sweep |
| signed overflow in `x*3+7`, `x-(x<<1)` (GCC wraps) | matches over 200019 values (incl. `INT_MIN`, `INT_MAX`) at `-O0` **and** `-O2` |
| `x >> 3` arithmetic shift for negative `x` | same sweep |
| `x/2 + x%7` truncating-toward-zero division/remainder | same sweep |
| `""` (empty argv[1]) is **accepted** as seed 0 — `strtoul` converts nothing, leaves `endptr` at the NUL, so `*endptr == '\0'` and `errno == 0` | `fullrun_empty_argument_is_seed_zero` |
| leading white space (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`) skipped, `'+'`/`'-'` sign accepted | `fullrun_leading_whitespace_and_plus`, `invalid_whitespace_only` |
| whitespace-only / sign-only / no-digit input leaves `endptr` at the *start*, so `*endptr != '\0'` -> invalid | `invalid_no_digits_at_all`, `invalid_whitespace_only` |
| trailing garbage (`12a`, `0x10`, `1 `, `1\n`, `  42  `) -> invalid | `invalid_trailing_garbage` |
| `ERANGE` for values above `ULONG_MAX` (returns `ULONG_MAX`), but **no** `ERANGE` for exactly `ULONG_MAX` | `invalid_erange_overflow`, `invalid_ulong_max_boundary_without_errno` |
| `temp_seed > UINT_MAX` check, boundary at 4294967295 / 4294967296 | `fullrun_seed_uint_max`, `invalid_above_uint_max` |
| order of checks: `argc` before seed validation | `argc_check_precedes_seed_validation` |
| `Invalid seed: '%s'` echoes `argv[1]` as raw bytes (invalid UTF-8, `%d`, control bytes) | `invalid_non_utf8_arguments`, `invalid_seed_message_echoes_argument_verbatim` |
| `printf("%d\n", xor_result)` — one line, no padding, trailing newline | every `fullrun_*` test |
| exit 1 on both error paths, exit 0 on success | all tests assert the status |

Additional bulk cross-checks (fast oracles built from the same C source and the
same Rust modules, kept out of the committed suite):

* 400083 argument strings drawn from a digit/sign/space/garbage/high-byte
  alphabet: identical accept/reject decision **and** identical resulting seed.
* 133 seeds x 262144 `rand()` draws: identical streams.
* 200019 `int` values x 100 transform steps: identical results, and `-O0` C ==
  `-O2` C == Rust (so the signed-overflow UB is stable across optimisation
  levels).

---

## 5. Result

No remaining differences. `cargo test` passes with no test ignored, skipped or
disabled, and nothing under `c_src/` was modified (only the `c_src/build/`
output directory that the documented CMake build creates).
