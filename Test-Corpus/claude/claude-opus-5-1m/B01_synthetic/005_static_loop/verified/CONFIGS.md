# CONFIGS.md — configuration-surface table (Phase B)

Derived from `c_src/src/main.c` + `c_src/CMakeLists.txt`, the same way
`ERRORS.md` is derived. The library has no options struct, no flags and no
`#ifdef`s; the axes the C code actually branches on / is value-sensitive to are:

**A. Entry point (all public entry points, lowest level first)**

| id | entry point | signature | how it is driven in the tests |
|----|-------------|-----------|-------------------------------|
| `EP1` | `static_sum` | `int static_sum(int update)` | loaded from both `.so`s with `libloading`, called directly (lowest level, carries the `static int sum` state) |
| `EP2` | `main` | `int main(int argc, char **argv)` | loaded from both `.so`s with `libloading`, called with hand-built `argc`/`argv`, stdout captured by `dup2`-redirecting fd 1 |
| `EP3` | the `driver` program | process `argv` → stdout + exit status | both executables (`c_src/build/driver` from CMake and `target/<prof>/driver`) spawned as subprocesses |

**B. Runtime state axis** (`static int sum` is the only mutable state)

`S0` fresh library instance (`sum == 0`), `S1` state carried over from previous
calls, `S2` state pushed past `INT_MAX`/`INT_MIN` (wrapping).
Fresh state is obtained by copying the `.so` to a unique path before `dlopen`
(same path ⇒ same `dlopen` refcount ⇒ same `sum`).

**C. Input-shape axes for `argv[1]`** (what `strtol(…, 10)` distinguishes)
leading whitespace ∅/one/many/all-6-chars · sign ∅/`+`/`-` · digit count
1/2/…/19/200 · leading zeros · trailing garbage · in-`int`-range vs.
`int`-truncating vs. `long`-saturating · value making `i * stride` overflow ·
value making `sum += update` overflow.

**D. `argc` axis** `0`, `1`, **`2` (the only accepted one)**, `3`, `4…64`, negative.

**E. stdout axis** regular file · pipe · closed pipe (`SIGPIPE`) · `/dev/full`
(`ENOSPC`) · closed fd 1 (`EBADF`).

Every row below is compared **byte-for-byte between the C and the Rust build**
(stdout bytes + return value/exit status). Rows marked *(rand)* run many
randomized inputs from a fixed seed (`SplitMix64`, seed `0x5EED_C0DE_1234_5678`).

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `EP1` | `S0`, single call, `update = 0` | `ffi_static_sum::ffi_static_sum_single` | [x] |
| 2  | `EP1` | `S0`, single call, small positive / small negative / `1` / `-1` *(rand)* | `ffi_static_sum::ffi_static_sum_single` | [x] |
| 3  | `EP1` | `S0`, single call, boundary `update ∈ {INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1, 2^k, -(2^k)}` | `ffi_static_sum::ffi_static_sum_boundaries` | [x] |
| 4  | `EP1` | `S1`, 2 calls, mixed signs *(rand)* | `ffi_static_sum::ffi_static_sum_sequences` | [x] |
| 5  | `EP1` | `S1`, 10 calls (same count `main` uses) *(rand)* | `ffi_static_sum::ffi_static_sum_sequences` | [x] |
| 6  | `EP1` | `S1`, 1…64 calls, full-range random `i32` *(rand)* | `ffi_static_sum::ffi_static_sum_sequences` | [x] |
| 7  | `EP1` | `S2`, accumulation deliberately wrapping past `INT_MAX` and `INT_MIN` (repeated `INT_MAX`, repeated `INT_MIN`, `+2^30` ×16) | `ffi_static_sum::ffi_static_sum_overflow` | [x] |
| 8  | `EP1` | `S0` vs `S1` distinction itself: two independently `dlopen`ed copies must each start at 0 in *both* builds | `ffi_static_sum::ffi_state_is_per_instance` (+ `ffi_static_sum_is_process_wide_not_thread_local`) | [x] |
| 9  | `EP1`+`EP2` | interleaving: `static_sum` called before `main`, so `main`'s 10 lines start from a non-zero `sum` (checks the shared static, not a private one) | `ffi_main` "row 9" | [x] |
| 10 | `EP2`,`EP3` | `argc == 2`, `argv[1]` = plain small integer `"0"`, `"1"`, `"2"`, `"7"`, `"-1"`, `"-3"` | `cli_diff::cfg_small_strides`, `ffi_main` "row 10" | [x] |
| 11 | `EP2`,`EP3` | `argc == 2`, sign/whitespace/leading-zero shapes: `"+2"`, `"-0"`, `"+0"`, `"0000"`, `" 7"`, `"\t-4"`, `"\n\v\f\r 12"`, `"   +000123"` *(rand)* | `cli_diff::cfg_whitespace_sign_zeros`, `ffi_main` "row 11" | [x] |
| 12 | `EP2`,`EP3` | `argc == 2`, valid prefix + trailing garbage: `"5abc"`, `"0x10"`, `"3 4"`, `"7\n"`, `"12."`, `"9,9"` *(rand suffixes)* | `cli_diff::cfg_trailing_garbage`, `ffi_main` "row 12" | [x] |
| 13 | `EP2`,`EP3` | `argc == 2`, digit-count sweep 1…19 digits, both signs *(rand)* | `cli_diff::cfg_digit_count_sweep`, `ffi_main` "row 13" | [x] |
| 14 | `EP2`,`EP3` | `argc == 2`, 20…200 digit runs (far past `LONG_MAX`, exercises the saturation loop) *(rand)* | `cli_diff::cfg_long_digit_runs`, `ffi_main` "row 14" | [x] |
| 15 | `EP2`,`EP3` | `argc == 2`, full-range random `i32` strides *(rand, 400 values)* — covers `i * stride` and `sum += …` overflow | `cli_diff::cfg_random_i32_strides`, `ffi_main` "row 15" | [x] |
| 16 | `EP2`,`EP3` | `argc == 2`, `int` boundaries `2147483647`, `2147483648`, `-2147483648`, `-2147483649`, `4294967295`, `4294967296` (truncation of `long` → `int`) | `cli_diff::cfg_int_boundaries`, `ffi_main` "rows 16/17" | [x] |
| 17 | `EP2`,`EP3` | `argc == 2`, values around `2^31 / 2^32` multiples where truncation yields 0 (`4294967296`, `8589934592`, `-4294967296`) | `cli_diff::cfg_int_boundaries`, `ffi_main` "rows 16/17" | [x] |
| 18 | `EP2`,`EP3` | `argc == 2`, `LONG_MAX` exactly (`9223372036854775807`) | `cli_diff::cfg_long_boundaries`, `ffi_main` "rows 18-21" | [x] |
| 19 | `EP2`,`EP3` | `argc == 2`, `LONG_MAX + 1` (saturating, `ERANGE` ignored) | `cli_diff::cfg_long_boundaries`, `ffi_main` "rows 18-21" | [x] |
| 20 | `EP2`,`EP3` | `argc == 2`, `LONG_MIN` exactly (`-9223372036854775808`) | `cli_diff::cfg_long_boundaries`, `ffi_main` "rows 18-21" | [x] |
| 21 | `EP2`,`EP3` | `argc == 2`, `LONG_MIN - 1` (saturating) | `cli_diff::cfg_long_boundaries`, `ffi_main` "rows 18-21" | [x] |
| 22 | `EP2`,`EP3` | `argc == 2`, random 64-bit decimal values (in/out of `int` range) *(rand)* | `cli_diff::cfg_random_i64_strings`, `ffi_main` "row 22" | [x] |
| 23 | `EP2`,`EP3` | `argc == 2`, random byte-soup arguments (printable + high-bit bytes, accepted or rejected — whichever the C picks) *(rand, 400 values)* | `cli_diff::cfg_random_byte_soup`, `ffi_main` "row 23" | [x] |
| 24 | `EP2` | `argc == 2`, non-UTF-8 argument bytes (`"\xff7"`, `"7\xff"`, `"\xc3\xa9"`) — must not change the parse | `ffi_main` "row 24", `cli_errors::err_non_utf8_arg` | [x] |
| 25 | `EP2` | `argc == 2` with `argv` containing extra entries past `argv[1]` (must be ignored) | `ffi_main` "row 25" | [x] |
| 26 | `EP2` | repeated invocation of `main` on the same instance (`sum` keeps accumulating across runs — 3 back-to-back calls) | `ffi_main` "row 26" | [x] |
| 27 | `EP2`,`EP3` | rejected-input configurations, `argc == 2`: see `ERRORS.md` rows 6–15 | see ERRORS.md | [x] |
| 28 | `EP2`,`EP3` | `argc` sweep `0,1,3,4,…,64` (+ negative for `EP2`): see `ERRORS.md` rows 1–5 | see ERRORS.md | [x] |
| 29 | `EP3` | stdout = regular file (fully buffered) vs pipe (default in tests) — identical bytes | `cli_diff::cfg_stdout_file_vs_pipe` | [x] |
| 30 | `EP3` | stdout = closed pipe ⇒ killed by `SIGPIPE`; stdout = `/dev/full` ⇒ write error ignored, exit 0 | `cli_errors::err_epipe_kills_process`, `cli_errors::err_dev_full_ignored`, `cli_diff::cfg_stdout_closed` | [x] |
| 31 | `EP3` | `argv[0]` varied (long/odd program name) — must not affect output | `cli_diff::cfg_argv0_variation` | [x] |
| 32 | `EP3` | environment: empty env, `LC_ALL=C`, `LC_ALL=en_US.UTF-8`, `LC_NUMERIC=de_DE.UTF-8` (the program never calls `setlocale`, so `strtol` stays in the "C" locale) | `cli_diff::cfg_locale_env` | [x] |
| 33 | `EP2`,`EP3` | oversized arguments: 1 000 / 10 000 / 100 000-byte digit runs, whitespace runs, junk runs, digits + junk *(rand)* | `cli_diff::cfg_oversized_arguments`, `ffi_errors` "oversized …" | [x] |
| 34 | `EP3` | fd 1 closed before `exec` (`EBADF` on every write, ignored by the C) | `cli_diff::cfg_stdout_closed` | [x] |
| 35 | `EP2` | `main(argc, NULL)` for every `argc != 2` — the C never touches `argv` there | `ffi_errors` "argv == NULL" | [x] |

Build/feature configurations (Phase D): `Cargo.toml` has no `[features]`, and
`CMakeLists.txt` has no options, so there is exactly one combination
(`--no-default-features` ≡ default). Both the `dev` and the `release` Rust
profile are exercised, because `release` sets `panic = "abort"`.
