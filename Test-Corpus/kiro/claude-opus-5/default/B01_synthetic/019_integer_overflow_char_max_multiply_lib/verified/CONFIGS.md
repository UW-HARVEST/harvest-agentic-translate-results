# CONFIGS.md — Configuration surface table (Phase B)

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually branches on

**A. Runtime option / mode flag** — there is exactly one, the `driver` parameter:

| flag | set via | C branch | state it selects |
|---|---|---|---|
| `useGood` | `driver(int useGood)` line 91 | `if (useGood)` / `else` | `good()` (both mitigations) vs `bad()` (the overflow) |

There are no globals, no `#ifdef`s in `driver.c`, no init/teardown, no
environment lookups — the library is **stateless** (relevant axis: call order /
repetition must not change results).

**B. Public entry points** — the full set exported by the `.so`, not just the
top-level `driver` convenience wrapper:

| level | entry point | signature |
|---|---|---|
| lowest | `printLine` | `void(const char *)` |
| lowest | `printHexCharLine` | `void(char)` |
| mid | `bad` | `void(void)` |
| mid | `good` | `void(void)` (composes the two `static` helpers) |
| top | `driver` | `void(int)` |

`goodG2B` / `goodB2G` are `static`; they are reachable only *through* `good`, so
`good` is the lowest-level handle on them.

**C. Input shapes the code special-cases**

- `printLine`: NULL vs non-NULL (line 32); length 0 / 1 / many; bytes containing
  `%` conversion specifiers; bytes containing embedded newlines; high-bit
  (non-ASCII / negative `char`) bytes; long strings that cross the stdio buffer
  size (>4096) so that flush behaviour is exercised.
- `printHexCharLine`: the `char` value domain — `< 16` (needs `%02x`
  zero-padding), `16..=127` (2 digits, no padding), `0`, and `< 0` (sign-extended
  to 8 hex digits). Full 256-value sweep.
- `driver`: `0` vs non-zero; and among non-zero, values whose *low byte* is zero
  (`256`, `0x10000`, `INT_MIN`) to prove the whole `int` is tested.
- Composition: repeated / interleaved calls, to confirm statelessness and
  identical stdio buffering.

## Table — one row per combination the C treats differently

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `printLine` | non-NULL, single ASCII line, randomized length 1..64, randomized printable bytes | [x] |
| C2 | `printLine` | non-NULL, **empty** string `""` (length 0) | [x] |
| C3 | `printLine` | non-NULL, length exactly 1, swept over all 255 non-NUL byte values (incl. high-bit / negative `char`) | [x] |
| C4 | `printLine` | non-NULL, randomized bytes drawn from the **full** `1..=255` range (non-ASCII / UTF-8-invalid) | [x] |
| C5 | `printLine` | non-NULL, contains `%s`, `%n`, `%d`, `%%`, `%p` — must be treated as data, never as a format | [x] |
| C6 | `printLine` | non-NULL, contains embedded `\n`, `\t`, `\r` (multi-line payload) | [x] |
| C7 | `printLine` | non-NULL, **long** string crossing the stdio buffer (randomized lengths 4000..9000) | [x] |
| C8 | `printLine` | NULL pointer (the line-32 false branch) — see also ERRORS.md E1 | [x] |
| C9 | `printHexCharLine` | full sweep of all 256 `char` bit patterns `-128..=127` (covers `<16` padded, `>=16` unpadded, `0`, and sign-extended negatives) | [x] |
| C10 | `printHexCharLine` | randomized `char` values, 512 draws from a fixed seed | [x] |
| C11 | `bad` | no options; called once (exercises `CHAR_MAX` overflow path, lines 46-48) | [x] |
| C12 | `bad` | called repeatedly (randomized 1..16 repeats) — statelessness + accumulated buffered output | [x] |
| C13 | `good` | no options; called once (runs `goodG2B` then `goodB2G`, incl. the line-72 range-check rejection) | [x] |
| C14 | `good` | called repeatedly (randomized 1..16 repeats) | [x] |
| C15 | `driver` | `useGood = 0` → `bad()` path | [x] |
| C16 | `driver` | `useGood = 1` → `good()` path | [x] |
| C17 | `driver` | `useGood` = randomized non-zero `i32` (full range incl. negatives, fixed seed) → `good()` path | [x] |
| C18 | `driver` | `useGood` ∈ {`256`, `0x10000`, `0x7FFFFF00`, `INT_MIN`, `INT_MAX`, `-1`, `2`} — non-zero ints whose low byte may be `0` | [x] |
| C19 | `driver` | randomized **sequence** of mixed `0` / non-zero calls (interleaving both modes in one capture) | [x] |
| C20 | mixed pipeline | randomized interleaving of `driver`, `good`, `bad`, `printLine`, `printHexCharLine` in one capture — end-to-end composed byte stream | [x] |

Every row is driven from `tests/differential.rs::phase_b_*`, calling **both**
`.so`s through `libloading` and comparing captured `stdout` byte-for-byte.
Randomized rows use a fixed-seed xorshift PRNG (see `Rng` in the test file) so
runs are reproducible.

## Binary executable

`c_src/CMakeLists.txt` defines a single `add_library(driver SHARED ...)` target
and **no** `add_executable`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `src/main.rs` / `[[bin]]`. There is no
driver binary, so the "compare C and Rust stdout from the binaries" gate is
**not applicable**; the equivalent coverage is obtained by capturing `stdout` at
the file-descriptor level around every `.so` call (rows C1-C20).
