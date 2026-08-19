# CONFIGS.md — configuration-surface table (Phase B)

## Axes the C code actually branches on

`c_src/src/main.c` has no runtime options, no flags, no `#ifdef`s and no
compile-time configuration (`grep -c '#if' c_src/src/main.c` → 0; `Cargo.toml`
declares no `[features]`, `CMakeLists.txt` declares no options). What it *does*
branch on is data:

| axis | where in the C | values that are treated differently |
|---|---|---|
| A1 entry point | `void run(house_t*, int)` (external), `int main(void)` (external), plus the process itself | `run` via FFI, `main` via FFI, `driver` executable |
| A2 `house->floors` | `add_floor`: `house->floors++` | `0`, `±1`, `2` (program default), `INT_MAX` (overflow), `INT_MIN`, random |
| A3 `house->bedrooms` | `add_bedrooms`: `house->bedrooms += extra` | `5` (default), `0`, `INT_MAX`, `INT_MIN`, random |
| A4 `extra_bedrooms` | argument of `run`/`add_bedrooms` | `0`, `±1`, `INT_MAX`, `INT_MIN`, random `int` |
| A5 `house->bathrooms` | `bathrooms += 1.0` and `printf("%.1f")` | exact 1-decimal, exact rounding ties (`.25`/`.75`), near-ties (`.05`, `.35`, `2.05`), `±0.0`, subnormals, ≥2⁵³/10 (where `%.1f` has no fraction to round), `1e300`/`DBL_MAX`, `±Inf`, `NaN`/`-NaN` |
| A6 call sequencing | `main` calls `run` **twice** on the same struct; each `run` calls `print_house` 4× | 1 call vs 2 consecutive calls on the carried-over state |
| A7 stdin content shape | `fgets(in, 100, stdin)` then `strtol(in, &endp, 10)` | sign, leading whitespace class, leading zeros, trailing garbage, `0x`/exponent forms, line length vs the 99-byte cap, presence of `\n`, extra lines after the first, embedded NUL, high bytes, empty |
| A8 numeric text range | `strtol` + `INT_MIN`/`INT_MAX` guard | inside `int`, `int` boundaries, between `int` and `long`, `long` boundaries, beyond `long` |

`print_house`'s format string is a single `printf` with `%d`, `%d`, `%.1f`, so
the interesting cross-product is (A2 × A3 × A4) for the two integer fields and
(A5 × A6) for the double.

## Configuration table

`run` rows are driven through the exported `run` symbol of both `.so`s
(the *lowest-level* public entry point) with the resulting `house_t` compared
bit-for-bit in addition to stdout. `main`/executable rows are driven through
**both** the exported `main` symbol (FFI, forked for stdio isolation) **and**
the two executables (stdout + stderr + exit status + signal).

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| R01 | `run` (FFI) | program default state `{2, 5, 2.5}`, `extra = 7`; also `extra ∈ {0, ±1}` | [x] |
| R02 | `run` (FFI) | `floors ∈ {0, -1, 1, INT_MAX-1, INT_MAX, INT_MIN}` × nominal rest (`++` at the overflow edge) | [x] |
| R03 | `run` (FFI) | `bedrooms ∈ {INT_MAX, INT_MAX-1, INT_MIN, INT_MIN+1, 0}` × `extra ∈ {INT_MIN, -1, 0, 1, INT_MAX}` (signed `+=` overflow both directions, full 5×5 cross-product) | [x] |
| R04 | `run` (FFI) | `extra ∈ {0, 1, -1, INT_MAX, INT_MIN}` × nominal house; plus 512 random `int`s | [x] |
| R05 | `run` (FFI) | `bathrooms` = exact 1-decimal values `k/10` for `k ∈ [-2000, 2000]` sampled, and `x.0 … x.9` for several magnitudes | [x] |
| R06 | `run` (FFI) | `bathrooms` = exact ties for `%.1f`: `n + 0.25`, `n + 0.75`, `n/4`, `n/8`, `n/2^k` (round-half-to-even) | [x] |
| R07 | `run` (FFI) | `bathrooms` = near-tie decimals whose nearest double falls just below/above `x.x5`: `0.05, 0.15, 0.25, 0.35, 0.45, …, 2.05, 8.3, 1.15, 0.145, 4.35` | [x] |
| R08 | `run` (FFI) | `bathrooms ∈ {0.0, -0.0}` (sign of zero must survive `%.1f` and `+= 1.0`) | [x] |
| R09 | `run` (FFI) | `bathrooms ∈ {+Inf, -Inf, NaN, -NaN, NaN with payload, signalling NaN}` | [x] |
| R10 | `run` (FFI) | `bathrooms` subnormal / tiny: `5e-324`, `-5e-324`, `2.2250738585072014e-308`, `±1e-300`, `±1e-16` | [x] |
| R11 | `run` (FFI) | `bathrooms` large: `2⁵³/10` neighbourhood (`9.007199254740992e14 ± ulp`), `1e15`, `2⁵³`, `1e16`, `1e300`, `DBL_MAX`, and the negatives of each | [x] |
| R12 | `run` (FFI) | `bathrooms` where `+= 1.0` is lossy or saturating: `2⁵³`, `2⁵³-1`, `2⁵²+0.5`, `1e16`, `DBL_MAX`, `-1.0`, `-0.5` | [x] |
| R13 | `run` (FFI) | 20 000 uniformly random *bit patterns* for `bathrooms` × random `floors`/`bedrooms`/`extra` (includes NaNs/Inf/subnormals) | [x] |
| R14 | `run` (FFI) | 20 000 random "mixed" doubles (quarters, tenths, ×10^±k, huge integers) × random ints | [x] |
| R15 | `run` (FFI) | two consecutive `run` calls on the same struct (what `main` does) for every special `bathrooms` value of R06–R12 | [x] |
| R16 | `run` (FFI) | 20 000 random decimal-looking doubles `±m/10^d` (`m` up to 18 digits, `d` ≤ 18) — the densest source of `%.1f` rounding decisions; plus 4 000 of them through two consecutive `run` calls | [x] |
| R17 | `run` (FFI) | structured sweep of the hardest `%.1f` inputs: every `k/10` for k in [-5000, 5000] **and both ulp neighbours** (30 003 values), every dyadic `n/2^k` for n in [-1500, 1500], k in 1..=12 (36 012 values), and those dyadics scaled by 10^{-3,-1,1,3,6,12,15} (44 100 values) | [x] |
| M01 | `main` (FFI) + both exes | plain non-negative decimal: `"0\n"`, `"7\n"`, `"12345\n"`, `"2147483646\n"` | [x] |
| M02 | `main` (FFI) + both exes | explicit sign: `"-7\n"`, `"+7\n"`, `"-0\n"`, `"+0\n"` | [x] |
| M03 | `main` (FFI) + both exes | leading whitespace classes accepted by `strtol`: `" 7\n"`, `"\t7\n"`, `"\v7\n"`, `"\f7\n"`, `"\r7\n"`, `"\n7\n"` (first line empty → rejection), mixtures, whitespace + sign | [x] |
| M04 | `main` (FFI) + both exes | leading zeros: `"007\n"`, `"0000000000000000000000000007\n"`, 90 zeros + `"7\n"`, `"-007\n"` | [x] |
| M05 | `main` (FFI) + both exes | trailing garbage after a valid prefix: `"7abc\n"`, `"7 8\n"`, `"7.9\n"`, `"0x10\n"`, `"7e3\n"`, `"12,34\n"`, `"5-\n"` | [x] |
| M06 | `main` (FFI) + both exes | no trailing newline (`"7"`, `"-7"`, `"abc"`, `""`) — `fgets` stops at EOF | [x] |
| M07 | `main` (FFI) + both exes | empty stdin (0 bytes) → `fgets` returns NULL, buffer stays `""` | [x] |
| M08 | `main` (FFI) + both exes | newline-only / blank first line (`"\n"`, `"\n7\n"`, `"\r\n"`) | [x] |
| M09 | `main` (FFI) + both exes | `int` boundaries exactly: `"2147483647\n"`, `"-2147483648\n"`, `"2147483646\n"`, `"-2147483647\n"` | [x] |
| M10 | `main` (FFI) + both exes | between `int` and `long`: `"2147483648\n"`, `"-2147483649\n"`, `"4294967296\n"`, `"9223372036854775807\n"` | [x] |
| M11 | `main` (FFI) + both exes | beyond `long` (ERANGE): `"9223372036854775808\n"`, `"-9223372036854775809\n"`, 30/60/98/99 digit numbers, `"1"` + 98 zeros | [x] |
| M12 | `main` (FFI) + both exes | line length vs the 99-byte `fgets` cap: 96, 97, 98, 99, 100, 101 and 150 bytes x {digits only, digits + trailing garbage, padding + digit, zeros + digit} x {with, without} a trailing `\n` | [x] |
| M13 | `main` (FFI) + both exes | multi-line stdin — only the first line may be consumed (`"7\n9\n"`, `"abc\n7\n"`, 100 lines) | [x] |
| M14 | `main` (FFI) + both exes | embedded NUL bytes (`"\0"`, `"\0007\n"`, `"7\0 9\n"`, `"12\0"`) | [x] |
| M15 | `main` (FFI) + both exes | non-UTF-8 / high bytes (`0xFF`, `0x80`, `0xC3 0x28`, byte 0x00–0xFF sweep prefixes) | [x] |
| M16 | `main` (FFI) + both exes | CRLF and lone CR (`"7\r\n"`, `"7\r"`, `"\r7\n"`) | [x] |
| M17 | `main` (FFI) + both exes | oversized stdin: 100 KiB single line, 100 KiB with no newline, 100 KiB of whitespace | [x] |
| M18 | `main` (FFI) + both exes | 384 random byte strings (length 0-120, alphabet biased to digits/signs/whitespace/NUL/high bytes), plus 768 longer ones (0-260 bytes) through the executables | [x] |
| M19 | `main` (FFI) + both exes | 512 random decimal texts around the `int`/`long` boundaries (2^31 +- d, 2^63 +- d, random 1-40 digit numbers, random `i64`/`i32`/`u32` values, random prefixes/suffixes), plus 1024 more through the executables | [x] |
| M20 | `main` (FFI) + both exes | whitespace-only lines: each of `' '`, `'\t'`, `'\v'`, `'\f'`, `'\r'` x lengths 1, 2, 98, 99, 100, 101 x {with, without} a trailing `\n` | [x] |
| M21 | `main` (FFI) + both exes | a number split by the 99-byte cap: 88-99 spaces of padding followed by a 13-digit number, a negative one, and a 22-digit one | [x] |

Both `.so`s' `main` is additionally checked to return the same `int` (always 0)
and to terminate the same way (same exit status, no signal), and both
executables are checked for identical stderr (always empty) and exit status.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|--------------------------------------------|---|
| M22 | both exes | locale environment (`LC_ALL`/`LC_NUMERIC`/`LANG`/`LANGUAGE` = `C`, `POSIX`, `en_US.UTF-8`, `de_DE.UTF-8`, `fr_FR.UTF-8`, `tr_TR.UTF-8`, `ru_RU.UTF-8`, an invalid name, empty) × valid/invalid input — neither program calls `setlocale`, so `%.1f` must keep `.` and `strtol` the C whitespace class | [x] |
| M23 | both exes | command-line arguments present (`main` takes none): none, one, several, empty strings, embedded spaces/newlines | [x] |
| M24 | both exes | stdout is a **regular file** (fully buffered in C) rather than a pipe | [x] |
| M26 | both exes | stdin delivered in slow chunks over a pipe (several `read()` syscalls before the newline/EOF arrives), with and without a final newline, incl. an empty write then close | [x] |
| M25 | both exes | stdin is a **regular file** (seekable; `fgets` may read ahead) × single line / multi-line / unterminated / 99-byte / 250-byte shapes, with stdout to a pipe and to a file | [x] |

## Systematic (exhaustive, not sampled) sweeps — `tests/exhaustive.rs`

Enumerating whole input spaces rather than trusting the hand-picked rows above:

| # | entry point(s) | configuration | ✔ |
|---|----------------|---------------|---|
| S01 | both exes | every one of the 256 byte values alone, and each followed by `\n` (512 inputs) | [x] |
| S02 | both exes | every byte value placed before a number, after a number, and between a sign and a digit (768 inputs) | [x] |
| S03 | both exes | all 2-byte strings over the 22-byte alphabet `0129+- \t\n\r\v\f xX.\0\xff89aeE` (484 inputs) | [x] |
| S04 | both exes | all 3-byte strings over the 16-byte alphabet `09+- \t\n\r\0\xff2x.E7` (4096 inputs) | [x] |
| S05 | both exes | digit strings of every length 1…25 (of `0`, `1`, `9`) × {no sign, `-`, `+`} × {with, without} `\n`, plus the exact decimal texts of `0`, `INT_MAX`, `INT_MIN`, `UINT_MAX`, `LONG_MAX`, `LONG_MIN`, `ULONG_MAX` and their ±1/±2 neighbours (≈300 inputs) | [x] |
