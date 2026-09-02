# CONFIGS.md — configuration surface for VALID inputs

Mechanical enumeration of the axes the C code actually branches on.

## Public entry points (the FULL set, from `nm -D --defined-only`)

| entry point | signature | level |
|-------------|-----------|-------|
| `foo` | `int foo(const char *in, char c)` | **low-level** — not in `driver.h`, but exported; takes the search byte as a parameter |
| `driver` | `void driver(const char *in)` | convenience wrapper — calls `foo(in,'A')` then `foo(in,'x')` and `printf`s both |

Tests drive `foo` directly (all byte values), not only through `driver`.

## Runtime options / modes / flags

There are **none**. Greps for `#ifdef`, `#if`, `switch`, and any global/static
state in `c_src/src/driver.c` return nothing beyond the `DRIVER_H_` include
guard; the library is stateless and has no configuration API. The only "option"
the API can set is the `c` parameter of `foo`, which is therefore treated as a
first-class configuration axis below.

## Axes the code distinguishes

* **A1 — search byte `c`** (the only runtime option): `'A'`, `'x'` (the two
  values `driver` hard-codes), other ASCII, digit, space, `0x01`, `0x7F`,
  high-bit `0x80..0xFF` (negative `c_char`). `c == 0` is excluded — it is
  undefined behaviour, see `ERRORS.md` row 5.
* **A2 — input length**: `0` (empty), `1`, small, ≥16, ≥32, ≥64, ≥4096
  (crosses the vector-width and page-size boundaries that glibc `strchr`'s
  aligned SIMD loop special-cases).
* **A3 — match count**: `0`, `1`, `2`, many, `strlen(in)` (all bytes match).
* **A4 — match position**: first byte, last byte before the terminator,
  interior only, adjacent/consecutive matches (exercises the `s++` step).
* **A5 — start alignment**: the string's first byte placed at offset
  `0..=63` inside a 64-byte-aligned allocation. glibc `strchr` reads aligned
  words, the Rust translation reads byte-by-byte; results must agree for every
  alignment.
* **A6 — byte content**: ASCII only vs. arbitrary bytes `0x01..0xFF`
  (non-UTF-8, high-bit set) — the API is byte-oriented.
* **A7 — digit width of the printed counts** (`driver` only, via `printf("%d")`):
  0, 1-digit, 2-digit, 3-digit, 4-digit, 5-digit; and the two counts differing
  from each other.

## Rows — meaningful combinations (cross-product, pruned to what the C distinguishes)

Every row is exercised with **many randomised inputs** (fixed seed
`0x5EED_1234_ABCD_0001`, deterministic SplitMix64), not a single hand-picked
value, and asserted byte-for-byte between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `foo` | A2=0 (empty string) × A1 = all 255 non-zero byte values | [x] |
| 2 | `foo` | A2=1 (single byte) × A1 = all 255 non-zero byte values × A6 = every possible single content byte | [x] |
| 3 | `foo` | A3=0 (no match) × A2 small/medium random × A1 random non-zero | [x] |
| 4 | `foo` | A3=1, A4=first byte × randomised remainder | [x] |
| 5 | `foo` | A3=1, A4=last byte before terminator × randomised prefix | [x] |
| 6 | `foo` | A3=1, A4=interior × randomised | [x] |
| 7 | `foo` | A3=2 with the two matches **adjacent** (consecutive bytes) | [x] |
| 8 | `foo` | A3=many, matches scattered at random positions, A2 random 0..512 | [x] |
| 9 | `foo` | A3=`strlen(in)` (all bytes match) × A2 ∈ {1,2,15,16,17,31,32,33,63,64,65,4095,4096,4097} | [x] |
| 10 | `foo` | A1 = high-bit byte (`0x80..0xFF`, negative `c_char`) × A6 = random arbitrary bytes | [x] |
| 11 | `foo` | A1 ∈ {`0x01`, `0x7F`} boundary search bytes × random content | [x] |
| 12 | `foo` | A5 = start alignment `0..=63` inside a 64-byte-aligned buffer × random content and random `c` | [x] |
| 13 | `foo` | A2 = length boundaries {15,16,17,31,32,33,63,64,65,127,128,129,4095,4096,4097} × random content × random `c` | [x] |
| 14 | `foo` | A6 = full random bytes `0x01..0xFF` (non-UTF-8), A2 random 0..1024, A1 random — broad property sweep | [x] |
| 15 | `foo` | A1 = `'A'` and `'x'` specifically (the bytes `driver` uses) × random content | [x] |
| 16 | `driver` | A7=0: input containing neither `'A'` nor `'x'` → `A: 0\nx: 0\n` | [x] |
| 17 | `driver` | A7: exactly one `'A'`, no `'x'` → counts `1` and `0` | [x] |
| 18 | `driver` | A7: no `'A'`, exactly one `'x'` → counts `0` and `1` | [x] |
| 19 | `driver` | A7: both present, 2-digit counts, counts differing from each other | [x] |
| 20 | `driver` | A7: 3-, 4- and 5-digit counts (long inputs) | [x] |
| 21 | `driver` | A2=0 (empty input) | [x] |
| 22 | `driver` | A6 = random arbitrary bytes incl. high-bit, A2 random 0..2048 — broad property sweep of the composed pipeline | [x] |
| 23 | `driver` | A5 = start alignment `0..=63` × random content | [x] |
| 24 | `foo` + `driver` | composed: the same buffer fed to `driver` and to `foo(in,'A')`/`foo(in,'x')`, asserting `driver`'s printed digits equal the low-level results in **both** libraries (catches wrapper/pipeline bugs invisible per-function) | [x] |
