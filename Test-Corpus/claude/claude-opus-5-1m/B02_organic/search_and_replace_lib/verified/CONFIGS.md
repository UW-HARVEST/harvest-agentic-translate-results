# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

## §0 Build configurations (feature combinations)

`Cargo.toml` has **no `[features]` section** ⇒ the set of valid feature
combinations is exactly one: the empty set. `c_src/CMakeLists.txt` defines no
`option()` / compile definitions and `lib.c` contains no `#if*`, so the C side
likewise has one configuration.

| # | build config | command | status |
|---|--------------|---------|--------|
| 0a | default (no features) | `cargo check` / `cargo test` | [x] |
| 0b | `--no-default-features` (identical: empty set) | `cargo check --no-default-features` / `cargo test --no-default-features` | [x] |

## §1 Public entry points

`c_src/include/lib.h` declares exactly one symbol, which is also the lowest
level entry point (there are no convenience wrappers, no init/teardown, no
opaque handles, no global state):

```c
char *searchAndReplace(const char *orig, const char *search, const char *value);
```

There are no runtime options, modes, flags, byte-order or element-type
parameters. The entire configuration surface is therefore the **shape of the
three input strings**, i.e. the conditions the C actually branches on:

| branch | source line | condition |
|--------|-------------|-----------|
| B1 | `lib.c:23` | `strstr(orig, search) == NULL` → `strdup` early return |
| B2 | `lib.c:32` | `inx_start > 0` → allocate + copy the prefix before the first match |
| B3 | `lib.c:42` | loop repeats while another occurrence is found (1 vs ≥2 iterations) |
| B4 | `lib.c:59` | `inx_start2 > from` → copy the gap between two matches (gap > 0 vs adjacent) |
| B5 | `lib.c:78` | `(from < orig_len) && from > 0` → copy the tail after the last match |
| B6 | `lib.c:44/50` | `value_len == 0` (pure deletion) vs `> 0`; `value_len` vs `search_len` (grow / shrink / same) |
| B7 | `lib.c:54` | re-scan starts at `orig + inx_start + search_len` → overlapping occurrences are consumed non-overlapping |

## §2 Rows (cross-product of the axes above, pruned to what the C distinguishes)

Every row is run against **many randomized inputs with a fixed seed**
(deterministic xorshift PRNG in `tests/harness/mod.rs`), calling the C `.so` and
the Rust `.so` through `libloading` and comparing NULL-ness, `strlen`, and every
returned byte.

| #  | entry point | configuration (input shape + branches hit) | [x] |
|----|-------------|--------------------------------------------|-----|
| 1  | `searchAndReplace` | no match, non-empty random `orig`/`search`/`value` → B1 (strdup path) | [x] |
| 2  | `searchAndReplace` | no match, `orig == ""` (empty haystack), non-empty `search` → B1 with 1-byte result | [x] |
| 3  | `searchAndReplace` | no match because `search` is LONGER than `orig` (incl. `search` = `orig`+1 byte) → B1 | [x] |
| 4  | `searchAndReplace` | exactly 1 match at offset 0, tail present, `value` non-empty → !B2, 1×B3, !B4, B5 | [x] |
| 5  | `searchAndReplace` | exactly 1 match and `search == orig` (no prefix, no tail) → !B2, !B4, !B5 (`from == orig_len`) | [x] |
| 6  | `searchAndReplace` | exactly 1 match at offset > 0 with tail → B2, B5 | [x] |
| 7  | `searchAndReplace` | exactly 1 match at the very end (prefix, no tail) → B2, !B5 | [x] |
| 8  | `searchAndReplace` | `value == ""` (deletion), 1 match at offset 0 / middle / end → B6 with `value_len == 0` | [x] |
| 9  | `searchAndReplace` | 2 adjacent matches (gap == 0) at offset 0 → 2×B3, !B4 | [x] |
| 10 | `searchAndReplace` | 2 matches separated by a gap > 0 → 2×B3, B4 | [x] |
| 11 | `searchAndReplace` | 2 matches, the second ending exactly at `orig_len` → B4, !B5 | [x] |
| 12 | `searchAndReplace` | ≥5 matches at random offsets with random gaps (some adjacent, some not), random prefix/tail presence | [x] |
| 13 | `searchAndReplace` | `orig` = `search` repeated k times (k = 1..8): all-adjacent, no prefix, no tail | [x] |
| 14 | `searchAndReplace` | overlapping occurrences, even count: `orig = "a"*2n`, `search = "aa"` → B7 | [x] |
| 15 | `searchAndReplace` | overlapping occurrences, odd remainder: `orig = "a"*(2n+1)`, `search = "aa"` → B7 + B5 tail of 1 | [x] |
| 16 | `searchAndReplace` | `value_len > search_len` (buffer grows), many matches | [x] |
| 17 | `searchAndReplace` | `value_len < search_len` (buffer shrinks relative to input), many matches | [x] |
| 18 | `searchAndReplace` | `value_len == search_len` (in-place-size), many matches | [x] |
| 19 | `searchAndReplace` | `value` CONTAINS `search` (output must NOT be re-scanned), 1 and many matches | [x] |
| 20 | `searchAndReplace` | `search` = 1 byte; `orig` random over a 2-symbol alphabet (dense matches, every branch combination reached by chance) | [x] |
| 21 | `searchAndReplace` | `search` = 2..8 bytes; `orig` random over a 2–4 symbol alphabet (dense/overlapping multi-byte matches) | [x] |
| 22 | `searchAndReplace` | high / non-UTF8 bytes (0x80..0xFF) in `orig`, `search`, `value`; also 0x01..0x7F full range | [x] |
| 23 | `searchAndReplace` | long inputs: `orig` 4 KiB–64 KiB with hundreds of matches, `value` up to 1 KiB (many `realloc` growth steps) | [x] |
| 24 | `searchAndReplace` | 1-byte `orig` with 1-byte `search` (match and no-match), `value` empty and non-empty | [x] |
| 25 | `searchAndReplace` | `orig == search == value` (identity replacement), and `orig == search`, `value == ""` (empty result) | [x] |
| 26 | `searchAndReplace` | matches at BOTH ends plus the middle (no prefix, no tail, mixed gaps) | [x] |
| 27 | `searchAndReplace` | full randomized sweep: 20 000 cases, random lengths 0–64, random alphabets 1–6 symbols, random `search`/`value` lengths 0–8 (the empty-`search` case is excluded — it does not terminate, see `ERRORS.md` rows 10/11) | [x] |
