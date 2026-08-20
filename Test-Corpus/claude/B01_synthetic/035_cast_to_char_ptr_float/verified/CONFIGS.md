# CONFIGS.md — Phase B configuration-surface table

## Build-time configuration axes

### Cargo features

The crate as translated had **no `[features]` section**, i.e. exactly one
configuration. One feature was added during verification, `c_main`, to make
the shared-object symbol diff against the C `.so` reach empty (see
SYMBOLS.md §3). `default = []`, so the default build is unchanged.

```toml
[features]
default = []
c_main = []
```

The complete set of valid feature combinations is therefore the power set of
`{c_main}`:

| # | feature combination | effect |
|---|---------------------|--------|
| 1 | *(none)* — `--no-default-features` | `driver` exported from the lib; `main` emitted by the `[[bin]]` |
| 2 | `c_main` | `driver` **and** a C-ABI `main` exported from the lib; `src/main.rs` becomes `#![no_main]` and the exported `main` is the entry point |

Enumerated mechanically (not hard-coded) and checked by
`./check_features.sh`, which parses the `[features]` table out of
`Cargo.toml` and loops over its power set.

### CMake configuration

`c_src/CMakeLists.txt` declares no `option()`, no `if()`, and no
`target_compile_definitions`. `main.c` contains no `#ifdef`, no `#if` and no
conditional compilation of any kind. One configuration.

### Cargo profiles

Both `debug` and `release` are exercised, because `[profile.release] panic =
"abort"` is a real behavioural difference between them and float codegen can
differ with optimisation level. Verified by `./run_all.sh`, which runs the
whole differential suite under both profiles.

**Total build configurations to verify: 2 feature combos × 2 profiles = 4.**
All four are swept by `./run_all.sh`, which re-runs every Phase B and Phase C
test in each of them.

## Runtime configuration axes (derived from what the C branches on)

`main.c` itself has no flags, no options and no modes — its only runtime
input is the byte stream on `stdin`. The branching that matters therefore
lives in the `scanf("%f")` conversion the C calls, and the Rust translation
reimplements that branching explicitly in `scan_float` / `c_strtof` /
`assemble_f32`. The axes below are the ones those branches actually
distinguish (cross-referenced against the `if`/`while`/`match` arms of
`src/lib.rs`, each of which mirrors a glibc `vfscanf`/`strtof` branch):

* **A. leading whitespace** — none / space / tab / `\n` / `\r` / `\v` / `\f` /
  mixed run / whitespace-then-EOF
* **B. sign** — absent / `+` / `-`
* **C. subject form** — decimal integer / decimal with point / point-leading
  (`.5`) / point-trailing (`5.`) / hex (`0x…`) / hex with point / `inf` /
  `infinity` / `nan` / `nan(payload)`
* **D. exponent** — absent / `e` / `E` / `p` / `P` / with `+` / with `-` /
  no digits after / digit string wider than 64 bits
* **E. magnitude class** — `+0` / `-0` / subnormal / smallest normal /
  ordinary normal / `FLT_MAX` / overflow→`±inf` / underflow→`±0` /
  exact halfway (ties-to-even) / just above and just below a halfway point
* **F. significand length** — 0 digits / 1 / a few / 9 (exactly round-trip) /
  17 / 40 / >1000 digits, and hex mantissas short enough to fit the 60-bit
  accumulator vs long enough to set the sticky bit
* **G. trailing bytes after the token** — none (EOF) / whitespace / newline /
  alphabetic junk / a second number / `)`
* **H. byte-level hostility** — embedded NUL / bytes ≥ 0x80 / non-UTF-8
  sequences / `\r\n` line endings / very long line
* **I. entry point** — the process (`main`, stdin→stdout, the top level) *and*
  the exported low-level `driver(float)` called directly through the `.so`
  via `libloading` (bypasses parsing entirely, so it isolates the
  object-representation printing) *and* `print_hex` semantics observed
  through `driver`

Axis I is the "lowest-level entry point" requirement: `print_hex` is `static`
in C so `driver` *is* the lowest callable level, and it is tested directly
through the FFI boundary rather than only through the `main` wrapper.

## Configuration table

Each row is a combination the C treats differently. Every row is driven with
**many randomized inputs** from fixed seeds (the SplitMix64 generator in
`tests/common/corpus.rs`), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `main` | A=none, B=absent, C=decimal integer, F=1 digit (`"0"`, `"7"`) | [x] |
| 2 | `main` | A=none, B=`-`/`+`, C=decimal integer, E=`±0` (`"-0"`, `"+0"`) | [x] |
| 3 | `main` | A=none, B=absent, C=decimal integer, F=many/leading zeros (`"007"`, `"0000"`) | [x] |
| 4 | `main` | A=none, B=all, C=decimal with point, D=absent (`"1.5"`, `"-2.25"`) | [x] |
| 5 | `main` | A=none, C=point-leading (`".5"`, `"-.5"`, `"+.5"`) | [x] |
| 6 | `main` | A=none, C=point-trailing (`"5."`, `"-5."`) | [x] |
| 7 | `main` | C=decimal, D=`e` unsigned (`"1e5"`) | [x] |
| 8 | `main` | C=decimal, D=`E` unsigned (`"1E5"`) | [x] |
| 9 | `main` | C=decimal, D=`e+` / `e-` (`"1e+5"`, `"1e-5"`) | [x] |
| 10 | `main` | C=decimal, D=`e` with no digits (`"1e"`, `"1e+"`, `"1e-"`) | [x] |
| 11 | `main` | C=decimal, D=exponent wider than i64 (`"1e999999999999999999999"`, `"1e-99…9"`) | [x] |
| 12 | `main` | C=decimal, E=`+0` mantissa with huge exponent (`"0e999999999999999999999"`) | [x] |
| 13 | `main` | C=hex, F=1 hex digit, D=absent (`"0x1"`, `"0xf"`, `"0XF"`) | [x] |
| 14 | `main` | C=hex with point, D=absent (`"0x1.8"`, `"0x.8"`, `"0x1."`) | [x] |
| 15 | `main` | C=hex, D=`p`/`P` signed and unsigned (`"0x1p4"`, `"0x1P-4"`, `"0x1.8p+1"`) | [x] |
| 16 | `main` | C=hex, D=`p` with no digits (`"0x1p"`, `"0x1p+"`) | [x] |
| 17 | `main` | C=hex, F=mantissa short enough to fit the 60-bit accumulator exactly | [x] |
| 18 | `main` | C=hex, F=mantissa long enough to overflow the accumulator → **sticky bit** path (`"0x123456789abcdef0123456789abcdef"`, `"0x1."+"f"*200+"p0"`) | [x] |
| 19 | `main` | C=hex, E=overflow (`"0x1p128"`, `"0x1.ffffffp127"`) | [x] |
| 20 | `main` | C=hex, E=subnormal / underflow boundary (`"0x1p-126"`, `"0x1p-149"`, `"0x1p-150"`, `"0x0.000002p-126"`) | [x] |
| 21 | `main` | C=`inf`, all cases of B and letter case (`"inf"`, `"INF"`, `"iNf"`, `"-inf"`, `"+inf"`) | [x] |
| 22 | `main` | C=`infinity`, all letter cases (`"infinity"`, `"INFINITY"`, `"InFiNiTy"`, `"-infinity"`) | [x] |
| 23 | `main` | C=`inf` + G=trailing junk that is *not* `i` (`"inf1"`, `"infx"`, `"inf "`) | [x] |
| 24 | `main` | C=`nan`, all cases of B and letter case (`"nan"`, `"NAN"`, `"NaN"`, `"-nan"`, `"+nan"`) | [x] |
| 25 | `main` | C=`nan(payload)` (`"nan(1)"`, `"nan(123)"`, `"nan(0x7f)"`, `"-nan(5)"`) | [x] |
| 26 | `main` | A=single space / tab / newline / CR / VT / FF before the token | [x] |
| 27 | `main` | A=long mixed whitespace run before the token (`"  \n\t \v\f\r  1.5"`) | [x] |
| 28 | `main` | G=trailing newline / trailing spaces / second number (`"1.5\n"`, `"1 2"`, `"1\n2"`) | [x] |
| 29 | `main` | G=trailing alphabetic junk (`"1.5abc"`, `"0x1p3xyz"`) | [x] |
| 30 | `main` | E=exact halfway, ties-to-even **down** (`"8388608.5"`, `"16777215.5"`) | [x] |
| 31 | `main` | E=exact halfway, ties-to-even **up** (`"8388609.5"`, `"8388610.5"`) | [x] |
| 32 | `main` | E=one ULP either side of a halfway point (`"1.00000005960464477539062"`, `"0.99999999999999999999"`) | [x] |
| 33 | `main` | E=`FLT_MAX` and its exact decimal expansion (39 digits) | [x] |
| 34 | `main` | E=first value that overflows (`"3.4028236e38"`, `"1e39"`) | [x] |
| 35 | `main` | E=smallest normal / largest subnormal / smallest subnormal / half of it | [x] |
| 36 | `main` | F=>1000 significant digits (`"1."+"2"*1000`, `"1"+"0"*500`, `"0."+"0"*500+"1"`) | [x] |
| 37 | `main` | E=random `f32` bit pattern → `repr()`, `%.17g` and `%a` (hex) round-trip, 3×500 per seed | [x] |
| 38 | `main` | C=random decimal literal, all of B/D/F randomized, 500 per seed | [x] |
| 39 | `main` | C=random hex literal, all of B/D/F randomized, 500 per seed | [x] |
| 40 | `main` | random junk soup over the alphabet `+-.eEpPxX inf0-9`, 500 per seed | [x] |
| 41 | `main` | random (leading whitespace ⧺ fixed token ⧺ trailing junk) triples, 500 per seed | [x] |
| 42 | `main` | random mantissa ⧺ `e` ⧺ exponent in [-60, 60] — straddles the whole normal/subnormal/overflow range, 500 per seed | [x] |
| 43 | `main` | random 20–60 digit near-tie significands × exponent in [-50, 50], 500 per seed | [x] |
| 44 | `main` | random 14–40 hex-digit mantissa × `p` exponent in [-160, 160] — sticky-bit path, 500 per seed | [x] |
| 45 | `main` | H=embedded NUL byte, leading and after digits (`"\0"`, `"\x001"`, `"1\x002"`) | [x] |
| 46 | `main` | H=raw bytes ≥ 0x80 / invalid UTF-8 (`b"\x80\xff"`, `b"1\xc3"`, `b"\xff\xfe1.5"`) | [x] |
| 47 | `main` | H=`\r\n` line endings, and a 64 KiB single line | [x] |
| 48 | `driver` (via `.so`) | E=`+0.0`, `-0.0` | [x] |
| 49 | `driver` (via `.so`) | E=`±FLT_MIN` (smallest normal), `±FLT_TRUE_MIN` (smallest subnormal) | [x] |
| 50 | `driver` (via `.so`) | E=`±FLT_MAX`, `±1.0`, `±0.5`, `±3.14159` | [x] |
| 51 | `driver` (via `.so`) | E=`±inf` | [x] |
| 52 | `driver` (via `.so`) | E=quiet NaN (`7fc00000`), negative qNaN (`ffc00000`) | [x] |
| 53 | `driver` (via `.so`) | E=**signalling** NaN (`7fa00000`) and NaN payloads with no "valid variant" — every bit pattern is a legal `float` input | [x] |
| 54 | `driver` (via `.so`) | E=all 256 values of each byte lane, i.e. bit patterns `0x00000000..0xff000000` stepped per lane | [x] |
| 55 | `driver` (via `.so`) | E=randomized 32-bit patterns, 20 000 per seed, uniform over the whole space | [x] |
| 56 | `driver` (via `.so`) | E=exhaustive-by-sampling sweep of the full 2^32 space (stride 65 521, a prime → hits every exponent/mantissa residue class) | [x] |
| 57 | `main` | I=stdin closed (`0<&-`) — read fails immediately rather than returning EOF | [x] |
| 58 | `main` | I=stdin is a slow/chunked pipe delivering one byte at a time (exercises the short-read loop in `ByteReader::getc`) | [x] |
| 59 | `main` | build profile = `debug` — all of rows 1–58 re-run | [x] |
| 60 | `main` | build profile = `release` (`panic = "abort"`) — all of rows 1–58 re-run | [x] |
| 61 | `main`, `driver` | features = *(none)* — all of rows 1–58 re-run in both profiles | [x] |
| 62 | `main`, `driver` | features = `c_main` (`#![no_main]` binary, C-ABI `main` export) — all of rows 1–58 re-run in both profiles | [x] |

62 rows.

## Where each row is verified

| rows | test |
|---|---|
| 1–47 | `tests/exe_diff.rs` (one `#[test]` per row group) |
| 48–56 | `tests/ffi_diff.rs` (`driver` loaded via `libloading`) |
| 57–58 | `tests/exe_diff.rs::row57_closed_stdin`, `row58_byte_at_a_time_stdin` |
| 59–62 | `./run_all.sh` sweeps the 2 × 2 matrix and re-runs everything above |

Rows 37–44 additionally supply **randomized** inputs (2 000 per seed × 3
seeds each, from the seeded SplitMix64 generator in
`tests/common/corpus.rs`), so no row is checked off on the strength of a
single hand-picked value.
