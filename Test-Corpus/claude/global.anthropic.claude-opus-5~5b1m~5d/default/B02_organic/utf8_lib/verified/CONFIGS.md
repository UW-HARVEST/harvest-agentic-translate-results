# CONFIGS.md — Phase B configuration-surface table

## Axes mechanically derived from `c_src/src/lib.c`

**Public entry points** (the full set, lowest-level first):

* `w_utf8_drop(const char *)` — the low-level scanner. Not in `lib.h`, but exported
  (`nm -D` shows `T w_utf8_drop`), so it is a real public entry point and is driven
  **directly**, not only through the `w_utf8_filter` wrapper.
* `w_utf8_filter(const char *, bool)` — the convenience/one-shot wrapper.

**Runtime options** (the only one the API can set):

* `replacement` (`bool`): toggles the `if (replacement)` block at `lib.c:97`.
  * `false` → invalid bytes are silently **dropped** (`valid++` only); output is never
    longer than the input, so the initial `malloc(strlen+1)` is never grown and the
    `repl`/`realloc` code is dead.
  * `true` → each invalid byte emits `EF BF BD` (U+FFFD) and the
    `repl < 3` / `size += REPLACEMENT_INC` / `realloc` growth machinery is live.
  * non-canonical byte values (`2`, `0xFF`, ...) — see ERRORS.md E21.

**Internal branch/state axis** in `w_utf8_filter`:

* `*valid == '\0'` (input is entirely valid) → `strdup(string)` fast path.
  Allocation is `strlen+1` from `strdup`, `replacement` is irrelevant.
* `*valid != '\0'` (input has ≥1 invalid byte) → `malloc` + `memcpy` prefix path.
  The `memcpy` length `i = valid - string` makes the **offset of the first invalid
  byte** an axis in its own right (0 / 1 / mid-string / just before the NUL).

**Input-shape axes the code special-cases** (one branch per `valid_N` macro and
per sub-condition inside them):

* length: empty (0), 1 byte, 2, 3, 4, sub-4096, ~4096, > 4096 bytes.
* `valid_1`: `00`-`7F` (incl. the NUL loop terminator).
* `valid_2`: lead `C2`-`DF`; rejected leads `C0`,`C1`; continuation in/out of `80`-`BF`.
* `valid_3`: lead `E0`-`EF`; special leads `E0` (needs b1 ≥ `A0`), `ED` (needs b1 < `A0`),
  `EF` (needs b1 ≤ `BF`, vacuously true); generic leads `E1`-`EC`,`EE`.
* `valid_4`: lead `F0`-`F4`; special leads `F0` (needs b1 ≥ `90`), `F4` (needs b1 ≤ `8F`);
  generic leads `F1`-`F3`; rejected leads `F5`-`F7` (and `F8`-`FF` match nothing).
* bare continuation bytes `80`-`BF` as a lead → matches nothing.
* truncated sequences abutting the NUL terminator (short-circuit / no over-read).
* count and clustering of invalid bytes: 0 / 1 / a few / many (> 1365, to force
  repeated `realloc` cycles) / all bytes invalid / alternating valid+invalid.
* signedness trap: bytes ≥ `0x80` (the C compares `char` **signed** in
  `(x)[0] >= (char)0xC2` and `(x)[0] != (char)0xE0`, but **unsigned** in
  `(unsigned char)(x)[1] >= 0xA0`) — every row uses high bytes.

No `#ifdef`, no `switch`, no other flags, no byte-order or element-type axes exist
in this library.

## Rows (cross-product, pruned to what the C actually distinguishes)

Every row is driven with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG in `tests/common/mod.rs`), and both `.so`s are called through
`libloading` and compared byte-for-byte (plus, for `w_utf8_drop`, the returned
*offset* is compared, and for `w_utf8_filter` the full NUL-terminated bytes).

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| C1  | `w_utf8_drop` | empty string `""` | [x] |
| C2  | `w_utf8_drop` | random pure-ASCII (`01`-`7F`), lengths 1..64 | [x] |
| C3  | `w_utf8_drop` | random valid 2-byte sequences only (lead `C2`-`DF`), incl. boundary leads `C2`/`DF` | [x] |
| C4  | `w_utf8_drop` | random valid 3-byte sequences only, generic leads `E1`-`EC`,`EE` | [x] |
| C5  | `w_utf8_drop` | 3-byte with lead `E0`, b1 swept over the whole `80`-`BF` range (only ≥ `A0` valid) | [x] |
| C6  | `w_utf8_drop` | 3-byte with lead `ED`, b1 swept over `80`-`BF` (only < `A0` valid — surrogates) | [x] |
| C7  | `w_utf8_drop` | 3-byte with lead `EF`, b1 swept over `80`-`BF` (the vacuous `<= BF` condition) | [x] |
| C8  | `w_utf8_drop` | random valid 4-byte sequences, generic leads `F1`-`F3` | [x] |
| C9  | `w_utf8_drop` | 4-byte with lead `F0`, b1 swept over `80`-`BF` (only ≥ `90` valid) | [x] |
| C10 | `w_utf8_drop` | 4-byte with lead `F4`, b1 swept over `80`-`BF` (only ≤ `8F` valid) | [x] |
| C11 | `w_utf8_drop` | rejected leads `C0`,`C1`,`F5`-`F7`,`F8`-`FF` and bare continuations `80`-`BF`, each at offset 0 | [x] |
| C12 | `w_utf8_drop` | **exhaustive**: all 256 single-byte strings; all 65 536 two-byte strings; all 16 777 216 three-byte strings (offset compared for every one) | [x] |
| C13 | `w_utf8_drop` | truncated sequence at end of buffer, string placed flush against an unmapped guard page (over-read detection) | [x] |
| C14 | `w_utf8_drop` | fully random bytes, lengths 0..64, thousands of cases (mixed valid/invalid, arbitrary clustering) | [x] |
| C15 | `w_utf8_drop` | long random inputs, lengths 1000..9000 (crossing the 4096 boundary) | [x] |
| C16 | `w_utf8_filter` | `replacement=false`, fully valid input → `strdup` fast path (ASCII / 2 / 3 / 4-byte, and empty) | [x] |
| C17 | `w_utf8_filter` | `replacement=true`, fully valid input → `strdup` fast path (must be identical to C16) | [x] |
| C18 | `w_utf8_filter` | `replacement=false`, first invalid byte at offset 0 (`memcpy` length 0) | [x] |
| C19 | `w_utf8_filter` | `replacement=true`, first invalid byte at offset 0 | [x] |
| C20 | `w_utf8_filter` | `replacement=false`, first invalid byte mid-string (non-zero `memcpy` prefix) | [x] |
| C21 | `w_utf8_filter` | `replacement=true`, first invalid byte mid-string | [x] |
| C22 | `w_utf8_filter` | `replacement=false`, invalid byte as the **last** byte before the NUL | [x] |
| C23 | `w_utf8_filter` | `replacement=true`, invalid byte as the last byte before the NUL | [x] |
| C24 | `w_utf8_filter` | `replacement=false`, alternating valid/invalid bytes, all four `valid_N` widths interleaved | [x] |
| C25 | `w_utf8_filter` | `replacement=true`, alternating valid/invalid, all four widths interleaved | [x] |
| C26 | `w_utf8_filter` | `replacement=false`, **all** bytes invalid (output is empty string) | [x] |
| C27 | `w_utf8_filter` | `replacement=true`, all bytes invalid (output is 3× input length) | [x] |
| C28 | `w_utf8_filter` | `replacement=true`, exactly 1 / 2 / 1365 / 1366 / 1367 / 2731 / 2732 invalid bytes — the `repl < 3` refill boundaries (`4096 / 3 = 1365.33`) | [x] |
| C29 | `w_utf8_filter` | `replacement=true`, > 5000 invalid bytes → multiple `realloc` growth cycles | [x] |
| C30 | `w_utf8_filter` | both `replacement` values, truncated multi-byte sequence at end of string | [x] |
| C31 | `w_utf8_filter` | both `replacement` values, fully random bytes, lengths 0..64, thousands of cases | [x] |
| C32 | `w_utf8_filter` | both `replacement` values, long random inputs, lengths 1000..9000 | [x] |
| C33 | `w_utf8_filter` | **exhaustive**: all 256 single-byte and all 65 536 two-byte inputs, for `replacement` ∈ {0,1} | [x] |
| C34 | `w_utf8_filter` | non-canonical `bool` bytes `2,3,0x7F,0x80,0xFF` on inputs containing invalid bytes (ERRORS.md E21) | [x] |
| C35 | `w_utf8_drop` + `w_utf8_filter` composed | pipeline: `w_utf8_filter(s, r)` result fed back through `w_utf8_drop` — the filtered output must be fully valid (drop returns the terminating NUL) for `r=1`, and for `r=0`; compared across both `.so`s | [x] |
| C36 | `w_utf8_filter` | returned pointer is `free()`-able (allocated by the same libc `malloc`/`strdup`) — checked for both `.so`s on both paths | [x] |
