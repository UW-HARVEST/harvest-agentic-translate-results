# Differential verification log

C ground truth: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Rust under test: `translation/target/{debug,release}/driver`.
Harness: `translation/tests/differential.rs` — runs **both binaries as
subprocesses**, feeds identical stdin, and compares stdout bytes, stderr bytes
and exit status (including death-by-signal) for every input.

## Result

**No mismatches were found.** 40 differential tests pass in both the debug and
release profiles, plus an ad-hoc sweep of 400 random binary stdins (0–19 bytes)
and ~270 numeric inputs (`-130..130`, plus `INT_MAX`, `INT_MIN±1`, `2^32`,
`2^32+1`, `2^32+99`, `±999999999999`) — all byte-identical with identical
statuses. The Rust translation already modelled every behaviour listed below;
each item is a place where a naive translation *would* have diverged, and how
it was confirmed correct.

## Behaviours that had to match exactly (all verified)

| # | C behaviour | Naive Rust would do | Verified by |
|---|---|---|---|
| 1 | `data` stays `-1` when `fgets` returns NULL, so **empty stdin reaches `strncpy(dest, source, (size_t)-1)` and the process dies of SIGSEGV** (exit status 139 / signal 11). | Exit 0 after printing `fgets() failed.` | `empty_input_fgets_fails`; also `driver <&-` and `< /dev/null` |
| 2 | `printf` output sits in the fully buffered `stdout` FILE buffer when stdout is a pipe, and the fatal signal discards it — so on empty input **stdout is completely empty** even though `printLine("fgets() failed.")` ran. | Print the message (Rust's `println!` is unbuffered per call) | `empty_input_fgets_fails` (0 bytes on stdout for both) |
| 3 | On a **terminal** stdout is line buffered, so the same run *does* emit `fgets() failed.\n` before dying. The buffering mode is observable. | Always-empty (or always-printed) output | `stdout_buffering_mode_matches` (runs both under a pty via `script -qec`) |
| 4 | Any `data < 0` — from `-1`, `" -5"`, `INT_MIN`, or **`(int)` truncation of a larger `long`** such as `4294967295` → `-1` and `9999999999999` (LONG_MAX-saturated) → `-1` — takes the same crash path. | Panic with a Rust message on stderr, or clamp to 0 | `negative_one`, `negative_with_leading_space`, `int_min`, `negative_via_int_truncation`, `long_overflow_saturates_negative` |
| 5 | The crash must produce **empty stderr** and terminate by signal, not by a Rust panic message. The Rust program therefore restores `SIG_DFL` for SIGSEGV (the Rust runtime installs its own stack-overflow handler) before `raise(SIGSEGV)`. | `thread 'main' panicked ...` on stderr, exit 101 | every crash-path test asserts stderr equality |
| 6 | `atoi` is `(int)strtol(s, NULL, 10)`: parses a leading sign, stops at the first non-digit, yields 0 on no digits, **saturates at LONG_MAX/LONG_MIN**, then **truncates to `int`**. So `8589934592` → 0, `4294967301` → 5, `1234567890123` → 1912276171 (≥100, no copy). | `str::parse::<i32>()` erroring out, or `i32` wrapping instead of `long` saturation | `zero_via_int_truncation`, `small_positive_via_int_truncation`, `buffer_exactly_full_thirteen_digits`, `hex_prefix_...`, `float_like_...`, `exponent_notation_...`, `sign_with_no_digits`, `only_spaces`, `non_numeric_input` |
| 7 | `fgets(inputBuffer, 14, stdin)` reads **at most 13 bytes and stops after a newline** — it does *not* read across newlines like `scanf`. `"42\n99\n"` prints 42 A's; `"12345678901234567"` only sees `1234567890123`. | Read a whole line/all of stdin, or read a second token | `stops_at_newline_ignoring_second_line`, `input_longer_than_buffer_is_truncated`, `newline_inside_buffer_window`, `fourteenth_digit_would_change_the_value` |
| 8 | `fgets` returns non-NULL for a lone `"\n"` (data = 0 → blank line), and NULL only when EOF hits with zero bytes read. | Treat a blank line as failure | `lone_newline_is_read_successfully` vs `empty_input_fgets_fails` |
| 9 | Input bytes are arbitrary, not UTF-8 (`\xff\xfe 7`), and may contain an embedded NUL, which terminates the string `atoi` sees (`"\x0042\n"` → 0). | `String::from_utf8` failing, or reading past the NUL | `non_utf8_bytes`, `embedded_nul_byte` |
| 10 | `0 <= data < 100`: `strncpy` copies `data` `'A'`s and `dest[data] = '\0'`; `data == 99` is the maximum handled (last in-bounds byte of `dest[100]`). `data >= 100` skips the copy entirely, leaving `dest` as `""` → a **single newline** on stdout. | Off-by-one at 99/100, or printing nothing at all for `data >= 100` | `max_handled_length`, `ninety_eight`, `exactly_one_hundred`, `just_over_one_hundred`, `int_max` |
| 11 | `printf("%s\n", line)` stops at the first NUL in `dest`, so output is exactly `data` `'A'`s plus `\n`; the program always `return 0` on the non-crashing paths. | Printing the full 100-byte array | all positive-value tests (exact byte comparison) |

## Enumerated input classes and their C paths

| Input class | C path taken | Observable result |
|---|---|---|
| empty stdin / closed stdin / `/dev/null` | `fgets` → NULL, `printLine("fgets() failed.")`, `data == -1` → `strncpy` with huge size | pipe: no output, killed by SIGSEGV (139); tty: `fgets() failed.` then SIGSEGV |
| `"\n"`, `"abc"`, `"-"`, `"   "`, `"0x10"`, `"1e3"`, `"\0..."` | `atoi` → 0, `strncpy(dest, src, 0)`, `dest[0] = 0` | one empty line, exit 0 |
| `"1"` (single item) … `"99"` (maximum handled) | `data < 100`, copy `data` `'A'`s | `data` `'A'`s + `\n`, exit 0 |
| `"100"`, `"101"`, `"2147483647"`, `"1234567890123"` | `data < 100` false, copy skipped | one empty line, exit 0 |
| `"-1"`, `" -5"`, `"-2147483648"`, `"4294967295"`, `"9999999999999"` | `data < 0` → `strncpy` length underflows to a huge `size_t` | no output, killed by SIGSEGV (139), empty stderr |
| more than 13 bytes, or bytes after a `\n` | `fgets` truncates / stops at `\n` | value comes from the first ≤13 bytes only |

## Notes

* `printLine(NULL)` is dead code: both call sites pass a non-NULL pointer, so
  the `line != NULL` guard can never be false. It is modelled in Rust as
  `Option<&[u8]>` for fidelity but is unreachable, exactly as in C.
* Nothing under `c_src/` was modified — `c_src/src/main.c` and
  `c_src/CMakeLists.txt` still carry their original mtimes. The test harness
  only *builds* the C tree, into the generated `c_src/build/` directory.
* No test is `#[ignore]`d, skipped or disabled.
