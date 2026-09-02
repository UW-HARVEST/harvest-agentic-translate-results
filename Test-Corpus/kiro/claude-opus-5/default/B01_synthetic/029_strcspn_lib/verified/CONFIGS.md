# CONFIGS.md — Phase A configuration-surface table

## How this table was derived

The public surface is one entry point, taken verbatim from
`c_src/include/driver.h`:

```c
void driver(const char *s1, const char *s2);
```

There is no init/teardown, no handle, no context struct, no runtime
option/mode/flag setter, no `#ifdef` in the library, and no second (lower-level)
entry point — `nm -D` on the C `.so` confirms `driver` is the *entire* exported
surface, so "exercise the low-level entry points, not just the convenience
wrappers" collapses to: exercise `driver` itself, which is simultaneously the
lowest-level and the only entry point.

So the configuration axes are not option flags but the **input shapes the C code
actually branches on**. The branches live in the `strcspn` call `driver` makes,
and they are:

* **A1 — reject-set size** (`s2`): glibc's generic `strcspn` branches on
  `reject[0] == '\0'` (empty ⇒ degenerates to `strlen(s1)`) and on
  `reject[1] == '\0'` (single character ⇒ degenerates to `strchrnul`); anything
  longer takes the table/SIMD path. The x86-64 SSE4.2 path additionally
  distinguishes reject sets that fit in one 16-byte vector from longer ones.
  Distinguished values: 0, 1, 2, 15, 16, 17, 31, 32, 33, many.
* **A2 — match position in `s1`**: match at index 0 (result `0`), match in the
  middle, match at the very last byte, and no match at all (result
  `strlen(s1)`). This is the value-dependent axis that a single hand-picked
  input cannot cover.
* **A3 — `s1` length**: 0, 1, small, spanning the 16/32-byte SIMD block sizes,
  and long enough that the result needs several `%zu` digits.
* **A4 — byte domain**: ASCII-only vs. bytes in `0x80..=0xFF`. `char` is
  *signed* on x86-64, so a naive translation that compares `c_char` values or
  indexes a table with a sign-extended `char` diverges here. Also: the full
  1..=255 alphabet, and NUL handling (`s2`'s terminating NUL must **not** count
  as a member of the reject set, so a `s1` byte is never "matched" by it).
* **A5 — reject-set redundancy**: duplicate bytes in `s2`, and `s2` containing
  every byte that occurs in `s1` (result always `0` unless `s1` is empty).
* **A6 — pointer alignment / placement**: glibc's `strcspn` uses *aligned*
  16-byte SIMD loads, so behaviour must be checked with `s1` and `s2` starting
  at every offset 0..16 within their buffer, and with a string placed so it ends
  immediately before a page boundary (the case where over-reading would fault).
* **A7 — output formatting**: the result is printed with `%zu\n`, so results
  spanning 1, 2, 3, 4 and 5+ decimal digits must be compared byte-for-byte
  (including the trailing newline and the absence of any other bytes).

Rows below are the cross-product of those axes, pruned to the combinations the C
actually treats differently. Every row is exercised with **many randomized
inputs** (fixed seed, deterministic xorshift PRNG in `tests/common/mod.rs`), not
a single hand-picked value, and compared byte-for-byte between the C `.so` and
the Rust `.so`.

## The table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `driver` | `s1` empty, `s2` empty — both degenerate; expect `0` | [x] |
| C2 | `driver` | `s1` empty, `s2` non-empty (randomized size/content) — expect `0` | [x] |
| C3 | `driver` | `s1` non-empty (randomized), `s2` empty — `strlen` path; expect `strlen(s1)` | [x] |
| C4 | `driver` | `s1` single byte, `s2` single byte, all 255×255 non-NUL combinations | [x] |
| C5 | `driver` | `s2` single byte (`strchrnul` fast path), `s1` randomized ASCII of random length | [x] |
| C6 | `driver` | `s2` exactly 2 bytes — first size that leaves the fast paths; `s1` randomized | [x] |
| C7 | `driver` | `s2` of size 15 / 16 / 17 (one step either side of the 16-byte SIMD block), `s1` randomized | [x] |
| C8 | `driver` | `s2` of size 31 / 32 / 33 (one step either side of two SIMD blocks), `s1` randomized | [x] |
| C9 | `driver` | `s2` large (64..255 bytes, randomized), `s1` randomized | [x] |
| C10 | `driver` | match at index 0 of `s1` — forced by construction; expect `0` | [x] |
| C11 | `driver` | match in the middle of `s1` — forced at a randomized index | [x] |
| C12 | `driver` | match at the final byte of `s1`; expect `strlen(s1)-1` | [x] |
| C13 | `driver` | no match at all: `s1` and `s2` drawn from disjoint byte sets; expect `strlen(s1)` | [x] |
| C14 | `driver` | high-bit bytes `0x80..=0xFF` only, in both `s1` and `s2` (signed-`char` sign-extension hazard) | [x] |
| C15 | `driver` | mixed ASCII + high-bit bytes in both arguments, randomized | [x] |
| C16 | `driver` | `s1` and `s2` drawn from the full `0x01..=0xFF` alphabet, randomized | [x] |
| C17 | `driver` | `s2` full 255-byte alphabet (every non-NUL byte present) — result is `0` for any non-empty `s1` | [x] |
| C18 | `driver` | `s2` with heavy duplicate bytes (randomized, drawn from a 2-byte alphabet, length up to 200) | [x] |
| C19 | `driver` | `s2` a superset of `s1`'s bytes; expect `0` (or `0` for empty `s1`) | [x] |
| C20 | `driver` | `s1` lengths 0..=64 exhaustively, randomized content, randomized `s2` — spans the SIMD block boundaries | [x] |
| C21 | `driver` | long `s1` (1 KiB..64 KiB) with the match placed at a randomized index — multi-digit `%zu` output | [x] |
| C22 | `driver` | long `s1` (up to 100000 bytes) with **no** match — 5–6 digit `%zu` output, exercises the widest formatting | [x] |
| C23 | `driver` | `s1` at every start offset 0..=16 inside its buffer (unaligned SIMD loads), randomized content and `s2` | [x] |
| C24 | `driver` | `s2` at every start offset 0..=16 inside its buffer (unaligned SIMD loads), randomized content and `s1` | [x] |
| C25 | `driver` | `s1` placed so its terminating NUL is the last readable byte before a `PROT_NONE` page (over-read would fault), `s2` randomized | [x] |
| C26 | `driver` | `s2` placed so its terminating NUL is the last readable byte before a `PROT_NONE` page, `s1` randomized | [x] |
| C27 | `driver` | both `s1` and `s2` page-guarded simultaneously, randomized lengths 0..=40 | [x] |
| C28 | `driver` | repeated calls in one process, interleaved between the C and the Rust `.so`, to confirm no residual state and identical stdio buffering | [x] |
| C29 | `driver` | broad randomized fuzz sweep: 20000 iterations, random lengths (0..=512), random alphabet size (1..=255), random alphabet contents | [x] |

## Test mapping

Rows map to tests in `tests/valid_paths.rs` by name (`C1` → `c1_both_empty`,
etc.). The one exception is C7 and C8, which share the single test
`c7_c8_reject_sizes_around_simd_blocks` — it iterates the reject-set sizes
`15, 16, 17, 31, 32, 33`, so both rows are exercised there. That gives 28 tests
for 29 rows.

## Feature combinations

`Cargo.toml` has no `[features]` section, so the only two build configurations
are the default build and `--no-default-features`; `run_all.sh` runs the whole
suite under both.
