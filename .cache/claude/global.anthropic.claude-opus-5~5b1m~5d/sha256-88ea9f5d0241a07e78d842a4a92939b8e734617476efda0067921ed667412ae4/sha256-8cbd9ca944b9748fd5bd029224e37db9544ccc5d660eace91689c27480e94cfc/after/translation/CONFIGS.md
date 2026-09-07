# CONFIGS.md — Configuration surface table

Mechanically derived from the branches the C actually takes.

## Axes found in the C source

* **Public entry points** (`nm -D` on the C `.so`, cross-checked against
  `c_src/src/driver.c`):
  * `driver(int)` — the one-shot / convenience wrapper.
  * `printLine(const char *)` — the **lowest-level** entry point; it is
    non-`static` and therefore directly callable by an external consumer, so it
    is tested directly and not only through `driver`.
* **Runtime options / flags / modes:** none. There is no global state, no
  setter, no mode flag, no `#ifdef` in `driver.c` or `driver.h`
  (`grep -c '#if' c_src/src/driver.c` → only the header guard in `driver.h`).
  The library is stateless: the sole configuration axis is the argument value.
* **Branches:** `if (line != NULL)` in `printLine`; `if (data < 100)` in
  `driver`.
* **Input shapes the code special-cases:**
  * `driver`: sign of `data`; `data == 0`; `0 < data < 99`; `data == 99`
    (largest accepted, writes the final in-bounds byte); `data == 100`
    (boundary); `data > 100`; `data < 0` (UB).
  * `printLine`: `NULL`; empty string; 1 byte; many bytes; embedded-NUL buffer
    (only the prefix is printed); non-ASCII / high bytes; a 99-byte string
    equal to what `driver` produces.
* **Cross-cutting shape:** repeated / interleaved calls, to prove the libraries
  are stateless *and* that stdio buffering of the accumulated stream matches.
* **Byte order / element width / element type:** not applicable — the API takes
  a single `int` and a `char *`; there is no serialization, no width option and
  no endianness handling in the C.

## Rows (pruned cross-product of the axes the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | `data == 0` (degenerate: copy of 0 bytes, `dest[0] = 0`) | [x] |
| 2 | `driver` | `data == 1` (single byte copied) | [x] |
| 3 | `driver` | every `data` in `2..=98` exhaustively (interior of the accepted range) | [x] |
| 4 | `driver` | `data == 99` (largest accepted; `dest[99]` is the last in-bounds byte, and `strncpy` copies all 99 `'A'` without a NUL of its own) | [x] |
| 5 | `driver` | `data == 100` (first value rejected by `data < 100`; `dest` remains `""`) | [x] |
| 6 | `driver` | `data == 101` and randomized `data` in `101..=INT_MAX` (rejected range, seeded RNG) | [x] |
| 7 | `driver` | `data == INT_MAX` (extreme of the rejected range) | [x] |
| 8 | `driver` | randomized sweep over the whole accepted range `0..=99`, seeded, many iterations, order shuffled (catches value-dependent + index bugs) | [x] |
| 9 | `driver` | repeated calls with the *same* `data` (statelessness of `source`/`dest` re-init: `dest` must be re-zeroed each call) | [x] |
| 10 | `driver` | interleaved descending/ascending sequence, e.g. `99, 0, 98, 1, …` — a long `data` followed by a short one would expose a `dest` that is not re-zeroed | [x] |
| 11 | `printLine` (low-level, direct) | `NULL` pointer | [x] |
| 12 | `printLine` (low-level, direct) | empty string `""` | [x] |
| 13 | `printLine` (low-level, direct) | 1-byte string | [x] |
| 14 | `printLine` (low-level, direct) | randomized printable ASCII strings, lengths `0..=200`, seeded | [x] |
| 15 | `printLine` (low-level, direct) | randomized strings containing high bytes `0x80..=0xFF` (non-UTF-8; must be passed through verbatim, not re-encoded) | [x] |
| 16 | `printLine` (low-level, direct) | buffer with an **embedded NUL** (`"ab\0cd"`) — only the prefix before the NUL is printed | [x] |
| 17 | `printLine` (low-level, direct) | 99 `'A'` + NUL — exactly the buffer `driver(99)` builds, called directly | [x] |
| 18 | `printLine` (low-level, direct) | long string (4096 bytes) — crosses the stdio buffer size | [x] |
| 19 | `printLine` + `driver` | interleaved calls to both entry points in one captured stream (composed pipeline; checks the shared stdout stream and buffering order) | [x] |
| 20 | `driver` | `data < 0` — see `ERRORS.md` row 7; UB, verified by forked-subprocess signal comparison rather than by output comparison | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only
buildable configuration is the default one (there is nothing to pass to
`--features`). This is verified by `scripts/check_features.sh`, which extracts
the feature list from `Cargo.toml` and loops over it; it also runs
`--no-default-features` to confirm that configuration is identical.
