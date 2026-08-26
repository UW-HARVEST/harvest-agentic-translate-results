# CONFIGS.md — Configuration-surface table (valid inputs)

Derived **mechanically** from the C source, the same way `ERRORS.md` is.

## Mechanical derivation of the axes

```sh
grep -nE '#if|#ifdef|#ifndef|#define' c_src/src/driver.c c_src/include/driver.h
#   -> only the DRIVER_H_ include guard: ZERO build-time configuration
grep -nE 'if *\(|switch|while|for|\?|else|goto' c_src/src/driver.c
#   -> NO MATCHES: zero runtime branches in the library's own code
grep -nE '^[a-zA-Z_].*\(' c_src/src/driver.c c_src/include/driver.h
#   -> exactly one entry point: void driver(const char *s1, const char *s2)
grep -n '\[features\]' Cargo.toml
#   -> NO MATCH: the Rust crate declares no cargo features
```

### Consequences

* **Build-time configurations: exactly one.** `CMakeLists.txt` has no options and
  compiles a single file; `Cargo.toml` has no `[features]` table. The complete
  set of feature combinations is therefore the single empty combination, checked
  and tested as `--no-default-features`.
* **Runtime options/modes/flags: none.** `driver` takes no flag, mode, enum,
  length, byte-order or format parameter. There is no state to set up, nothing to
  initialise or destroy, and no option to toggle.
* **Public entry points: one.** `driver` is simultaneously the highest- and the
  *lowest*-level entry point — it is not a convenience wrapper over a finer-grained
  API, so "exercise the low-level entry points too" is satisfied by construction.

The entire behavioural surface is therefore the **input shape** of the two
NUL-terminated byte strings, plus the `%zu` formatting of the result. The C body
delegates to `printf` (which the Rust translation calls *identically*, the same
`printf@GLIBC_2.2.5`) and to `strcspn` (which the Rust translation
**reimplements** — this is the one place a divergence can exist, so the axes
below are chosen to attack it).

`strcspn` in glibc is a hand-optimised SIMD routine with special-cased paths for
small reject sets and vector-width-aligned scanning; the Rust version is a naive
256-entry table plus byte loop. The axes below enumerate exactly what those two
implementations could disagree on.

### Axes the behaviour actually depends on

| axis | values the code distinguishes |
|------|-------------------------------|
| **A** `strlen(s2)` (reject-set size) | 0 (empty → pure `strlen`), 1, 2, 3, 4–16, 17–64, 255 (full non-NUL domain) |
| **B** match position in `s1` | 0 (first byte), 1, interior, last byte, **none** (→ returns `strlen(s1)`) |
| **C** `strlen(s1)` | 0, 1, small, vector boundaries 15/16/17, 31/32/33, 63/64/65, 127/128/129, KiB, 1 MiB |
| **D** pointer alignment | `s1`/`s2` at byte offsets 0..63 within a page; buffer ending exactly at a page boundary |
| **E** byte-value domain | ASCII only; high-bit `0x80..0xFF` (negative `char` → sign-extension hazard); full `0x01..0xFF` |
| **F** reject-set redundancy | all-distinct bytes vs. heavy duplicates vs. one byte repeated |
| **G** result magnitude (`%zu` width) | 0, 1, 2, 3, 4, 5, 6, 7 decimal digits |
| **H** call sequencing | single call; many calls in sequence (per-call state must not leak); C and Rust calls interleaved on the shared `stdout` |
| **I** buffer aliasing | disjoint buffers; `s1 == s2` (same pointer); overlapping buffers |

## Configuration-surface table

Every row is a meaningful combination of the axes above, tested by calling
**both** the C `.so` and the Rust `.so` through `libloading` and comparing the
captured stdout **byte-for-byte**. Every row uses **many randomized inputs**
(fixed seed `0x5EED_C5BD_1234_9ABC`, deterministic SplitMix64 PRNG) unless the
row is inherently a single exhaustive sweep.

| # | entry point(s) | configuration (options set + input shape) | axes | test | [x] |
|---|----------------|--------------------------------------------|------|------|-----|
| 1 | `driver` | `s2` empty + `s1` empty → result `0` | A0,B–,C0,G1 | `cfg01_empty_s1_empty_s2` | [x] |
| 2 | `driver` | `s2` empty + `s1` random len 1..=64, random bytes → pure `strlen` path | A0,C small,E full | `cfg02_empty_s2_random_s1` | [x] |
| 3 | `driver` | `s1` empty + `s2` random non-empty → result `0` | A1..,C0 | `cfg03_empty_s1_random_s2` | [x] |
| 4 | `driver` | `|s2|==1`, match at position 0 | A1,B0 | `cfg04_s2_len1_match_first` | [x] |
| 5 | `driver` | `|s2|==1`, match at last position | A1,B last | `cfg05_s2_len1_match_last` | [x] |
| 6 | `driver` | `|s2|==1`, no match anywhere (returns `strlen(s1)`) | A1,B none | `cfg06_s2_len1_no_match` | [x] |
| 7 | `driver` | `|s2|==1`, match at **every** interior position, `s1` len sweep 1..=80 (exhaustive cross product) | A1,B all,C sweep | `cfg07_s2_len1_all_positions` | [x] |
| 8 | `driver` | `|s2|==2`, randomized match positions incl. none | A2,B all | `cfg08_s2_len2` | [x] |
| 9 | `driver` | `|s2|==3`, randomized match positions incl. none | A3,B all | `cfg09_s2_len3` | [x] |
| 10 | `driver` | `|s2|` in 4..=16, randomized | A 4–16,B all | `cfg10_s2_len_4_16` | [x] |
| 11 | `driver` | `|s2|` in 17..=64, randomized | A 17–64,B all | `cfg11_s2_len_17_64` | [x] |
| 12 | `driver` | `s2` = all 255 non-NUL bytes `0x01..0xFF` (full domain) + random non-empty `s1` → always `0` | A255,E full | `cfg12_s2_full_255_domain` | [x] |
| 13 | `driver` | `s2` = all 255 non-NUL bytes, `s1` empty → `0` | A255,C0 | `cfg13_s2_full_domain_s1_empty` | [x] |
| 14 | `driver` | `|s1|` swept 0..=136 with **no** match → checks every length incl. all vector boundaries | C sweep,B none | `cfg14_s1_length_sweep_no_match` | [x] |
| 15 | `driver` | `|s1|` = 15,16,17,31,32,33,63,64,65,127,128,129,255,256,257 with match at each of the final 3 and first 3 positions | C vector bounds,B edge | `cfg15_vector_boundary_match_positions` | [x] |
| 16 | `driver` | `s1` alignment sweep: offsets 0..=63 within a page, fixed content, match near end | D s1,C | `cfg16_s1_alignment_sweep` | [x] |
| 17 | `driver` | `s2` alignment sweep: offsets 0..=63 within a page | D s2,A | `cfg17_s2_alignment_sweep` | [x] |
| 18 | `driver` | `s1` NUL-terminated so its final byte sits **exactly** on the last byte of a mapped page (next page unmapped), no match → maximal legal over-read pressure | D page edge,B none | `cfg18_s1_terminated_at_page_edge` | [x] |
| 19 | `driver` | `s2` NUL-terminated exactly at a mapped-page edge | D page edge,A | `cfg19_s2_terminated_at_page_edge` | [x] |
| 20 | `driver` | `s1` all high-bit bytes `0x80..0xFF`, `s2` ASCII (no match) → sign-extension hazard, returns `strlen` | E high,B none | `cfg20_s1_high_bit_no_match` | [x] |
| 21 | `driver` | `s1` and `s2` both from `0x80..0xFF` with a real match → sign-extension on the reject-table index | E high,B all | `cfg21_high_bit_match` | [x] |
| 22 | `driver` | `s2` contains `0xFF` specifically; `s1` contains `0xFF` at a random position → top-of-domain off-by-one | E 0xFF | `cfg22_byte_0xff_boundary` | [x] |
| 23 | `driver` | `s2` heavy duplicates (same byte repeated 1..=64 times) | F dup,A | `cfg23_s2_duplicate_bytes` | [x] |
| 24 | `driver` | `s2` = one byte repeated 200 times, `s1` random | F repeat | `cfg24_s2_single_byte_repeated` | [x] |
| 25 | `driver` | `s1` 4 KiB, no match → 4-digit `%zu` | C KiB,G4 | `cfg25_s1_4kib_no_match` | [x] |
| 26 | `driver` | `s1` 64 KiB, no match → 5-digit `%zu` | C KiB,G5 | `cfg26_s1_64kib_no_match` | [x] |
| 27 | `driver` | `s1` 1 MiB, no match → 7-digit `%zu` (widest result) | C MiB,G7 | `cfg27_s1_1mib_no_match` | [x] |
| 28 | `driver` | result-magnitude sweep: exact lengths 0,1,9,10,11,99,100,101,999,1000,1001,9999,10000,10001,99999,100000 → every `%zu` digit-count transition | G all | `cfg28_result_digit_width_sweep` | [x] |
| 29 | `driver` | many sequential calls (200) with **alternating** reject sets (full domain ↔ empty) → per-call reject-table state must not leak between calls | H sequence,A0/A255 | `cfg29_no_state_leak_between_calls` | [x] |
| 30 | `driver` | C and Rust calls **interleaved** on the shared `stdout` FILE, 50 rounds → identical stream/buffering behaviour | H interleave | `cfg30_interleaved_c_and_rust_calls` | [x] |
| 31 | `driver` | `s1 == s2` (identical pointer) → every byte of `s1` is in the reject set, so result is `0` for non-empty and `0` for empty | I alias | `cfg31_s1_equals_s2_same_pointer` | [x] |
| 32 | `driver` | overlapping buffers: `s2` points into the interior of `s1`'s buffer | I overlap | `cfg32_overlapping_buffers` | [x] |
| 33 | `driver` | embedded NUL mid-buffer in `s1` **and** `s2`; bytes after the NUL differ between the two libraries' view → must be ignored | E,B | `cfg33_embedded_nul_mid_buffer` | [x] |
| 34 | `driver` | large randomized property sweep: 4000 cases, `|s1|` 0..=300, `|s2|` 0..=300, bytes over the full `0x01..0xFF` domain, random alignments | A,B,C,D,E,F cross | `cfg34_randomized_property_sweep` | [x] |
| 35 | `driver` | biased randomized sweep: `s2` drawn from a tiny alphabet so matches are dense and occur early; `|s1|` 0..=500 | A small,B early,C | `cfg35_dense_match_sweep` | [x] |
| 36 | `driver` | biased randomized sweep: `s2` drawn from a large alphabet disjoint from `s1`'s alphabet so matches are rare/absent; `|s1|` 0..=500 | A large,B none | `cfg36_sparse_match_sweep` | [x] |
| 37 | `driver` | result exactly `2^31` (`s1` = 2 GiB of `'a'`, no match) — the **only** shape that can distinguish `printf("%zu")` from `printf("%d")` on x86-64, since below 2^31 the low 32 bits print identically | G 10-digit / 64-bit | `cfg37_result_crosses_2gib_zu_vs_int_boundary` | [x] |

Row 37 needs ~2.8 GiB of free RAM; it reads `MemAvailable` from `/proc/meminfo`
and prints a `SKIPPED` notice instead of failing on a smaller machine. On this
host it ran and passed.

## Are these rows actually load-bearing? (mutation testing)

A row that passes proves nothing unless it would have failed on a wrong
implementation. Nine defects were seeded into a *copy* of `src/lib.rs`, each built
as its own `cdylib` and run through the suite. **All nine were caught:**

| seeded defect | caught by |
|---------------|-----------|
| `n += 1` → `n += 2` (off-by-one count) | 4 tests |
| reject index sign-extended (`b as i8 as i32 & 0x7f`) | 2 tests |
| last byte of `s2` dropped from the reject set | 2 tests |
| reject table sized `[false; 255]` instead of `256` | 3 tests |
| `%zu` → `%d` | 1 test (row 37 — nothing else can see it) |
| trailing `\n` removed from the format string | 6 tests |
| empty `s2` short-circuited to `0` instead of `strlen(s1)` | 3 tests |
| `s1` scan no longer stops at the NUL | 5 tests |
| reject-set build stops after the first byte | 3 tests |

The `%zu` → `%d` mutant initially escaped the suite; row 37 was added
specifically to close that blind spot, and it now catches it.

## Completion gate

- [x] All 37 rows pass across their randomized inputs (fixed seed) under the
      single valid feature combination (`--no-default-features`), in both the
      `release` and `debug` profiles.
- [x] Both the C and the Rust implementations are invoked **only** through
      `libloading` on their respective `.so` files — never by direct Rust call —
      so the `#[no_mangle] extern "C"` export wrapper is under test too.
