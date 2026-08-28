# CONFIGS.md — Phase B configuration-surface table

## Axes the C actually branches on

Derived mechanically from every `if` / loop condition in `c_src/src/lib.c`
(there are no `switch` and no `#ifdef` in the file):

| axis | source line | values the code distinguishes |
|------|-------------|-------------------------------|
| **A1** `ignore` pointer | `23` `ignore != NULL` | `NULL`, `""`, non-empty set |
| **A2** `state` parity when a non-hex byte is met | `23` `state == 0U` | even nibble (ignore honoured), odd nibble (ignore bypassed) |
| **A3** byte class of `hex[i]` | `22` `(c_num0 \| c_alpha0) == 0U` | digit `0-9`, upper `A-F`, lower `a-f`, non-hex |
| **A4** `bin_maxlen` vs digits supplied | `31` `bin_pos >= bin_maxlen` | `0`, short, exact, generous, `SIZE_MAX` |
| **A5** `state` when storing | `35` `state == 0U` | high nibble -> `c_acc`, low nibble -> `bin[]` |
| **A6** `hex_end_p` pointer | `50` `hex_end_p != NULL` | `NULL`, non-`NULL` |
| **A7** `hex_pos` vs `hex_len` at exit | `52` `hex_pos != hex_len` | fully consumed, stopped early |
| **A8** `hex_len` shape | `16` loop guard | `0`, `1` (odd), `2`, even, odd, long (1..=1024) |
| **A9** `bin`/`hex` aliasing | `17` read vs `38` write | disjoint buffers, `bin == hex` (in-place decode), `bin == NULL` |

`A3` is the only axis with an internal sub-structure worth pinning down: the
branch-free classifier accepts a byte iff `c ^ 48U < 10` (exactly `0x30..0x39`)
or `(c & ~32U) - 55U` lands in `10..=15` (exactly `0x41..0x46` and
`0x61..0x66`). Every configuration below is therefore also swept over the full
`0x00..=0xFF` byte domain by the randomized generator.

Every row is exercised with **many randomized inputs** (fixed-seed PRNG, 200–2000
cases per row) through both `.so` files, comparing the return value, the entire
`bin` buffer *plus a guard region past `bin_maxlen`*, and the exact `*hex_end_p`
pointer.

## Rows

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| C1  | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, hex = random even-length pure lowercase `[0-9a-f]` | [x] |
| C2  | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, hex = random even-length pure UPPERCASE `[0-9A-F]` | [x] |
| C3  | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, hex = random even-length **mixed** case | [x] |
| C4  | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, hex = random even-length **digits only** `[0-9]` | [x] |
| C5  | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, hex = random even-length **letters only** `[A-Fa-f]` | [x] |
| C6  | `hex2bin` | `ignore=NULL`, `hex_end_p` **non-NULL**, `bin_maxlen` exact, random valid hex — checks `*hex_end_p == hex+hex_len` | [x] |
| C7  | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` **generous** (`hex_len/2 + rand`), random valid hex; guard region must stay untouched | [x] |
| C8  | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen = SIZE_MAX`, random valid hex (oversized-length axis A4) | [x] |
| C9  | `hex2bin` | `hex_len = 0` × {`ignore` NULL/`""`/non-empty} × {`hex_end_p` NULL/non-NULL} — empty-input cross-product | [x] |
| C10 | `hex2bin` | `hex_len = 2` (single byte, minimal non-empty even) × all 3 `ignore` variants × both `hex_end_p` variants | [x] |
| C11 | `hex2bin` | Long input: `hex_len` up to 1024 random valid hex digits, `bin_maxlen` exact, both `hex_end_p` variants | [x] |
| C12 | `hex2bin` | `ignore=" "` (A1 non-empty), separators only at **even** boundaries (A2 even) so they are skipped, `hex_end_p` non-NULL | [x] |
| C13 | `hex2bin` | `ignore=": -"` multi-char set, separators at even boundaries, runs of 1..4 consecutive separators, `hex_end_p` non-NULL | [x] |
| C14 | `hex2bin` | `ignore` non-empty, **leading** separators before the first digit (`state==0` at index 0) | [x] |
| C15 | `hex2bin` | `ignore` non-empty, **trailing** separators after the last digit — stops at even boundary, `hex_pos == hex_len`, so still success | [x] |
| C16 | `hex2bin` | `ignore` non-empty, separator placed at an **odd** boundary (A2 odd) -> ignore bypassed, scan stops early; `hex_end_p` non-NULL so it is reported, not an error | [x] |
| C17 | `hex2bin` | `ignore=""` (empty set, A1) with random valid hex — `strchr("",c)` matches nothing but NUL | [x] |
| C18 | `hex2bin` | `ignore` set containing **hex digits** (`"0123abc"`) — dead entries, digits must still decode (never ignored) | [x] |
| C19 | `hex2bin` | Early stop reported via `hex_end_p`: random valid prefix + random **non-hex** byte + random tail, `hex_end_p` non-NULL -> success + partial count | [x] |
| C20 | `hex2bin` | Class-boundary bytes `/ : @ G ` g` (one step outside `[0-9A-Fa-f]`) spliced at random positions, `hex_end_p` non-NULL | [x] |
| C21 | `hex2bin` | **Full byte sweep**: for every `b` in `0x00..=0xFF`, `hex=[b]` and `hex=['a', b]`, × `ignore` NULL / `""` / `" "` / `[b]` , × `hex_end_p` NULL/non-NULL | [x] |
| C22 | `hex2bin` | **Embedded NUL** at an even boundary with `ignore` non-NULL (the `strchr`-terminator quirk) and with `ignore=NULL`, `hex_end_p` non-NULL | [x] |
| C23 | `hex2bin` | High bytes `0x80..=0xFF` spliced into valid hex (signed-`char` sign-extension axis) | [x] |
| C24 | `hex2bin` | `bin_maxlen` **short** (0..hex_len/2) with random valid hex — overflow path with `hex_end_p` non-NULL; asserts the *partially written* `bin` bytes match | [x] |
| C25 | `hex2bin` | Odd-length valid hex (`hex_len` odd) with `hex_end_p` non-NULL — the `hex_pos--` rewind path | [x] |
| C26 | `hex2bin` | Fully unstructured fuzz: `hex` = random bytes from `0x00..=0xFF`, random `hex_len` 0..64, random `bin_maxlen` 0..40, random `ignore` (NULL / random NUL-terminated set), random `hex_end_p` — the cross-product catch-all | [x] |
| C27 | `hex2bin` | **In-place decoding**: `bin` and `hex` are the *same address*. The C interleaves `bin[bin_pos]` writes with `hex[hex_pos]` reads (always `bin_pos < hex_pos`), so the read/write ordering is observable. Crossed with `bin_maxlen` short/exact/generous, separators, odd lengths, boundary bytes, all `ignore` variants, both `hex_end_p` variants | [x] |
| C28 | `hex2bin` | `ignore` set built from the bytes present in `hex` (incl. an embedded NUL in `hex`), both with a separate and an aliased `bin` buffer | [x] |

Note on the API surface: `c_src/include/lib.h` exports exactly one entry point,
`hex2bin`, and it *is* the lowest-level entry point — there is no convenience or
one-shot wrapper layered on top of it, and no separate init/state object. All 26
rows therefore drive the lowest-level function directly with full option control.
