# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation

Axes the C code actually distinguishes, read off `c_src/src/driver.c` and
`c_src/include/driver.h`:

* **Public entry points (full set, lowest level included).** The header exports
  exactly one: `void driver(char c);`. `nm -D --defined-only` on the C `.so`
  confirms `driver` is the *only* defined symbol — there is no lower-level
  entry point beneath it and no convenience wrapper above it, so "exercise the
  low-level API directly" reduces to calling `driver` itself.
* **Runtime options / modes / flags.** None are settable through the API: the
  function takes no flags and the only state it touches is the locale, which it
  pins itself with `setlocale(LC_ALL, "C")` on every call. The *ambient* locale
  is therefore an axis a real consumer can vary (via the environment) even
  though the API cannot — rows 12–14 cover it, because `setlocale` overriding it
  is exactly the behaviour that must be reproduced.
* **`#ifdef` branches.** None in `driver.c` (only the header's include guard).
* **Input shapes the code special-cases.** `driver` has a single scalar `char`
  parameter, so the shape axis is the *value class* of that byte. The branching
  is not in `driver` (which is straight-line) but in the 14 libc calls it
  makes; the classes below are the equivalence classes those 12 classifier bits
  and 2 conversion tables actually distinguish: sign of the `char`, and
  cntrl / blank / space / digit / xdigit-letter / upper / lower / punct /
  print / graph membership.
* **Call sequencing.** `setlocale` on every call makes repeat/interleaved
  invocation a distinct shape (row 15).

Each row is compared by capturing the 14 lines both `.so`s write to `stdout`
and asserting byte equality. Rows spanning more than one value use **many
randomized inputs** drawn from a fixed-seed PRNG (seed `0x5EED_C0DE_1234_5678`,
SplitMix64), plus the exhaustive sweep in row 1.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | exhaustive sweep: every one of the 256 distinct `char` bit patterns `0x00..0xFF`, in ascending order | [x] |
| 2 | `driver` | positive ASCII control chars `0x01..0x08`, `0x0E..0x1F` (cntrl bit only), randomized order | [x] |
| 3 | `driver` | whitespace family `0x09..0x0D` + `0x20` (space/blank interaction), randomized order | [x] |
| 4 | `driver` | decimal digits `'0'..'9'` — digit + xdigit + alnum + print + graph, randomized | [x] |
| 5 | `driver` | hex letters `'a'..'f'` and `'A'..'F'` — xdigit + alpha + case bits, randomized | [x] |
| 6 | `driver` | non-hex lowercase `'g'..'z'` — alpha + lower, no xdigit, randomized | [x] |
| 7 | `driver` | non-hex uppercase `'G'..'Z'` — alpha + upper, no xdigit, randomized | [x] |
| 8 | `driver` | punctuation/symbols (all printable non-alnum: `!"#$%&'()*+,-./:;<=>?@[\]^_`{|}~`), randomized | [x] |
| 9 | `driver` | negative `char` values `0x80..0xFF` (high-bit set ⇒ negative index into the ctype tables), randomized | [x] |
| 10 | `driver` | uniformly random `char` over the full `-128..=127` range, 4096 draws (value-dependent / table-index fuzz) | [x] |
| 11 | `driver` | promoted out-of-range `int` arguments (`256`, `-129`, `1000`, `0x1234`, `i32::MAX`, `i32::MIN`, plus randomized `i32`) passed through the `char` parameter | [x] |
| 12 | `driver` | ambient locale `LC_ALL` unset / `C` (baseline), full 256-value sweep | [x] |
| 13 | `driver` | ambient locale `LC_ALL=en_US.UTF-8` in the environment before load — `setlocale(LC_ALL,"C")` inside `driver` must override it identically in both libs, full 256-value sweep | [x] |
| 14 | `driver` | ambient locale forced to a multibyte locale via a `setlocale(LC_ALL,"")`-style pre-call from the test, then `driver` over the full 256-value sweep | [x] |
| 15 | `driver` | sequencing: 256-value sweep with C and Rust calls interleaved in the *same* process, then each library called repeatedly on the same input (idempotence / no state leak) | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)`
— there is no `add_executable`, and the Rust crate is `crate-type =
["cdylib"]` with no `[[bin]]`. Neither side builds a driver binary of its own,
so `scripts/compare_binaries.sh` supplies one: `tests/harness/driver_main.c` is
compiled by gcc against the unmodified `c_src/include/driver.h` and linked
against each `.so` in turn (`-l:libdriver.so -Wl,-rpath,...`). Its stdout is
diffed with `cmp` over all 256 `char` values plus five values the C compiler
must narrow itself. Result: byte-identical.

This also covers the case libloading cannot: a *real gcc call site* using the
declared `void driver(char)` prototype, which is what proves the Rust export
remains ABI-compatible after the row-11 fix.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one. `scripts/check_features.sh` derives the
combination list from `Cargo.toml` mechanically (powerset of the declared
features, plus the default build) and runs `cargo check`, `cargo build
--release`, `scripts/symbol_diff.sh`, the differential suite and
`scripts/compare_binaries.sh` under each; it reports
`FEATURE SWEEP OK across 1 combination(s)`. Adding a feature later widens the
sweep automatically instead of silently going untested.

## How to reproduce

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation && cargo build --release
./scripts/symbol_diff.sh          # Phase A / D symbol parity
cargo test --test differential    # Phase B + C, all 25 rows
./scripts/compare_binaries.sh     # binary stdout comparison
./scripts/check_features.sh       # everything, under every feature combo
```
