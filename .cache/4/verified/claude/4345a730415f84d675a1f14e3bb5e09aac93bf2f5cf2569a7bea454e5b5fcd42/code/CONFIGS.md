# CONFIGS.md — configuration surface table (valid inputs)

## Build-time configuration axes

Enumerated mechanically from the two build files:

| source | axis | values |
|--------|------|--------|
| `Cargo.toml` | `[features]` | **absent** — the crate declares no features, so the only combination is the empty one |
| `Cargo.toml` | default features | none |
| `c_src/CMakeLists.txt` | `option()` / `if()` / `add_definitions` / `target_compile_definitions` | **none** |
| `c_src/src/main.c` | `#if` / `#ifdef` / `#ifndef` | **none** |

**Total valid feature combinations: 1** (the empty set). `cargo check
--no-default-features` and `cargo check` are therefore the same build; both are
run by `scripts/verify.sh`, which derives the list as the power set of
`[features]` rather than hard-coding it. There is no `#[cfg(feature = ...)]` code
to gate, because there are no features and no C preprocessor conditionals to
mirror.

The one remaining build axis that *does* change generated code is the Cargo
profile, because `[profile.release]` sets `panic = "abort"`. Both profiles are
verified (row 40).

## Runtime configuration axes

Enumerated from the C source. The program:

- takes **no** command-line arguments (`int main()` — no `argc`/`argv`, row 41);
- reads **no** environment variables (row 41);
- never calls `setlocale`, so it stays in the `"C"` locale regardless of
  `LC_ALL`/`LANG` (row 38);
- has **no** flags, modes, or options of any kind — there is not a single `if`
  or `switch` in the code.

So the entire configuration surface is (a) the **shape of the stdin byte
stream**, (b) the **kind of descriptor** stdin/stdout are connected to, and
(c) which **entry point** is used. The axes the code actually distinguishes, all
inside the `scanf("%d")` conversion:

| axis | distinct values the C code branches on |
|------|----------------------------------------|
| leading whitespace | none / ` ` / `\t` / `\n` / `\v` / `\f` / `\r` / repeated+mixed |
| sign | none / `+` / `-` / doubled |
| digit run | 1 digit / many / leading zeros / >19 digits / 10 000 digits |
| magnitude class | `0` / small / `INT_MAX`±1 / `UINT_MAX`±1 / `LONG_MAX`±1 / `LONG_MIN`∓1 / far past |
| terminating byte | EOF / whitespace / letter / punctuation / `.` / `x` / NUL / high byte |
| conversions that succeed | 0 / 1 / 2 |
| token count in stream | 0 / 1 / 2 / >2 |
| stream shape | empty / whitespace-only / short / multi-MB / never reaches EOF |
| stdin descriptor | file / pipe / dripped pipe / closed |
| stdout descriptor | file / pipe / closed / pipe with no reader |
| entry point | `main` (process) / `main` (FFI `.so`) / `driver` (FFI `.so`) |

## The table

Every row is exercised with **many randomized inputs** (fixed seed, xorshift64\*,
so failures reproduce) unless the row is inherently a single fixed shape. All
rows pass for both builds.

### `driver` — the low-level entry point, called directly through the `.so`

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `driver` (FFI) | full cross product of the 9-value boundary set `{0, 1, -1, 2, -2, INT_MIN, INT_MAX, 0x55555555, 0xAAAAAAAA}` for `x` × same for `y` (81 combinations) | `cfg01_driver_boundary_cross_product` | [x] |
| 2  | `driver` (FFI) | 4 000 uniformly randomized `(x, y)` pairs over the whole `i32` range | `cfg02_driver_randomized` | [x] |
| 3  | `driver` (FFI) | correlated pairs where the result is degenerate: `y = 0` (`x|-1 == -1`), `y = -1` (`x|0 == x`), `y = x`, `y = !x`, `y = -x` | `cfg03_driver_correlated` | [x] |
| 4  | `driver` (FFI) | single-bit `x` × single-bit `y` (`1<<i`, `1<<j` for all 32×32) — walks every bit position through `|` and `~` | `cfg04_driver_single_bits` | [x] |
| 5  | `driver` (FFI) | 1 000 back-to-back calls in one process — checks no per-call state leaks and stdout stays byte-identical | `cfg05_driver_repeated_calls` | [x] |

### `main` — via the `.so` export

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 6  | `main` (FFI) | stdin redirected from a temp **file**, stdout captured; 12 inputs spanning valid pairs, empty, garbage, sign-only, double-sign and out-of-range magnitudes (one subprocess per input, because `FILE *stdin` cannot be reset) | `err22_ffi_main_symbol` | [x] |

### `main` — as a process (stdin bytes → stdout bytes + exit status)

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 7  | `main` (process) | two ints, single space separator, both randomized over the full `i32` range (600 cases) | `cfg07_two_ints_space_separated` | [x] |
| 8  | `main` (process) | separator swept over every whitespace byte and repetition: ` `, `\t`, `\n`, `\v`, `\f`, `\r`, `\r\n`, `\n\n`, plus 300 randomized mixed runs | `cfg08_separator_sweep` | [x] |
| 9  | `main` (process) | leading whitespace before the first token, every whitespace byte × `{1, 2, 7, 4095, 4096, 4097, 9000}` repetitions (spanning the 4 096-byte stdio buffer) plus randomized runs | `cfg09_leading_whitespace` | [x] |
| 10 | `main` (process) | trailing byte after the second token: EOF immediately / `\n` / `\r\n` / whitespace run / letter / punctuation / sign | `cfg10_trailing_bytes` | [x] |
| 11 | `main` (process) | explicit `+` sign on the first / second / both tokens, randomized magnitudes | `cfg11_plus_sign_combinations` | [x] |
| 12 | `main` (process) | `-` sign on the first / second / both tokens, randomized magnitudes | `cfg12_minus_sign_combinations` | [x] |
| 13 | `main` (process) | leading zeros: 1–40 `0`s prefixed to a randomized value, with and without a sign, plus `010`/`0777`/`08`/`09` (checks base-10 does **not** switch to octal) | `cfg13_leading_zeros` | [x] |
| 14 | `main` (process) | `0`, `-0`, `+0`, `00000`, `-00000`, `+00000` in both positions (6×6) | `cfg14_zero_forms` | [x] |
| 15 | `main` (process) | `int` boundary values in both positions: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` (7×7) | `cfg15_int_boundaries` | [x] |
| 16 | `main` (process) | just past the `int` range: `2147483648`, `-2147483649`, `4294967295`, `4294967296`, `4294967297`, `-4294967296`, `-4294967295`, `8589934592` (8×8) | `cfg16_just_past_int_range` | [x] |
| 17 | `main` (process) | `long` boundaries exactly: `LONG_MAX`, `LONG_MIN`, and one step inside each (4×4) | `cfg17_long_boundaries_exact` | [x] |
| 18 | `main` (process) | past the `long` range (`ERANGE` clamp path): `LONG_MAX+1`, `LONG_MIN-1`, `UINT64_MAX`, `2^64`, 26-digit and 40-digit magnitudes, both signs (8×8) | `cfg18_past_long_range_erange_clamp` | [x] |
| 19 | `main` (process) | randomized decimal strings of 1–25 digits with a randomized sign — sweeps across the `int`, `uint` and `long` boundaries by construction (600 cases) | `cfg19_randomized_digit_lengths` | [x] |
| 20 | `main` (process) | 10 000-digit tokens, with and without a sign; plus a 10 000-zero prefix on a small value | `cfg20_very_long_digit_run` | [x] |
| 21 | `main` (process) | exactly **one** token, so the second conversion fails and `y` keeps `0`; randomized value × 4 trailing shapes | `cfg21_single_token_only` | [x] |
| 22 | `main` (process) | **more than two** tokens (3–8), so the surplus is never read; randomized values | `cfg22_more_than_two_tokens` | [x] |
| 23 | `main` (process) | digits immediately followed by a token-terminating non-digit: `5abc`, `5.75`, `1e5`, `0x5`, `0X5`, `5-3`, `5+3`, `12,34`, `7)`, `9_9`, `3/4`, `8:2` (each alone and as a 12×12 pair) | `cfg23_digits_then_nondigit` | [x] |
| 24 | `main` (process) | `0x`/`0X` forms in both positions — base-10 stops at `x`, so `0x1F` reads as `0` and leaves `x1F` | `cfg24_hex_prefix_forms` | [x] |
| 25 | `main` (process) | second token adjacent to the first with no separator (`12-34`, `12+34`, …) — the sign terminates token 1 and starts token 2 | `cfg25_adjacent_tokens_no_separator` | [x] |
| 26 | `main` (process) | embedded NUL bytes before, between, inside and after tokens | `cfg26_embedded_nul_bytes` | [x] |
| 27 | `main` (process) | all 128 high bytes `0x80`–`0xFF` as leading byte, terminator, separator, and alone — none is a digit or space in the `"C"` locale | `cfg27_high_bytes` | [x] |
| 28 | `main` (process) | **structured fuzz**: randomized token soup drawn from {whitespace, sign, digits, letters, punctuation, NUL, high bytes} with randomized lengths (1 000 cases) | `cfg28_structured_fuzz` | [x] |
| 29 | `main` (process) | **raw byte fuzz**: uniformly random bytes over `0x00`–`0xFF`, lengths 0–64 (1 000 cases) | `cfg29_raw_byte_fuzz` | [x] |
| 30 | `main` (process) | stdin **never reaches EOF** (endless producer, 5 patterns). Asserts both builds exit promptly **and** that each consumes < 1 MiB of the endless stream, which is what pins down `scanf`'s laziness. Also asserts that an endless **whitespace** stream makes *both* builds block forever, since `%d` skips whitespace without bound | `cfg30_unbounded_stdin` | [x] |
| 31 | `main` (process) | 8 MiB stdin whose first two tokens are valid — surplus must not affect the result; as pipe and as file | `cfg31_huge_stdin_valid_prefix` | [x] |
| 32 | `main` (process) | 8 MiB stdin containing **no** valid token at all; as pipe and as file | `cfg32_huge_stdin_no_valid_token` | [x] |
| 33 | `main` (process) | empty stdin (0 bytes); as pipe and as file | `cfg33_empty_stdin` | [x] |
| 34 | `main` (process) | whitespace-only stdin, every whitespace byte × `{1, 10, 4095, 4096, 4097, 100000}` repetitions, plus randomized runs | `cfg34_whitespace_only_stdin` | [x] |
| 35 | `main` (process) | stdin is a **regular file** vs a **pipe** — same bytes, both descriptor kinds, over 67 shapes | `cfg35_stdin_file_vs_pipe` | [x] |
| 36 | `main` (process) | stdout is a **regular file** vs a **pipe** (different stdio buffering decisions, same bytes required) | `cfg36_stdout_file_vs_pipe` | [x] |
| 37 | `main` (process) | stdin not connected to any data | `cfg37_stdin_closed` | [x] |
| 38 | `main` (process) | `LC_ALL`/`LANG`/`LC_NUMERIC` set to `C`, `POSIX`, `en_US.UTF-8`, `de_DE.UTF-8`, `tr_TR.UTF-8` — the program never calls `setlocale`, so output must be unchanged | `cfg38_locale_is_irrelevant` | [x] |
| 39 | `main` (process) | stdin delivered **one byte per write**, forcing short reads and partial buffer fills | `cfg39_dripped_stdin` | [x] |
| 40 | `main` (process) | Rust binary built in the **dev** profile and in the **release** profile (`panic = "abort"`) — the whole suite above is run twice against the same C binary | `scripts/verify.sh` | [x] |
| 41 | `main` (process) | command-line arguments (`-h`, `--help`, positional, mixed) × extra environment variables — `int main()` takes no parameters and never calls `getenv`, so neither may change a byte | `cfg41_argv_and_env_are_ignored` | [x] |
| 42 | `main` (process) | dense sweep of the ±4 neighbourhood around every conversion boundary (`2^15`, `2^16`, `2^31`, `2^32`, `2^63`, `2^64`, `2^65`, …) × {unsigned, `-`, `+`, leading zeros}, each in the `x` and `y` position plus 500 randomized pairs — this is where the `strtol` clamp and the `long`→`int` truncation interact | `cfg42_conversion_boundary_neighbourhoods` | [x] |
| 43 | `main` (process) | the **C reference at `-O0` vs `-O2`** over 510 shapes (CMakeLists uses no explicit `-O`, the FFI reference uses `-O2`). They agree everywhere, so "C is ground truth" is unambiguous and the expected values in `ERRORS.md` are not pinned to one build; Rust is compared against both | `cfg43_c_reference_is_optimisation_independent` | [x] |
| 44 | `main` (process) | stdin is a **directory** descriptor — `open` succeeds but `read(2)` fails with `EISDIR`, which the C library reports to `scanf` as end-of-file | `cfg44_stdin_is_a_directory` | [x] |

## Coverage notes

- Rows 1–5 hit the lowest-level entry point (`driver`) directly through the
  `.so`, not only through the `main` wrapper.
- Rows 7–42 drive the composed pipeline (`scanf` → `scanf` → `driver` →
  `printf`/`puts`) end to end at the process boundary, which is the only place
  the exit status, `SIGPIPE` disposition and lazy stdin consumption are visible.
- The error/rejection counterparts of these rows live in `ERRORS.md`.
- `scripts/negative_control.sh` mutation-tests the suite: it injects 11 realistic
  translation bugs one at a time and requires every one to be rejected. This is
  what makes the check marks above meaningful — an early version of row 30
  passed while a deliberately eager-reading mutant survived, which is how that
  row's assertion was strengthened from wall-clock time to bytes consumed.
