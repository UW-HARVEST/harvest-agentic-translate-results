# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

There are **no** runtime options, **no** flags, **no** modes, **no** `switch`
statements and **no** `#ifdef`s in `c_src/src/lib.c`, and the public header
declares exactly one entry point. The Cargo manifest declares **no** cargo
features, so there is exactly one feature combination (`--no-default-features`
and the default build are the same code). The entire configuration surface is
therefore the set of **input shapes** the code special-cases:

| axis | source of the branch | values |
|------|----------------------|--------|
| A1 — any match at all | `if (p == NULL)` @ 23 (the `strdup` fast path) | no match / ≥1 match |
| A2 — leading context | `if (inx_start > 0)` @ 32 (`malloc` vs `realloc(NULL,…)`) | first match at index 0 / at index > 0 |
| A3 — loop trip count | `while (p != NULL)` @ 42 | 1 match / 2 matches / many matches |
| A4 — further occurrence found | `if (p != NULL)` @ 55 | last iteration / non-last iteration |
| A5 — gap between matches | `if (inx_start2 > from)` @ 59 | adjacent matches (gap 0) / separated matches (gap > 0) |
| A6 — trailing context | `if ((from < orig_len) && from > 0)` @ 78 | match ends at end of `orig` / tail present |
| A7 — `value_len` vs `search_len` | drives `total_bytes_allocated` growth @ 44 | shrink / same size / grow / `value_len == 0` |
| A8 — `search` search semantics | `strstr(orig + inx_start + search_len, …)` @ 54 (non-overlapping scan) | single-byte needle / multi-byte needle / self-overlapping needle |
| A9 — `orig_len` | all length arithmetic | 0 / 1 / small / large (many `realloc` round-trips) |
| A10 — byte values | `strncpy`/`strstr` are byte-wise | ASCII / high bytes 0x80–0xFF / bytes that look like UTF-8 continuations |

`search == ""` is deliberately absent: it is the non-terminating case, filed as
row **E9** in `ERRORS.md`.

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is exercised through the `searchAndReplace` export of **both** `.so`s
with **many randomized inputs** (fixed seed, deterministic xorshift PRNG) and
the returned buffers are compared byte-for-byte, including the NUL terminator.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `searchAndReplace` | A1=no match, `orig == ""` (empty haystack), non-empty `search` → `strdup("")` | [x] |
| C2 | `searchAndReplace` | A1=no match, `orig` non-empty, `search` absent from `orig` → `strdup(orig)`; randomized lengths 1..64 | [x] |
| C3 | `searchAndReplace` | A1=no match because `search_len > orig_len` (needle longer than haystack) | [x] |
| C4 | `searchAndReplace` | A2=match at index 0 (`inx_start == 0`, no leading `malloc`), exactly 1 match, A6=no tail (`search == orig`) | [x] |
| C5 | `searchAndReplace` | A2=match at index 0, exactly 1 match, A6=tail present | [x] |
| C6 | `searchAndReplace` | A2=match at index > 0 (leading `malloc` taken), exactly 1 match, A6=no tail (match at very end) | [x] |
| C7 | `searchAndReplace` | A2=match at index > 0, exactly 1 match, A6=tail present (the "generic" middle-match case) | [x] |
| C8 | `searchAndReplace` | A3=2 matches, A5=gap > 0 between them (`inx_start2 > from` taken) | [x] |
| C9 | `searchAndReplace` | A3=2 matches, A5=adjacent (gap == 0, `inx_start2 > from` NOT taken) | [x] |
| C10 | `searchAndReplace` | A3=many matches (3..12), randomized mixture of adjacent and separated, randomized leading/trailing context | [x] |
| C11 | `searchAndReplace` | A7=`value == ""` (deletion of every occurrence), with leading + trailing context and ≥ 2 matches | [x] |
| C12 | `searchAndReplace` | A7=`value_len < search_len` (shrinking rewrite), many matches | [x] |
| C13 | `searchAndReplace` | A7=`value_len == search_len` (in-place-size rewrite), many matches | [x] |
| C14 | `searchAndReplace` | A7=`value_len > search_len` (growing rewrite), many matches | [x] |
| C15 | `searchAndReplace` | A8=single-byte needle, `orig` consisting *only* of matches (`"XXXX"`, every position adjacent) | [x] |
| C16 | `searchAndReplace` | A8=self-overlapping multi-byte needle (`"aa"` in `"aaaa…"`), exercising the non-overlapping `orig + inx_start + search_len` rescan | [x] |
| C17 | `searchAndReplace` | A8=multi-byte needle with a partial-prefix decoy before the real match (`"abab"` needle, `"abaabab"` haystack) | [x] |
| C18 | `searchAndReplace` | A7 × A8: `value` *contains* `search` (checks the result is not rescanned/re-replaced) | [x] |
| C19 | `searchAndReplace` | A9=`orig_len == 1`, all of {match, no match} | [x] |
| C20 | `searchAndReplace` | A9=large `orig` (4 KiB–64 KiB) with hundreds of matches → hundreds of `realloc` round-trips | [x] |
| C21 | `searchAndReplace` | A10=high bytes 0x80–0xFF in `orig`, `search` and `value` (byte-wise, not UTF-8) | [x] |
| C22 | `searchAndReplace` | A10 × A9: fully randomized bytes 0x01–0xFF over a tiny alphabet so matches occur densely and unpredictably (the main property-style fuzz row: 200 000 cases) | [x] |
| C23 | `searchAndReplace` | A5 × A6 corner: match at index > 0, ≥2 matches, last match ends exactly at `orig_len` (tail branch skipped after the loop) | [x] |
| C24 | `searchAndReplace` | needle == haystack == value (identity rewrite), and needle == haystack with empty value | [x] |

All 24 rows are implemented as one `#[test]` each in `tests/valid_paths.rs`
(names `c1_…` … `c24_…`), together roughly **560 000 randomized differential
calls**. Every row is checked off only because it passes across those inputs —
see `all_configs.log` (`24 passed; 0 failed`, in all six build configurations).

## Why there is no binary/driver comparison

`c_src/CMakeLists.txt` contains only `add_library(driver SHARED src/lib.c)` —
there is no `add_executable`, and `translation/Cargo.toml` declares no `[[bin]]`.
The project therefore builds no driver executable, so there is no stdout to
compare; the entire observable surface is the one exported function, which is
compared byte-for-byte above.

## Harness self-check (mutation testing)

Passing tests only prove something if they can fail. `mutation_selfcheck.sh`
rebuilds the Rust `cdylib` 16 times, each time with one deliberate defect
injected, and re-runs this whole suite against the mutant:

* **13 behaviour-changing mutations — all 13 detected** (leading-copy guard
  off-by-one, overlapping rescan, `strstr` empty-needle contract, `strncpy`
  NUL-padding, `strdup` fast path, an added NULL check the C does not have,
  `total_bytes_allocated` off-by-one, frozen `inx_start` (detected via
  non-termination), dropped `from` resync, growing by `search_len` instead of
  `value_len`, tail-copy length, gap-copy source offset, misplaced final NUL).
* **3 provably semantically-equivalent mutations — all 3 survived**, as they
  must: `inx_start2 > from` → `>=` and `from < orig_len` → `<=` both only add a
  0-byte no-op branch, and the `from > 0` term at lib.c:78 is dead code (it can
  only be false for an empty needle, which never reaches line 78 because the
  loop does not terminate). Proofs are in the script's comments.

Result: `SELF-CHECK OK` (`mutation_selfcheck.log`).
