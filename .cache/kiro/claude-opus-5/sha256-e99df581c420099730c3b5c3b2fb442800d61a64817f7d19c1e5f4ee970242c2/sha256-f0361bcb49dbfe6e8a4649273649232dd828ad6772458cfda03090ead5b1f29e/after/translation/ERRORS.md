# ERRORS.md — error / rejection surface table

## How this was derived

Mechanical grep over the *library* sources only (`c_src/include/lib.h`,
`c_src/src/lib.c`; `c_src/build/CMakeFiles/.../CMakeCCompilerId.c` is CMake's
compiler-probe file and is not part of the library):

```
grep -nE 'return|assert|NULL|errno|enum|#if|#ifdef|-1|malloc|free|size|len' c_src/src/lib.c c_src/include/lib.h
```

Findings:

* `return` statements: **2** (one per function) — both are a single boolean
  expression, not an error/success branch.
* `assert`: **0**. `NULL` checks: **0**. `errno`: **0**. error enums / error
  macros (`RETURN_ERROR`, …): **0**. `return -1` / `return NULL`: **0**.
* allocation / length / count parameters: **0** — the API takes two raw
  `const uint8_t *` and no length.
* `#if` / `#ifdef` / conditional compilation: **0**.
* Min/max constants: the only constants are the bit masks and shift/compare
  literals `0xff, 0xF0, 0xf0, 0xFE, 0xe2, 0x0C, 15, 3, 2, 1, 0` (line-tagged
  above), all used as bit tests.

So this library has **no error channel**: `hdr_compare` returns `int` `1` or `0`
only. "Rejection" therefore means *returning 0*, and the exhaustive set of
distinct rejection branches is the set of `&&` / `||` sub-conditions that can
make the expression false. Each row below is one such branch, taken straight
from the source text — 5 from `hdr_valid` (reached through `hdr_compare(h1,h2)`
on `h2`) and 3 from `hdr_compare` itself.

`hdr_valid` is `static` and unexported, so its rejections are only observable
via `hdr_compare`'s second argument.

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `hdr_compare` → `hdr_valid(h2)` | `h2[0] != 0xff` (any of the 255 other first bytes; sync byte missing) | `0` | [x] |
| 2 | `hdr_compare` → `hdr_valid(h2)` | `(h2[1] & 0xF0) != 0xf0` **and** `(h2[1] & 0xFE) != 0xe2` (second byte in neither accepted class: not high-nibble `0xF`, not `0xE2`/`0xE3`) | `0` | [x] |
| 3 | `hdr_compare` → `hdr_valid(h2)` | `((h2[1] >> 1) & 3) == 0` (layer field zero, e.g. `h2[1] == 0xf0` or `0xf1`) | `0` | [x] |
| 4 | `hdr_compare` → `hdr_valid(h2)` | `(h2[2] >> 4) == 15` (high nibble of third byte all-ones, `h2[2] >= 0xf0`) | `0` | [x] |
| 5 | `hdr_compare` → `hdr_valid(h2)` | `((h2[2] >> 2) & 3) == 3` (bits 2–3 of third byte both set, `h2[2] & 0x0C == 0x0C`) | `0` | [x] |
| 6 | `hdr_compare` | `h2` valid but `((h1[1] ^ h2[1]) & 0xFE) != 0` (second bytes differ in any of bits 1–7) | `0` | [x] |
| 7 | `hdr_compare` | `h2` valid, byte-1 masks agree, but `((h1[2] ^ h2[2]) & 0x0C) != 0` (third bytes differ in bit 2 and/or bit 3) | `0` | [x] |
| 8 | `hdr_compare` | `h2` valid, rows 6–7 pass, but `((h1[2] & 0xF0) == 0) ^ ((h2[2] & 0xF0) == 0)` — exactly one of the two third-byte high nibbles is zero | `0` | [x] |

## Generic FFI boundary conditions (not in the table above, tested anyway)

| # | condition | expected C behaviour | how tested | [x] |
|---|-----------|----------------------|-----------|-----|
| G1 | `h1 == NULL` while `h2` is **invalid** | `0`, and `h1` is never dereferenced (`&&` short-circuits) — no fault | differential, both `.so`s called with a real null pointer | [x] |
| G2 | `h1 == NULL` while `h2` is **valid** | undefined behaviour in C (unconditional `h1[1]` load) — **not testable**, documented, deliberately not exercised | n/a (documented) | [x] |
| G3 | `h2 == NULL` | undefined behaviour in C (`hdr_valid` loads `h2[0]` unconditionally) — **not testable**, documented, deliberately not exercised | n/a (documented) | [x] |
| G4 | read-extent / "oversized length": does the C read past byte 2? | never reads `h1[0]`, never reads any index `> 2`; reads `h2[2]`/`h1[1]`/`h1[2]` only when the preceding `&&` terms hold | `mmap` + `PROT_NONE` guard page placed immediately after the header so any over-read faults; asserted for both `.so`s | [x] |
| G5 | out-of-range enum value across the FFI boundary | **N/A** — the API has no `enum`, no flag, and no integer-mode parameter (grep for `enum` returns 0 hits). Every one of the 2^8 values of every byte parameter is in range and meaningful; there is no "one past the valid range" scalar. Full coverage instead comes from the exhaustive byte sweeps in Phase B. | exhaustive 2^24 `h2` sweep + exhaustive 2^16 `h1` sweep | [x] |
| G6 | return-value domain | C returns only `0` or `1` (`&&` yields `int` 0/1); Rust must not return e.g. `-1` or `2` | asserted on every differential call | [x] |
| G7 | non-NULL but unaligned / arbitrary pointer | `uint8_t` loads have no alignment requirement; every odd/even offset must behave identically | headers placed at every offset 0..7 inside a buffer | [x] |

## Row → test mapping

| rows | test |
|------|------|
| 1 | `phase_c_errors::err01_bad_sync_byte` (all 255 wrong sync bytes × 64 vectors) |
| 2 | `phase_c_errors::err02_byte1_in_neither_class` (all 238 rejected byte-1 values) |
| 3 | `phase_c_errors::err03_layer_zero` (all 4 layer-0 values) |
| 4 | `phase_c_errors::err04_nibble_all_ones` (all 16 values `0xF0..0xFF`) |
| 5 | `phase_c_errors::err05_sbits_three` (all 60 values with `S==3`, nibble ≠ 15) |
| 6 | `phase_c_errors::err06_byte1_mask_fe_mismatch_exhaustive_masks` (all 254 XOR masks touching `0xFE`) |
| 7 | `phase_c_errors::err07_byte2_mask_0c_mismatch_exhaustive` (exhaustive `(h2[2], h1[2])` pairs) |
| 8 | `phase_c_errors::err08_high_nibble_zeroness_xor_exhaustive` (exhaustive, isolated from rows 6–7) |
| G1 | `phase_c_errors::g01_null_h1_with_invalid_h2` |
| G2, G3 | not testable (C dereferences unconditionally → UB); documented above |
| G4 | `phase_c_errors::g04_no_read_past_byte_two` (`mmap` + `PROT_NONE` guard page) |
| G5 | `phase_c_errors::g05_no_enum_or_flag_parameter_in_the_api` (asserts the header still declares no enum/flag, so the row cannot rot) + exhaustive byte coverage in `phase_b_sweeps` |
| G6 | asserted on **every** differential call by `common::Pair::assert_same`, plus `g06_g07_return_domain_and_unaligned` |
| G7 | `phase_c_errors::g06_g07_return_domain_and_unaligned` (odd offsets) and `phase_b_configs::row27_pointer_placement_and_alignment` |

All 12 tests in `tests/phase_c_errors.rs` pass against both `.so`s, under both
feature combinations and both build profiles.
