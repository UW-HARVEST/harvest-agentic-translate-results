# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

## Build-time configurations

* `c_src/CMakeLists.txt` declares no `option()`, no `add_definitions` and no
  `target_compile_definitions`; `c_src/src/main.c` contains **no `#ifdef`**
  (`grep -c '#if' c_src/src/main.c` → 0). The C therefore has exactly **one**
  build configuration.
* `Cargo.toml` `[features]` contains only `default = []`; there is no optional
  dependency and no `#[cfg(feature = ...)]` anywhere in `src/`. The complete set
  of valid feature combinations is therefore:

  | # | feature combination | command |
  |---|---------------------|---------|
  | 1 | *(default)* = `default` = `{}` | `cargo check/build/test` |
  | 2 | `--no-default-features` (same empty set, since `default = []`) | `cargo check/build/test --no-default-features` |

  Both combinations are run in **both** profiles (`dev` and `release`, the
  latter being the deliverable, built with `panic = "abort"`) by
  `scripts/verify_all.sh` — 4 configurations in total.

## Runtime configuration axes the C actually branches on

| axis | values the C distinguishes |
|---|---|
| entry point | `main` (process; reads stdin) — `run(int)` (exported; callable directly, any number of times) |
| accumulated global state | `the_house.floors` (+1 per `run`), `.bathrooms` (+1.0 per `run`), `.bedrooms` (+`extra` per `run`) → every call prints different values |
| `scanf` whitespace prefix | none / `' '` / `'\t'` / `'\n'` / `'\v'` / `'\f'` / `'\r'` / long mixed runs crossing the stdio buffer |
| `scanf` sign | absent / `'+'` / `'-'` |
| `scanf` digit run | 1 digit / many digits / leading zeros / ≥19 digits (the `long` overflow path) / 100 000 digits |
| `scanf` terminator | EOF / newline / space / non-digit garbage / a second token |
| parsed `x` value class | `0` / positive / negative / `INT_MAX` / `INT_MIN` / values that wrap `bedrooms` |
| stdin kind | pipe / regular file (seekable → exit-time offset fix-up) / directory / closed / never-EOF stream |
| stdout kind | pipe (fully buffered in C) / regular file / `/dev/null` / closed / pipe without a reader |
| argv | absent / present (ignored by `int main()`) |

## Configuration rows (every combination the C treats differently)

Legend — **P** = `tests/differential_process.rs` (C `c_src/build/driver` vs Rust
`target/*/driver`; stdout + stderr + exit code/signal compared byte for byte),
**F** = `tests/differential_ffi.rs` (`build_c/libcdriver.so` vs
`target/*/libdriver.so`, both `dlopen`ed with `libloading`, captured stdout
compared), **S** = `tests/symbol_parity.rs`, **E** = `tests/error_paths.rs`.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `main` (P) | stdin `"0\n"` — baseline, `x = 0` | `row01_06_basic_values_and_terminators` | [x] |
| 2  | `main` (P) | stdin `"3\n"` — small positive | `row01_06_…` | [x] |
| 3  | `main` (P) | stdin `"-4\n"` — small negative | `row01_06_…` | [x] |
| 4  | `main` (P) | stdin `"+7\n"` — explicit plus sign | `row01_06_…` | [x] |
| 5  | `main` (P) | no trailing newline (`"3"`, EOF right after the digits) | `row01_06_…` | [x] |
| 6  | `main` (P) | `"\r\n"`-terminated (`"3\r\n"`) | `row01_06_…` | [x] |
| 7  | `main` (P) | every whitespace kind as prefix (`' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'`) plus 100 randomized mixes | `row07_whitespace_prefixes` | [x] |
| 8  | `main` (P) | token straddling the 4096/8192-byte stdio buffer boundary | `row08_buffer_boundary` | [x] |
| 9  | `main` (P) | leading zeros; digit runs of length 1…25 × 3 sign forms (randomized digits) | `row09_digit_runs` | [x] |
| 10 | `main` (P) | trailing garbage after the number (`"5abc"`, `"5.5"`, `"0x10"`, embedded NUL, …) | `row10_trailing_garbage` | [x] |
| 11 | `main` (P) | several tokens (`"1 2 3\n"`, `"1\n2\n"`) — only the first is read | `row11_multiple_tokens` | [x] |
| 12 | `main` (P) | exact `int` boundaries (`INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1`, 16-bit boundaries) | `row12_int_boundaries` | [x] |
| 13 | `main` (P) | out-of-`int`/out-of-`long` values, `2^31`, `2^32`, `LONG_MAX`, `LONG_MIN`, 30-digit values | `row13_long_boundaries` | [x] |
| 14 | `main` (P) | 400 randomized `i32` values (fixed seed) with randomized sign form, leading zeros, whitespace and terminator | `row14_randomized_int_values` | [x] |
| 15 | `main` (P) | 300 randomized junk strings + 200 fully random byte strings (fixed seed, incl. NUL and high bytes) | `row15_randomized_junk` | [x] |
| 16 | `main` (P) | stdout → regular file (C switches to full buffering), stdin → regular file | `row16_17_stdout_destinations` | [x] |
| 17 | `main` (P) | stdout → `/dev/null`, stdin → `/dev/null` | `row16_17_stdout_destinations` | [x] |
| 18 | `main` (P) | extra argv entries with a normal and with an empty stdin | `row18_extra_argv` | [x] |
| 19 | `main` (P) | 1 MiB of whitespace, 200 000 newlines, 100 000-digit runs (all three sign forms), 64 KiB digit run without newline | `row19_large_input` | [x] |
| 20 | `run` (F) | `run(0)` as the very first call — pristine global state | `ffi_differential` | [x] |
| 21 | `run` (F) | `run(v)` 200× with randomized `v` (fixed seed), compared step by step | `ffi_differential` | [x] |
| 22 | `run` (F) | `run(INT_MAX)`, `run(INT_MIN)`, `run(±1)`, `run(±10^9)` in sequence — wrap-around accumulation | `ffi_differential` | [x] |
| 23 | `run` (F) | 150 consecutive `run(1)` calls so `bathrooms` grows past 100.5 → `%.1f` of larger magnitudes | `ffi_differential` | [x] |
| 24 | `run` (F) | `run` after `run`: the mutated `static` / `static mut` state must match at every step (per-step comparison) | `ffi_differential` | [x] |
| 25 | `main` (F) | exported `main` called through `dlsym` with fd 0 redirected to a file (`"7\n"`); return value compared too | `ffi_differential` | [x] |
| 26 | both | `nm -D` symbol parity, symbol types, `dlsym` resolvability, and that no `static` C function leaked into either `.so` | `symbol_parity` (3 tests) | [x] |
| 27 | build | every feature combination × both profiles (4 configurations) | `scripts/verify_all.sh` | [x] |
| 28 | `main` (P) | **seekable** stdin: C's `stdio` seeks back to the logically consumed position at exit, so `{ driver >/dev/null; cat; } < file` must show identical leftovers — 23 hand-picked + 200 randomized inputs | `row28_seekable_stdin_leftover` | [x] |
| 29 | `main` (P) | stdin that never reaches EOF (`/dev/zero`, `yes 5 \| driver`): must terminate with identical output instead of draining the stream | `row29_never_ending_stdin` | [x] |

All error/rejection configurations (closed fds, EISDIR, SIGPIPE, out-of-range
values, …) are enumerated separately in `ERRORS.md` and covered by
`tests/error_paths.rs` (**E**).
