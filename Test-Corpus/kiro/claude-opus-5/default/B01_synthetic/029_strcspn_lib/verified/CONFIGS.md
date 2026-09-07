# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the C source and the public header.

## Axes the C code actually distinguishes

**Runtime options / modes / flags:** *none.* `c_src/include/driver.h` exposes a
single entry point with no flag, mode, enum or context parameter:

```c
void driver(const char *s1, const char *s2);
```

`grep` over `c_src/` finds no `if`, `switch`, `#ifdef` (other than the header's
include guard), no global/static state, and no setter — so there is no
configuration to toggle. `translation/Cargo.toml` likewise defines no
`[features]`, so there is no compile-time axis either.

**Public entry points:** `driver` is both the highest- and the lowest-level entry
point; there is no convenience wrapper hiding a lower-level API. The one
*internal* routine the C composes is libc `strcspn`, which the Rust reimplements
by hand — that reimplementation is the real subject of these tests, and it is
driven through the exported `driver` symbol exactly as an external caller would.

**Input shapes the code special-cases:** the branching all lives inside
`strcspn`'s scan, whose behaviour is a function of (length of `s1`, length of
`s2`, position of the first byte of `s1` that is a member of `s2`, and whether
such a byte exists at all), plus `printf("%zu\n", ...)`'s rendering of the
resulting `size_t`. glibc's `strcspn` additionally switches between a
bitmap/SIMD path and a scalar path on the size of the reject set, so reject-set
size is a real axis even though the visible C has no branch on it.

Axes, enumerated:

* `len(s1)`: 0 · 1 · small (2–8) · medium (9–64) · large (crosses vector-width
  and page boundaries) · huge (1 MiB)
* `len(s2)`: 0 · 1 · small · medium · 255 (every non-NUL byte) · huge
* match position in `s1`: none · first byte · interior · last byte
* byte alphabet: ASCII printable · full `0x01`–`0xFF` (exercises the signed-`char`
  comparison) · single repeated byte
* `s2` contains duplicates / is a superset of `s1`'s alphabet / is disjoint from it
* pointer placement: `s1` and `s2` aliasing the same buffer; `s1` at a
  non-8-byte-aligned offset (glibc's `strcspn` has alignment-dependent paths)

## Rows (cross-product, pruned to combinations the code treats differently)

Every row is exercised with **many randomized inputs** (`SEED = 0x5EED_1234`,
deterministic xorshift PRNG, ≥256 cases per row unless noted), and both `.so`s
are called through `libloading` with their stdout captured and compared
byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `driver` | `len(s1)==0`, `s2` random non-empty ASCII | `cfg_01_empty_s1` | [x] |
| 2 | `driver` | `s1` random non-empty, `len(s2)==0` → result is `strlen(s1)` | `cfg_02_empty_s2` | [x] |
| 3 | `driver` | both empty | `cfg_03_both_empty` | [x] |
| 4 | `driver` | `len(s1)==1`, `len(s2)==1`, match and no-match both generated | `cfg_04_single_single` | [x] |
| 5 | `driver` | `len(s1)` small (2–8), `s2` disjoint from `s1` → no match, result `len(s1)` | `cfg_05_small_disjoint` | [x] |
| 6 | `driver` | `len(s1)` small, first byte of `s1` in `s2` → result `0` | `cfg_06_match_at_first` | [x] |
| 7 | `driver` | `len(s1)` small, only the LAST byte of `s1` in `s2` → result `len(s1)-1` | `cfg_07_match_at_last` | [x] |
| 8 | `driver` | `len(s1)` small, match at a random interior index | `cfg_08_match_interior` | [x] |
| 9 | `driver` | `len(s1)` medium (9–64), `s2` random small, unconstrained overlap | `cfg_09_medium_random` | [x] |
| 10 | `driver` | `len(s1)` medium, `s2` medium (9–64), unconstrained overlap | `cfg_10_medium_medium` | [x] |
| 11 | `driver` | `len(s1)` large (65–4096, crosses vector widths), `s2` small | `cfg_11_large_s1_small_s2` | [x] |
| 12 | `driver` | `len(s1)` large, `s2` = all 255 non-NUL bytes → always matches at 0 | `cfg_12_s2_all_bytes` | [x] |
| 13 | `driver` | full `0x01..=0xFF` alphabet in BOTH strings (signed-`char` comparison) | `cfg_13_high_bit_alphabet` | [x] |
| 14 | `driver` | `s1` a single repeated byte, `s2` a single byte (equal / not equal) | `cfg_14_repeated_byte` | [x] |
| 15 | `driver` | `s2` full of duplicate bytes (same byte repeated N times) | `cfg_15_s2_duplicates` | [x] |
| 16 | `driver` | `s2` a strict superset of `s1`'s alphabet → result `0` for non-empty `s1` | `cfg_16_s2_superset` | [x] |
| 17 | `driver` | `s1 == s2` (same pointer, aliased buffer) → result `0` for non-empty | `cfg_17_aliased_same_ptr` | [x] |
| 18 | `driver` | `s1` at every misaligned offset 0..16 within an over-allocated buffer | `cfg_18_misaligned_s1` | [x] |
| 19 | `driver` | `s2` at every misaligned offset 0..16 | `cfg_19_misaligned_s2` | [x] |
| 20 | `driver` | huge `len(s1)` (1 MiB) with no match — `%zu` renders a 7-digit value | `cfg_20_huge_s1_no_match` | [x] |
| 21 | `driver` | huge `len(s1)` (1 MiB) with the match at a random far offset | `cfg_21_huge_s1_far_match` | [x] |
| 22 | `driver` | huge `len(s2)` (64 KiB) built from random bytes, `s1` small | `cfg_22_huge_s2` | [x] |
| 23 | `driver` | lengths exactly on power-of-two / vector boundaries (15,16,17,31,32,33,63,64,65,127,128,129), no match | `cfg_23_boundary_lengths` | [x] |
| 24 | `driver` | fully unconstrained fuzz: random lengths 0–512, random bytes `0x01..=0xFF`, both strings (2048 cases) | `cfg_24_unconstrained_fuzz` | [x] |
| 25 | `driver` | repeated calls in sequence on one loaded pair of `.so`s (statelessness / stdio buffering across calls) | `cfg_25_repeated_calls_stateless` | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)`
and `translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]`.
Neither side builds an executable driver, so the "compare C and Rust binary
stdout" clause of Phase B has no subject. The equivalent coverage — comparing the
bytes the two implementations write to stdout — is what every row above asserts.
