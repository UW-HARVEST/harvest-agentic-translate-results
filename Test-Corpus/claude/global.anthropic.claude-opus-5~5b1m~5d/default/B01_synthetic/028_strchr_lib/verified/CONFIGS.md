# CONFIGS.md — Configuration-surface table (Phase B)

Mechanically derived from the branches the C code actually takes.

## Axes the C code distinguishes

`c_src/src/driver.c` has no options, no global state, no `#ifdef`s and no
`switch`. Everything it branches on comes from its two parameters:

* **Axis 1 — entry point.** Two public entry points:
  * `foo(const char *in, char c)` — the *low-level* one (not declared in
    `driver.h`, but exported; it takes an arbitrary needle).
  * `driver(const char *in)` — the convenience wrapper, hard-wired to the two
    needles `'A'` then `'x'`, printing `"A: %d\n"` then `"x: %d\n"` via
    `printf`. Its observable output is the **stdout byte stream**.
* **Axis 2 — the loop/branch inside `foo`.** The single branch is
  `s = strchr(s, c)` being NULL vs non-NULL, i.e. **number of occurrences**:
  0, 1, many. Plus the `s++` after a match, which makes **adjacent /
  overlapping matches** and **a match at the very last byte** (where `s++`
  lands exactly on the terminator) distinct shapes.
* **Axis 3 — needle value `c`.** Promoted `char`→`int`: printable ASCII,
  byte not present at all, `0x7F`, high-bit/negative bytes (`0x80..0xFF`),
  `'A'`/`'x'` specifically (the ones `driver` uses). (`c == 0` is UB → see
  `ERRORS.md` E3.)
* **Axis 4 — haystack shape.** Length 0 / 1 / small / large (up to 256 KiB);
  match position first / middle / last / all bytes; byte alphabet restricted
  ASCII vs full `0x01..0xFF` (non-UTF-8) so that any `str`-based translation
  would break.

Feature axis: `translation/Cargo.toml` declares **no `[features]` section**,
so the only configuration is the default one. `--no-default-features` is
verified to be identical (see `run_all.sh`).

## Table (cross-product, pruned to what the C distinguishes)

Every row is exercised with **many randomized inputs** (deterministic
`SplitMix64`, fixed seed `0x5DEECE66D`), comparing the C `.so` and the Rust
`.so` loaded side by side with `libloading`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `foo` | length-0 haystack (`""`), needle over all of `0x01..0xFF` → always 0 | [x] |
| C2 | `foo` | length-1 haystack, needle == the byte (1 match) and needle != the byte (0 matches), all 255 byte values | [x] |
| C3 | `foo` | needle absent from haystack (0 matches); random ASCII haystacks, 200 iterations | [x] |
| C4 | `foo` | exactly 1 match, at a random position; 200 random haystacks | [x] |
| C5 | `foo` | many non-adjacent matches; random density 10–50 %, 200 random haystacks | [x] |
| C6 | `foo` | **adjacent matches** (runs of the needle, e.g. `"AAAA"`), which exercise the `s++` past a match; randomized run lengths/positions | [x] |
| C7 | `foo` | match at the **first** byte | [x] |
| C8 | `foo` | match at the **last** byte (so `s++` lands exactly on the NUL terminator — boundary of the scan) | [x] |
| C9 | `foo` | haystack is entirely the needle (all bytes match), lengths 1..64 | [x] |
| C10 | `foo` | **high-bit / negative needle** (`0x80..0xFF`) with high-bit bytes present in the haystack (non-UTF-8 data) | [x] |
| C11 | `foo` | full random byte haystack over `0x01..=0xFF` × needle swept over all 255 non-zero byte values (255 × haystacks) | [x] |
| C12 | `foo` | large haystack: 64 KiB and 256 KiB, random needle, thousands of matches (counter magnitude) | [x] |
| C13 | `driver` | empty input `""` → `"A: 0\nx: 0\n"`; stdout compared byte-for-byte | [x] |
| C14 | `driver` | input with only `'A'`s (0 `'x'`) | [x] |
| C15 | `driver` | input with only `'x'`s (0 `'A'`) | [x] |
| C16 | `driver` | input with **both** `'A'` and `'x'`, randomized counts/positions, 200 iterations | [x] |
| C17 | `driver` | input with **neither** `'A'` nor `'x'` (incl. lowercase `'a'` / uppercase `'X'` near-misses → case sensitivity) | [x] |
| C18 | `driver` | input with adjacent `'A'`/`'x'` runs and matches at first/last byte | [x] |
| C19 | `driver` | non-UTF-8 input (random `0x80..0xFF` bytes mixed with `'A'`/`'x'`) | [x] |
| C20 | `driver` | large input (128 KiB) with many `'A'`/`'x'` → multi-digit counts in the printed output | [x] |
| C21 | `driver` + `foo` | **composed pipeline**: for the same buffer, assert the numbers `driver` prints equal `foo(buf,'A')` / `foo(buf,'x')` for *both* libraries and cross-wise (C stdout == Rust stdout == C `foo` == Rust `foo`) | [x] |
| C22 | `foo` | needle `'A'`/`'x'` specifically (the values `driver` hard-codes) on randomized buffers — low-level path under the wrapper's exact configuration | [x] |
