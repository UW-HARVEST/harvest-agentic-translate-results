# CONFIGS.md — Phase A configuration-surface table

## Axis derivation

`driver` has no options, flags, modes, or `#ifdef`s. Grepping the C source for
branch points:

```
$ grep -cE 'if|switch|#if|for|while|\?' c_src/src/driver.c
0
```

The C function body is straight-line code. All behavioural variation therefore
comes from the two libc primitives it composes, so the configuration axes are
the input *shapes* those primitives branch on:

* **A1 — `strcspn` reject-set size** (`strlen(s2)`): `0` (empty reject set →
  `strcspn` degenerates to `strlen`), `1`, `2..16`, `>16`, all 255 non-NUL bytes.
  glibc dispatches `strcspn` to a `strlen` fast path when `s2[0] == '\0'`, and
  to a 16-byte `pcmpistri` / AVX2 path otherwise, so this axis selects real
  distinct machine code.
* **A2 — match position in `s1`**: no match at all (scan to NUL), match at
  index 0 (immediate return), match in the first vector chunk, match beyond the
  first chunk, match at the very last byte.
* **A3 — `strlen(s1)`**: 0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, ~4 KiB
  (page), 1 MiB. Straddles every SIMD block size and the page boundary that
  glibc's over-reading loads are careful about.
* **A4 — byte-value domain**: ASCII-only, bytes `0x80..=0xFF` (high bit set —
  distinguishes a signed-`char` from an unsigned-`char` comparison), full
  `0x01..=0xFF` mix.
* **A5 — pointer alignment**: `s1` and `s2` each offset `0..16` bytes into
  their allocation, independently. Selects between glibc's aligned and
  unaligned entry paths.
* **A6 — buffer relationship**: disjoint buffers, `s1 == s2` (aliased), `s1`
  pointing into the interior of `s2`'s buffer (overlapping).
* **A7 — result magnitude → `printf("%zu")` digit width**: 1..7 digits, since
  `%zu` formatting of a `size_t` is the other half of the observable output.

Duplicate bytes inside `s2` and a `s2` that is a permutation of itself are
folded into A1/A4 (they change no branch, only the table contents).

## Entry points

The library exposes exactly **one** public entry point, `driver`. There is no
convenience wrapper / low-level split to distinguish: `driver` *is* the
lowest-level entry point, and it is the one every row below calls, through the
`.so` export via `libloading`.

## Configuration table

Each row is exercised with **many randomized inputs** (fixed seed `0x5EED_1234`,
a `SplitMix64` generator in the test file, ≥64 cases per row unless noted) and
its stdout compared byte-for-byte between the C and Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | A1=0 (`s2` empty) × A3=0 (`s1` empty) — degenerate `strlen("")` | [x] |
| 2 | `driver` | A1=0 (`s2` empty) × A3∈{1,15,16,17,31,32,33,63,64,65} × A4=ASCII — `strcspn` ≡ `strlen`, all SIMD block sizes | [x] |
| 3 | `driver` | A1=0 (`s2` empty) × A3=4096 (page-sized `s1`) | [x] |
| 4 | `driver` | A1=0 (`s2` empty) × A3=1 MiB (oversized `s1`) | [x] |
| 5 | `driver` | A1=1 (single reject byte) × A2=no-match × A3∈{1..65} random × A4=ASCII | [x] |
| 6 | `driver` | A1=1 × A2=match-at-index-0 (immediate return, result `0`) × A4=ASCII | [x] |
| 7 | `driver` | A1=1 × A2=match-inside-first-16-byte-chunk × A3≥17 | [x] |
| 8 | `driver` | A1=1 × A2=match-beyond-first-chunk (index ≥16) × A3≥33 | [x] |
| 9 | `driver` | A1=1 × A2=match-at-last-byte × A3∈{1..65} random | [x] |
| 10 | `driver` | A1∈2..16 (small reject set) × A2=random × A3∈0..80 random × A4=ASCII | [x] |
| 11 | `driver` | A1>16 (reject set larger than one vector) × A2=random × A3∈0..80 random | [x] |
| 12 | `driver` | A1=255 (`s2` = every non-NUL byte) × A3≥1 — result always `0` | [x] |
| 13 | `driver` | A1=255 (`s2` = every non-NUL byte) × A3=0 — result `0`, `s1` never scanned | [x] |
| 14 | `driver` | A4=high bytes `0x80..=0xFF` in **both** `s1` and `s2` × A2=match present — signed-vs-unsigned `char` comparison | [x] |
| 15 | `driver` | A4=high bytes in `s1` only, ASCII `s2` (no match) — result is full `strlen` | [x] |
| 16 | `driver` | A4=full `0x01..=0xFF` random mix in both × A1,A3 random — broad property sweep, 512 cases | [x] |
| 17 | `driver` | A5=`s1` offset `0..16` × `s2` offset `0..16` (full 17×17 alignment cross-product) × A2=random | [x] |
| 18 | `driver` | A6=aliased (`s1 == s2`), non-empty — first byte of `s1` is in `s2`, result `0` | [x] |
| 19 | `driver` | A6=aliased (`s1 == s2`), both empty — result `0` | [x] |
| 20 | `driver` | A6=overlapping: `s1` = interior suffix pointer into `s2`'s buffer | [x] |
| 21 | `driver` | A3 tuned so the result spans A7=1..7 digit widths (0,1,9,10,99,100,999,1000,65535,65536,1048576) | [x] |
| 22 | `driver` | A3 sized so `s1`'s NUL lands exactly on the final byte of a page, no match — glibc's page-crossing guard | [x] |
| 23 | `driver` | A3 sized so the *match* byte lands exactly on a page boundary | [x] |
| 24 | `driver` | `s2` contains repeated/duplicate bytes only (e.g. `"aaaaaaaa"`) × A2=random | [x] |
| 25 | `driver` | `s1` is a single repeated byte, `s2` = that byte — result `0` | [x] |
| 26 | `driver` | `s1` is a single repeated byte, `s2` = a different single byte — result `strlen(s1)` | [x] |
| 27 | `driver` | repeated successive invocations in one process (stdio buffering / no hidden state carried between calls) | [x] |
| 28 | `driver` | unrestricted property sweep: random A1×A2×A3×A4×A5 for 2000 cases, fixed seed | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)`
— there is **no `add_executable`**, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. The project builds **no driver
binary**, so the "compare C and Rust stdout of the binaries" gate is vacuous.
Stdout *is* nevertheless compared byte-for-byte for every row above, because
`driver`'s only observable effect is what it `printf`s.
