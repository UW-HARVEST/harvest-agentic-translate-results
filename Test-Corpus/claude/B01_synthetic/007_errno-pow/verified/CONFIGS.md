# CONFIGS.md — configuration-surface table (Phase B)

## Axes the C actually branches on

The program has **no runtime options, flags or modes** — no `getopt`, no
environment variable is read, no `setlocale`, no `#ifdef`. Grepping `main.c` for
`if`/`switch` yields only the 7 error branches in `ERRORS.md`. The configuration
surface is therefore entirely made of **input shapes**, and it is the *libc*
routines the program delegates to (`strtod`, `pow`, `printf("%.2f")`) that branch:

| axis | values the code distinguishes |
|------|-------------------------------|
| A. `argc` | 0, 1, 2, **3 (the only valid one)**, 4, many |
| B. base/exponent *lexical* form (`strtod` grammar) | plain decimal, leading `+`/`-`, leading whitespace (space/`\t`/`\n`/`\v`/`\f`/`\r`), no integer part (`.5`), no fraction part (`5.`), exponent `e`/`E` with/without sign, hex-float `0x1p3`/`0X1.8P-2`, `inf`/`infinity`/`INF` any case, `nan`/`NAN`/`nan(chars)`, empty string, very long digit strings |
| C. base/exponent *numeric magnitude* | 0, ±0.0, subnormal, `DBL_MIN`, small, 1, integers, huge, `DBL_MAX`, ±inf, NaN |
| D. `pow` argument domain (glibc's own branch table) | `y == 0`, `y == 1`, `x == 1`, `x == 0` w/ `y>0`, `x == 0` w/ `y<0` (pole), `x < 0` w/ integer `y` (odd/even), `x < 0` w/ non-integer `y` (EDOM), `x == -1` w/ `±inf` `y`, `|x| < 1` w/ `±inf` `y`, `|x| > 1` w/ `±inf` `y`, NaN operands, `±inf` base, overflow, underflow (to 0 and to subnormal) |
| E. `%.2f` output shape | `0.00`, `-0.00`, exact-tie rounding (`x.xx5` exactly representable → round-half-even), 3-digit-precision inputs, huge values (up to 309 integer digits), `inf`, `-inf`, `nan`, `-nan` |
| F. stdio destination | stdout is a pipe / a file / closed; stderr is a pipe / a file / closed; pipe with closed reader (SIGPIPE) |
| G. process environment | empty env, `LC_ALL`/`LC_NUMERIC` set to a comma-decimal locale (must have **no** effect: the C never calls `setlocale`, so it stays in the `"C"` locale) |
| H. entry point | **the process itself is the only entry point** (`main`, via `argc`/`argv`). There is no library API and no convenience wrapper: every row below drives the full pipeline `argv → strtod(base) → strtod(exp) → pow → printf`, which is the lowest level available. |

## The table

Test file per row group; row id = `Cnn`. Every row runs **both** binaries and
compares stdout bytes, stderr bytes, exit status and terminating signal.
"random ×N" = N property-style randomized inputs from a fixed seed
(`SEED = 0x5EED_1234`), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C01 | `main` full pipeline | argc==3, both args plain decimal integers, small magnitudes — random ×400 (`-50..50` × `-20..20`) | [x] |
| C02 | `main` full pipeline | argc==3, both args random finite decimals with 1–17 significant digits — random ×600 | [x] |
| C03 | `main` full pipeline | base random, exponent **exactly 0** (`0`, `-0`, `0.0`, `+0.0`) → `pow(x,0)==1` for every x incl. NaN/inf — random ×120 | [x] |
| C04 | `main` full pipeline | base **empty string** `""` (C quirk: accepted as 0.0), exponent varied incl. `""`, `0`, `1`, `-1` | [x] |
| C05 | `main` full pipeline | base/exponent with **leading whitespace** — each of `' '`, `\t`, `\n`, `\v`, `\f`, `\r`, and runs of mixed WS, before a random number ×200 | [x] |
| C06 | `main` full pipeline | explicit `+`/`-` sign, `.5` (no integer part), `5.` (no fraction part), `+.5e+1`, `-.0` — random ×200 | [x] |
| C07 | `main` full pipeline | **exponent notation**: `e`/`E`, with `+`/`-`/absent sign, 1–3 exponent digits, leading zeros in exponent (`1e005`) — random ×300 | [x] |
| C08 | `main` full pipeline | **hex float** literals `0x1p3`, `0X1.8P-2`, hex mantissa 1–13 hex digits, binary exponent −60..60, upper/lower case — random ×300 | [x] |
| C09 | `main` full pipeline | **`inf` / `infinity`** in every case variant (`inf`, `INF`, `Inf`, `infinity`, `INFINITY`, `iNfInItY`) with `+`/`-`, as base and/or exponent — full cross product | [x] |
| C10 | `main` full pipeline | **`nan`** variants (`nan`, `NAN`, `NaN`, `-nan`, `+nan`, `nan(0)`, `nan(1234)`, `nan(x_9)`) as base and/or exponent — full cross product (exercises `-nan` printing) | [x] |
| C11 | `main` full pipeline | `pow` **negative base, integer exponent** — odd vs even, positive vs negative, incl. large odd/even (`-2 ^ ±1..±40`) — random ×300 | [x] |
| C12 | `main` full pipeline | `pow` **negative base, non-integer exponent** → EDOM (also in ERRORS E26) — random ×150 | [x] |
| C13 | `main` full pipeline | `pow` **zero base**: `0`/`-0.0` × exponent `>0`, `<0`, `0`, `inf`, `-inf`, NaN — full cross product (poles + ERANGE) | [x] |
| C14 | `main` full pipeline | `pow` **±inf base** × exponent `<0`, `0`, `>0`, odd/even integer, `±inf`, NaN — full cross product | [x] |
| C15 | `main` full pipeline | `pow` **±inf exponent** × base `-1`, `1`, `|x|<1`, `|x|>1`, `0`, NaN — full cross product (the `pow(-1,inf)==1` special case) | [x] |
| C16 | `main` full pipeline | `pow` **base == 1** and **base == -1** with arbitrary/huge/NaN exponents — random ×150 | [x] |
| C17 | `main` full pipeline | **near-overflow boundary**: exponents chosen so `pow` lands just under / just over `DBL_MAX` (`10^±(307..309)`, `2^(1023..1025)`) — random ×200 | [x] |
| C18 | `main` full pipeline | **near-underflow boundary**: results in the subnormal band and just below (`10^-(307..324)`, `2^-(1022..1080)`) — random ×200 | [x] |
| C19 | `main` full pipeline | **`%.2f` tie rounding**: results whose exact binary value ends in `...5` at the 3rd decimal (`x.125`, `x.375`, `x.625`, `x.875`, `0.0625`), reached via `pow` — round-half-even, random ×200 | [x] |
| C20 | `main` full pipeline | **`%.2f` huge output**: results with 100–309 integer digits (`10^100`, `10^308`, `2^1023`) — full digit expansion must match | [x] |
| C21 | `main` full pipeline | **`%.2f` tiny output**: results that print as `0.00` / `-0.00` (`10^-30`, `-2^-1000`), incl. negative-zero results `pow(-0.0, 3)` | [x] |
| C22 | `main` full pipeline | **DBL_MAX / DBL_MIN / DBL_TRUE_MIN / eps** as literal args (`1.7976931348623157e308`, `2.2250738585072014e-308`, `5e-324`, `2.220446049250313e-16`) × exponents `1`, `-1`, `0.5`, `2` | [x] |
| C23 | `main` full pipeline | **17-significant-digit round-trip** decimals (worst case for correct decimal→binary rounding) — random ×400 bit-patterns re-serialized as `{:.17e}` | [x] |
| C24 | `main` full pipeline | **raw random f64 bit patterns** (any exponent/mantissa, incl. subnormals & NaNs) rendered as hex floats `%a` so the exact bits reach `strtod` — random ×500 | [x] |
| C25 | `main` full pipeline | **very long argument strings**: 1 000 / 10 000 / 100 000 digit mantissas, long zero-padding (`0000…1`), long fraction tails | [x] |
| C26 | `main` (arg count) | argc == 0, 1, 2, 4, 11 (usage path — shares `ERRORS.md` E01–E05) | [x] |
| C27 | `main` + stdio axis F | success path with stdout → **pipe**, → **regular file**, → `/dev/null`, → **closed fd**; error path with stderr → pipe / file / closed fd | [x] |
| C28 | `main` + stdio axis F | stdout is a pipe with a **closed reader** (SIGPIPE, signal 13) on the success path; same for stderr on an error path | [x] |
| C29 | `main` + env axis G | success + EDOM + ERANGE paths run with `LC_ALL=de_DE.UTF-8`, `LC_NUMERIC=de_DE.UTF-8`, `LC_ALL=C`, and a **completely empty environment** — output must be identical in all four (no `setlocale`, so decimal point stays `.`) | [x] |
| C30 | `main` + argv encoding | args that are **not valid UTF-8** (`\xff`, `\x80\x81`, `\xc3` truncated) and args containing `%`/`\\`/quote characters — echoed verbatim in the error text | [x] |
| C31 | `main` full pipeline | **fully random arg pairs from a mixed generator** (any of the shapes above, valid or invalid, 10 % pure fuzz bytes) — random ×3000, seeded | [x] |
| C32 | `main` full pipeline | integer-valued exponents at the **`pow` odd/even/integrality decision boundary**: `2^53`, `2^53+1`, `1e15`, `1e16`, `0.5`, `1.5`, `-2.5`, with negative bases | [x] |
| C33 | `main` full pipeline | **concentrated `%.2f` formatter sweep**: `pow(x,1) == x`, so `driver <x> 1` prints `%.2f` of `x` itself. 2 000 random raw f64 bit patterns passed as exact hex floats + 1 000 values spread over decimal exponents −320…308 with full mantissas. (The formatter is the ONLY part the Rust implements itself instead of delegating to libc, so it gets the densest coverage.) | [x] |
| C34 | `main` full pipeline | **every binade, exhaustively**: `2^k` for k = −1074…1023 (all 2 098 exponents, normal *and* subnormal) and their negations, as exact hex floats — no magnitude class of the `%.2f` conversion is left untested | [x] |

Rows C01–C25 and C31–C32 are the *valid/mixed* path; C26–C30 pin the
process-level axes. Every row is checked off only after the whole randomized
batch for that row passes.

## Result

All **34** rows pass in `tests/configs.rs` (29 tests) and
`tests/process_axes.rs` (rows C26–C30), across roughly **17 000 randomized
argument pairs** from the fixed seed `0x5EED_1234`, plus the exhaustive
sweeps (all 2 098 binades in C34, the full inf/nan spelling cross-products in
C09/C10, and the −5..5 × −5..5 integer grid in C01).

An independent third oracle in `tests/ffi_libloading.rs` re-implements `main.c`
on top of `strtod`/`pow`/`__errno_location`/`snprintf` resolved at runtime with
`libloading`, and agrees with both binaries on every case it is given.

### Negative control (proof the rows actually discriminate)

Four mutant Rust binaries were built and run through the suite; each was caught:

| mutant | change | caught by |
|--------|--------|-----------|
| m1 | `{:.2}` → `{:.3}` | all 29 `configs.rs` rows |
| m2 | `EDOM` constant changed so the branch never matches | 12 `configs.rs` rows + `errors.rs` |
| m3 | `restore_default_sigpipe()` removed | `process_axes.rs` E36/E37 (only) |
| m4 | base `ERANGE` check disabled | 3 `configs.rs` rows + `errors.rs` |

m3 is the important one: it is invisible to every argv-only test and is caught
only by the dead-pipe rows, which is why the process-level axes (F) are part of
the configuration surface.
