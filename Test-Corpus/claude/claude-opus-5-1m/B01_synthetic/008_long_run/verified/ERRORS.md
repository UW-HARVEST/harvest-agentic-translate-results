# ERRORS.md — error-surface table (Phase A) and its differential tests (Phase C)

Derived mechanically from `c_src/src/main.c`. The whole file contains exactly
**two** rejection sites (there are no `assert`s, no allocation checks, no NULL
checks and no other `return`s):

```c
50:    if (argc != 2) {
51:        fprintf(stderr, "Usage: %s <seed>\n", argv[0]);
52:        return 1;
       }
...
58:    if (*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX) {
59:        fprintf(stderr, "Invalid seed: '%s'\n", argv[1]);
60:        return 1;
       }
```

Site 2 is a three-term `||`, so it contributes three distinct triggers, each of
which is reachable through several distinct glibc `strtoul` behaviours (no
conversion / partial conversion / `ERANGE` / unsigned negation). Every distinct
trigger below gets its own row.

Constants that bound the input domain: `ARRAY_SIZE = 256 * 1024`,
`ITERATIONS = 2000`, `UINT_MAX` (`4294967295`), `ULONG_MAX`
(`18446744073709551615`, the `strtoul` overflow point).

Expected results are stated as *(exit code, stderr bytes, stdout bytes)*. Both
implementations are called through `dlopen`+`dlsym` on `main` in
`libc_driver_O{0,2}.so` and `libdriver.so`, with fds 1/2 redirected to temp
files, and the captured bytes are compared for equality — never merely
"both failed".

## Site 1 — `argc != 2`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `main` | `argc == 0`, `argv[0] == NULL` | 1, `Usage: (null) <seed>\n` (glibc renders a NULL `%s` as `(null)`) | `errors.rs::argc_zero_null_argv0` |
| 2 | `main` | `argc == 0`, `argv[0] == "driver"` (legal FFI call: C reads `argv[0]` regardless of `argc`) | 1, `Usage: driver <seed>\n` | `errors.rs::argc_zero_with_argv0` |
| 3 | `main` | `argc == 1` (no seed given) | 1, `Usage: <argv0> <seed>\n` | `errors.rs::argc_wrong_counts` |
| 4 | `main` | `argc == 3` (one arg too many) | 1, `Usage: <argv0> <seed>\n`, `argv[1]` ignored even when valid | `errors.rs::argc_wrong_counts` |
| 5 | `main` | `argc == 4 … 8` | 1, same message | `errors.rs::argc_wrong_counts` |
| 6 | `main` | `argc == -1` (out-of-domain `int` across FFI; C only tests `!= 2`) | 1, same message | `errors.rs::argc_out_of_range` |
| 7 | `main` | `argc == INT_MIN`, `argc == INT_MAX` (extreme out-of-domain `int`) | 1, same message | `errors.rs::argc_out_of_range` |
| 8 | `main` | `argc != 2` with `argv[0]` = empty string | 1, `Usage:  <seed>\n` (two spaces) | `errors.rs::argc_wrong_counts` |
| 9 | `main` | `argc != 2` with `argv[0]` containing non-UTF-8 bytes (`\xff\xfe`) | 1, raw bytes echoed unchanged | `errors.rs::argv0_non_utf8` |

## Site 2a — `*endptr != '\0'` (strtoul stopped before the terminator)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 10 | `main` | `argv[1] = "abc"` — no conversion at all, `endptr == nptr` | 1, `Invalid seed: 'abc'\n` | `errors.rs::invalid_seed_strings` |
| 11 | `main` | `argv[1] = "42abc"`, `"1x"`, `"12 34"` — trailing garbage after digits | 1, `Invalid seed: '<arg>'\n` | `errors.rs::invalid_seed_strings` |
| 12 | `main` | `argv[1] = "   "` / `"\t"` / `"\n"` — whitespace only (skipped, then no digits) | 1, `Invalid seed: '<arg>'\n` | `errors.rs::invalid_seed_strings` |
| 13 | `main` | `argv[1] = "-"`, `"+"`, `"--5"`, `"+-5"`, `"- 5"` — sign without digits | 1, `Invalid seed: '<arg>'\n` | `errors.rs::invalid_seed_strings` |
| 14 | `main` | `argv[1] = "0x10"`, `"0b1"`, `"010z"` — base-16/2 syntax rejected in base 10 (stops after `0`) | 1, `Invalid seed: '<arg>'\n` | `errors.rs::invalid_seed_strings` |
| 15 | `main` | `argv[1] = "42 "`, `"42\n"`, `"42\t"` — trailing whitespace is **not** consumed | 1, `Invalid seed: '<arg>'\n` | `errors.rs::invalid_seed_strings` |
| 16 | `main` | `argv[1] = "4\xff"`, `"\xc3\xa9"` — non-ASCII / non-UTF-8 bytes | 1, raw bytes echoed unchanged | `errors.rs::invalid_seed_strings` |
| 17 | `main` | `argv[1] = "١٢٣"` (Arabic-Indic digits, UTF-8) — not ASCII digits | 1, raw bytes echoed | `errors.rs::invalid_seed_strings` |
| 18 | `main` | `argv[1] = "1,000"`, `"1_000"`, `"1.0"`, `"1e3"` — grouping/float syntax | 1, `Invalid seed: '<arg>'\n` | `errors.rs::invalid_seed_strings` |

## Site 2b — `errno != 0` (glibc `strtoul` set `ERANGE`)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 19 | `main` | `argv[1] = "18446744073709551616"` (`ULONG_MAX + 1`) | 1, `Invalid seed: '…'\n` (`ERANGE`) | `errors.rs::invalid_seed_strings`, `parse.rs::erange_boundary` |
| 20 | `main` | `argv[1]` = 20…200 digits of `9` | 1, `Invalid seed: '…'\n` (`ERANGE`) | `errors.rs::invalid_seed_strings`, `parse.rs::erange_boundary` |
| 21 | `main` | `argv[1] = "-18446744073709551616"` — negated **overflow** (glibc still sets `ERANGE`; its returned value differs from the positive case but the `errno` term rejects first) | 1, `Invalid seed: '…'\n` | `parse.rs::erange_boundary` |
| 22 | `main` | `argv[1] = "0000…0018446744073709551616"` — overflow behind leading zeros | 1, `Invalid seed: '…'\n` (`ERANGE`) | `parse.rs::erange_boundary` |

## Site 2c — `temp_seed > UINT_MAX` (in range for `unsigned long`, too big for `unsigned int`)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 23 | `main` | `argv[1] = "4294967296"` (`UINT_MAX + 1`, one step past the valid range) | 1, `Invalid seed: '4294967296'\n` | `errors.rs::invalid_seed_strings`, `parse.rs::uint_max_boundary` |
| 24 | `main` | `argv[1] = "18446744073709551615"` (`ULONG_MAX` exactly — **no** `ERANGE`) | 1, `Invalid seed: '…'\n` | `parse.rs::uint_max_boundary` |
| 25 | `main` | `argv[1] = "-1"` — unsigned negation yields `ULONG_MAX`, no `ERANGE` | 1, `Invalid seed: '-1'\n` | `errors.rs::invalid_seed_strings`, `parse.rs::negative_wraparound` |
| 26 | `main` | `argv[1] = "-4294967295"`, `"-2"`, `"-999999"` — other negatives that wrap above `UINT_MAX` | 1, `Invalid seed: '<arg>'\n` | `parse.rs::negative_wraparound` |
| 27 | `main` | `argv[1] = "9223372036854775808"` (`LONG_MAX + 1`, still `< ULONG_MAX`) | 1, `Invalid seed: '…'\n` | `parse.rs::uint_max_boundary` |

## Boundary conditions that are **accepted** (the mirror of the table above)

Recorded here because getting them wrong is an error-surface bug in the
opposite direction (rejecting what C accepts).

| # | function | input | expected C result | test |
|---|----------|-------|-------------------|------|
| 28 | `main` | `argv[1] = ""` — glibc reports "no conversion" *without* setting `errno`, and `*endptr` is already `'\0'`, so **all three terms are false**: accepted with `seed = 0` | 0, prints the seed-0 result | `parse.rs::accepted_forms`, `pipeline.rs::pipeline_matches_for_seed_arguments` |
| 29 | `main` | `argv[1] = "4294967295"` (`UINT_MAX`, last valid value) | 0, accepted | `parse.rs::uint_max_boundary` |
| 30 | `main` | `argv[1] = "-0"`, `"+0"`, `"-000"` | 0, accepted, `seed = 0` | `parse.rs::accepted_forms` |
| 31 | `main` | `argv[1] = "-18446744073709551615"` — unsigned negation wraps to `1`, no `ERANGE` | 0, accepted, `seed = 1` | `parse.rs::negative_wraparound` |
| 32 | `main` | `argv[1] = " \t\n\v\f\r42"` — every `isspace` byte is skipped | 0, accepted, `seed = 42` | `parse.rs::accepted_forms` |
| 33 | `main` | `argv[1] = "+42"`, `"0000000000000000000000042"` | 0, accepted, `seed = 42` | `parse.rs::accepted_forms` |
| 34 | `main` | `argv[1] = "-4294967296"` → wraps to `18446744069414584320` > `UINT_MAX` → rejected; `"-18446744069414584321"` → wraps to `4294967295` → accepted | as stated | `parse.rs::negative_wraparound` |

## Generic FFI boundaries (not in the C table, checked anyway)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 35 | `main` | `argc == 2`, `argv[1]` = 4096-byte argument | 1, whole argument echoed | `errors.rs::long_argument` |
| 36 | `perform_expensive_operations` | called with the global `array` in its initial (all-zero, `.bss`) state | no error path exists — pure computation, must match byte-for-byte | `perform.rs::all_zeros` |
| 37 | `perform_expensive_operations` | called repeatedly (state accumulates in `array`) | must stay in lock-step across calls | `perform.rs::all_zeros` (10 calls), `perform.rs::random_full_range_repeated` |
| 38 | `main` | called twice in one process (global `array` retains state; the seeding loop overwrites all of it) | second call reproduces the first result | `pipeline.rs::pipeline_is_repeatable` |
| 39 | `driver` binary | stdout **and** stderr are a pipe with no reader, so the `fprintf`/`printf` write fails | the C keeps the default `SIGPIPE` disposition and **dies from signal 13** (no exit code). Rust's runtime installs `SIG_IGN`, which would have exited 1 instead — `src/main.rs` restores `SIG_DFL` to match | `binary_cli.rs::cli_sigpipe_disposition` |
| 40 | `driver` binary | `execve` with an empty `argv` array | this kernel normalises it to `argc == 1`, `argv[0] == ""`, so the program prints `Usage:  <seed>\n` (row 1's NULL `argv[0]` is only reachable by calling `main` directly) | `binary_cli.rs::cli_argc_zero` |
| — | `main` | `argc == 2` with `argv[1] == NULL`, or `argv == NULL` | **C dereferences a NULL pointer and faults** — no defined behaviour to match, deliberately not tested | n/a |
| — | *enum arguments* | none exist in this API (`main`'s only integer parameter is `argc`, covered by rows 6–7) | n/a | n/a |

There are no other error returns, sentinels, `assert`s or range checks in
`c_src/src/main.c`.

## Additional (not row-specific) error-path tests

| test | what it adds |
|------|--------------|
| `parse.rs::rejected_syntax` | 48 hand-written `*endptr != '\0'` shapes, each asserted to be rejected by real glibc *and* to produce the same decision in Rust (covers rows 10–18 at the validation level) |
| `errors.rs::random_invalid_arguments` | 600 pseudo-random rejected `argv[1]` values through `main` on both sides (property-style, seed `0x5eed_7000`) |
| `errors.rs::error_messages_are_distinct_and_exact` | pins the two `fprintf` texts to exact bytes for all three implementations, so a swapped/garbled message cannot pass by "both differ identically" |
| `binary_cli.rs::binaries_exist_and_agree_on_misuse` | `--help` / `-h` / `--seed=42` style misuse at the process level |

## Status

Every row above has a passing differential test against the C compiled at both
`-O0` and `-O2`, in both the Rust `dev` and `release` profiles
(`scripts/run_all.sh`). Rows 1–35 are checked through the exported `main` of the
two shared objects (`tests/errors.rs`); rows 36–38 through
`perform_expensive_operations` / the reduced pipeline; rows 39–40 through the two
executables as processes (`tests/binary_cli.rs`).

One benign implementation difference is deliberately *not* mirrored: on overflow
of a **negated** number (`"-99999999999999999999"`), glibc's `strtoul` returns
`-ULONG_MAX` (= 1) with `ERANGE`, while `src/strtoul.rs` returns `ULONG_MAX` with
`ERANGE`. The C program tests `errno != 0` before ever using the value, so the
observable behaviour (`Invalid seed: '...'`, exit 1) is identical — this is
covered by row 21 and by `parse.rs::erange_boundary`, which compares the
*decisions*, exactly as the program does.
