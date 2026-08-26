# ERRORS.md — error-surface table (Phase C)

Derived mechanically from `c_src/src/main.c`. There are exactly **7** rejection
sites in the C (every `return 1;`), found by grepping every `return`, `fprintf`,
`errno ==`, and `!=` in the file. `main.c` contains **no** `assert`, no
`return NULL`, no error enum, and no min/max constant — the only error signals
are `errno == ERANGE`, `errno == EDOM`, `*endptr != '\0'`, `argc != 3`, and the
process exit status `1`.

Rejection sites, in source order:

| C line | condition | message | status |
|--------|-----------|---------|--------|
| 31–33 | `argc != 3` | `Usage: %s base exponent\n` | 1 |
| 41–43 | base: `errno == ERANGE` | `Range error while converting base '%s'\n` | 1 |
| 44–46 | base: `*endptr1 != '\0'` | `Invalid numeric input for base: '%s'\n` | 1 |
| 52–54 | exp: `errno == ERANGE` | `Range error while converting exponent '%s'\n` | 1 |
| 55–57 | exp: `*endptr2 != '\0'` | `Invalid numeric input for exponent: '%s'\n` | 1 |
| 63–65 | `errno == EDOM` after `pow` | `Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n` | 1 |
| 66–68 | `errno == ERANGE` after `pow` | `Range error: pow(%.2f, %.2f) caused overflow or underflow.\n` | 1 |

All messages go to **stderr**; stdout stays empty on every error path.

## The table

Test: `tests/errors.rs` (row id = `E##`). Every row asserts the SAME exit status
AND the SAME stderr bytes AND the SAME (empty) stdout from both binaries.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| E01 | `main` argc check (L31) | `argc == 1` (no args) | stderr `Usage: PROG base exponent\n`, exit 1 | [x] |
| E02 | `main` argc check (L31) | `argc == 2` (one arg: `2`) | same usage message, exit 1 | [x] |
| E03 | `main` argc check (L31) | `argc == 4` (`2 3 4`) | same usage message, exit 1 | [x] |
| E04 | `main` argc check (L31) | `argc == 11` (many args) | same usage message, exit 1 | [x] |
| E05 | `main` argc check (L31) | `argc == 0` via `execve(path, {NULL}, {NULL})` → Linux ≥5.18 rewrites to `argc==1, argv[0]==""` so `%s` prints empty (the `argv[0]==NULL` → `"(null)"` branch is unreachable on this kernel) | stderr `Usage:  base exponent\n` (two spaces), exit 1 | [x] |
| E06 | `main` argc check (L31) | `argv[0]` is an empty string, `argc==1` | `Usage:  base exponent\n`, exit 1 | [x] |
| E07 | `main` argc check (L31) | `argv[0]` contains non-UTF-8 bytes (`\xff\xfe`), argc==1 | usage message with the raw bytes echoed verbatim, exit 1 | [x] |
| E08 | `strtod` base overflow (L41) | base `1e400` (magnitude > DBL_MAX) → `ERANGE`, returns `HUGE_VAL` | `Range error while converting base '1e400'\n`, exit 1 | [x] |
| E09 | `strtod` base overflow (L41) | base `-1e400`, `1e999999`, `0x1p+50000` (hex-float overflow) | `Range error while converting base '<arg>'\n`, exit 1 | [x] |
| E10 | `strtod` base underflow (L41) | base `1e-400`, `-1e-400` (underflow to 0 → glibc sets `ERANGE`) | `Range error while converting base '<arg>'\n`, exit 1 | [x] |
| E11 | `strtod` base underflow (L41) | base `1e-320`, `5e-324`, **and `1e-308`** (**subnormal, gradual underflow**) — glibc `strtod` sets `ERANGE` even though the value is representable; note `1e-308 < DBL_MIN` so it is rejected while `2.2250738585072014e-308` is accepted | `Range error while converting base '1e-320'\n`, exit 1 | [x] |
| E12 | **precedence** ERANGE before endptr (L41 wins over L44) | base `1e400xyz` — BOTH invalid-suffix and out-of-range; C checks `errno` FIRST | `Range error while converting base '1e400xyz'\n` (NOT "Invalid numeric input"), exit 1 | [x] |
| E13 | `strtod` base bad suffix (L44) | base `abc`, `x`, `-`, `+`, `.`, `e5`, `--1`, `1..2` → no/partial conversion, `*endptr1 != 0` | `Invalid numeric input for base: '<arg>'\n`, exit 1 | [x] |
| E14 | `strtod` base bad suffix (L44) | base `12abc`, `1.5x`, `2 3`, `1,5` → partial conversion leaves trailing bytes | `Invalid numeric input for base: '<arg>'\n`, exit 1 | [x] |
| E15 | `strtod` base bad suffix (L44) | base **trailing** whitespace `"1.5 "`, `"1.5\t"`, `"1.5\n"` — leading WS is skipped by `strtod`, trailing is NOT | `Invalid numeric input for base: '1.5 '\n`, exit 1 | [x] |
| E16 | `strtod` base bad suffix (L44) | base whitespace-only `" "`, `"   "`, `"\t"`, `"\n"` — no conversion, `endptr == nptr`, so `*endptr` is the space | `Invalid numeric input for base: '<arg>'\n`, exit 1 | [x] |
| E17 | `strtod` base bad suffix (L44) | base `0x` and `0X` — glibc converts the leading `0`, leaves `x`; also `0x.`, `0xp1` | `Invalid numeric input for base: '0x'\n`, exit 1 | [x] |
| E18 | `strtod` base bad suffix (L44) | base non-UTF-8 bytes (`\xff\xfe\x80`) and embedded control bytes | `Invalid numeric input for base: '<raw bytes>'\n` echoed verbatim, exit 1 | [x] |
| E19 | `strtod` base bad suffix (L44) | base `nan(`, `inf1`, `infin`, `NANx` — partial special-value tokens | `Invalid numeric input for base: '<arg>'\n`, exit 1 | [x] |
| E20 | `strtod` exp overflow (L52) | base VALID (`2`), exponent `1e400` / `-1e400` / `1e999999` | `Range error while converting exponent '<arg>'\n`, exit 1 | [x] |
| E21 | `strtod` exp underflow (L52) | base VALID, exponent `1e-400`, `1e-320` (subnormal) | `Range error while converting exponent '<arg>'\n`, exit 1 | [x] |
| E22 | **precedence** ERANGE before endptr (L52 wins over L55) | base VALID, exponent `1e400zzz` | `Range error while converting exponent '1e400zzz'\n`, exit 1 | [x] |
| E23 | **ordering** base error reported before exponent error | base `abc` AND exponent `def` both invalid — only the BASE message is printed | `Invalid numeric input for base: 'abc'\n` only, exit 1 | [x] |
| E24 | **ordering** base ERANGE precedes exponent ERANGE | base `1e400` AND exponent `1e400` | `Range error while converting base '1e400'\n` only, exit 1 | [x] |
| E25 | `strtod` exp bad suffix (L55) | base VALID, exponent `abc`, `2x`, `" 2 "`, `""`+trailing WS, whitespace-only, `0x` | `Invalid numeric input for exponent: '<arg>'\n`, exit 1 | [x] |
| E26 | `pow` EDOM (L63) | negative base, non-integer exponent: `pow(-2, 0.5)`, `pow(-8, 1.0/3)`, `pow(-1, 2.5)` | `Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n`, exit 1 | [x] |
| E27 | `pow` EDOM (L63) | negative base, huge non-integer exponent `pow(-1.5, 1e15+0.5)`; and `-inf`-adjacent non-integer cases | `Domain error: ...`, exit 1 | [x] |
| E28 | `pow` EDOM message formatting | EDOM with base/exponent that stress `%.2f`: `-0.005`, `-1e300`, tie-rounding values | `Domain error: pow(%.2f, %.2f) ...` byte-identical | [x] |
| E29 | `pow` ERANGE overflow (L66) | `pow(10, 400)`, `pow(2, 5000)`, `pow(-10, 401)` → ±`HUGE_VAL` + `ERANGE` | `Range error: pow(10.00, 400.00) caused overflow or underflow.\n`, exit 1 | [x] |
| E30 | `pow` ERANGE underflow (L66) | `pow(10, -400)`, `pow(0.5, 5000)` → 0 + `ERANGE` | `Range error: pow(10.00, -400.00) caused overflow or underflow.\n`, exit 1 | [x] |
| E31 | `pow` ERANGE pole (L66) | `pow(0, -1)`, `pow(0, -2)`, `pow(-0.0, -1)`, `pow(-0.0, -3)` → finite-exponent divide-by-zero pole, glibc sets `ERANGE` → error. **But `pow(0, -inf)` sets NO errno** and succeeds with `Result: inf` — verified against the C, asserted as such | poles → `Range error: pow(0.00, -1.00) caused overflow or underflow.\n` exit 1; `0 -inf` → `Result: inf\n` exit 0 | [x] |
| E32 | `pow` gradual underflow does **NOT** error (L66 not taken) | `pow(10, -320)` → subnormal result. Verified: glibc's `pow` leaves `errno == 0`, so the C **succeeds** and prints `Result: 0.00`. (Contrast with `strtod`, which *does* set `ERANGE` for subnormals — E11. The two libc routines disagree and the C inherits both behaviours.) | `Result: 0.00\n`, exit **0** | [x] |
| E33 | **precedence** EDOM before ERANGE (L63 wins over L66) | an input where `pow` sets both — verified with `pow(-1e300, 1.5)`; C tests `EDOM` first | whatever the C prints — asserted identical | [x] |
| E34 | write-side failure: stdout closed | `>&-` (fd 1 closed) on the success path — C's `printf` fails, return value ignored | no stdout, exit **0** | [x] |
| E35 | write-side failure: stderr closed | `2>&-` on an error path — C's `fprintf` fails, return value ignored | no stderr, exit **1** | [x] |
| E36 | write-side failure: **SIGPIPE** | stdout is a pipe whose read end is closed → the C process is **killed by signal 13**, it does not exit | terminated by SIGPIPE (no exit status) | [x] |
| E37 | write-side failure: SIGPIPE on stderr | stderr is a closed pipe on an error path | terminated by signal 13 | [x] |
| E38 | out-of-range "enum"-like values across the boundary | this program has no enum parameter; the analogous unconstrained-integer input is the **exit status** and the raw `errno` value. Covered by asserting the full 8-bit status and signal number for every row above, and by feeding `strtod` inputs that make glibc set an errno OTHER than `ERANGE`/`EDOM` (e.g. `EINVAL` on some libcs) — the C ignores those and must still succeed | matched exactly | [x] |
| E39 | boundary: arguments one step past the valid range | `DBL_MAX` (`1.7976931348623157e308`, OK) vs the next representable decimal up (`1.7976931348623159e308` → ERANGE); `DBL_MIN`/`DBL_TRUE_MIN` vs one step below | exact boundary between success and `Range error` matched | [x] |
| E40 | boundary: zero-length and oversized args | empty base `""` (**accepted** by C: `endptr == nptr`, `*endptr == '\0'` → base `0`), and a 100 000-byte digit string argument | `""` → success; long arg → matched exactly | [x] |

## Notes on C quirks that MUST be reproduced (not fixed)

* **`""` is a valid number.** `strtod("")` performs no conversion, sets
  `endptr = nptr`, and `*endptr` is the NUL, so `*endptr != '\0'` is false and
  the C **accepts** the empty string as `0.0`. `./driver "" ""` prints
  `Result: 1.00`. Rows E40 / C-side `CONFIGS.md` row 4 lock this in.
* **`ERANGE` is checked before the suffix check** (E12, E22), so
  `1e400xyz` is a *range* error, not an *invalid input* error.
* **`errno` is never cleared between the ERANGE check and the endptr check**, so a
  stale `ERANGE` from the base conversion cannot leak into the exponent check —
  L50 resets `errno = 0` first. Verified by E24.
* **Subnormal results set `ERANGE`** in glibc `strtod` (E11) — `1e-320` is
  representable yet rejected.
* **SIGPIPE** (E36/E37): the C program has the default disposition and dies from
  signal 13. Rust's runtime installs `SIG_IGN` before `main`, which made the Rust
  binary exit 0 instead — a real divergence, fixed by restoring `SIG_DFL` at the
  top of `main`.

## Result

All **40** rows have a passing differential test in `tests/errors.rs` (35 tests)
and `tests/process_axes.rs` (10 tests, for the rows that need raw
`fork`/`execve`/pipe control: E05, E34–E37).

Every row asserts the SAME exit status, the SAME terminating signal, and the
SAME stderr/stdout bytes from both binaries — plus which C branch was taken, so
a row cannot pass by "both failed somehow".

### Divergence found and fixed

| # | divergence | fix |
|---|-----------|-----|
| 1 | **E36/E37 — SIGPIPE.** With stdout (or stderr) connected to a pipe with no reader, the C process is *killed by signal 13*; the Rust binary exited 0 instead, because the Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs and the write merely failed with `EPIPE`. | `restore_default_sigpipe()` (`signal(SIGPIPE, SIG_DFL)`) as the first statement of `main` in `src/main.rs`. Verified: both are now killed by signal 13, and neither dies when nothing is written to that stream (`e36b`). |

### Table rows corrected against the real C behaviour

Two rows were written from plausible-but-wrong assumptions and rewritten after
running the C:

* **E32** — glibc's `pow` does **not** set `ERANGE` for a subnormal (gradual
  underflow) result, so `driver 10 -320` **succeeds** with `Result: 0.00`.
* **E11/E39** — glibc's `strtod` *does* set `ERANGE` for subnormals, but only for
  *inexact* underflow: `0x1p-1023` and `0x1p-1074` are accepted, and
  `2.2250738585072013e-308` is accepted because it rounds up to `DBL_MIN`, while
  `2.2250738585072012e-308` is rejected.
