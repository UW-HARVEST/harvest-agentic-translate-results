# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

Derived mechanically from the branch structure of `c_src/src/lib.c`, not from
what looks important. The exported API is two functions plus five mutable
tables, so "configuration" here means *the runtime state and input shape the C
code actually branches on*.

## Axes the C code branches on

**`cp_inflate` (L318-379)**

| axis | source of the branch | values the C distinguishes |
|---|---|---|
| `in` pointer alignment | `first_bytes = ((in+3) & ~3) - in` (L327) — this changes `words`, `word_count`, `last_bytes`, the initial `bits`/`count` prime, **and** `cp_ptr` | `0, 1, 2, 3` |
| trailing partial word | `last_bytes = (in_bytes - first_bytes) & 3` → `final_word_available` (L332, L336) | `0` (no final word) vs `1, 2, 3` |
| `bfinal` loop | `do { … } while (!bfinal)` (L340, L375) | 1 block, 2 blocks, N blocks |
| `btype` | `switch (btype)` (L342) | `0` stored, `1` fixed, `2` dynamic (`3` → `ERRORS.md` E6) |
| `out_bytes` slack | `out_end` comparisons in `cp_block` (L260, L288) | exact fit vs slack |

**`cp_stored` (L170-198)**

| axis | source | values |
|---|---|---|
| bit-alignment fixup | `cp_read_bits(s, s->count & 7)` (L172) | `count & 7` ∈ 0…7 (driven by block position) |
| `LEN` | `memcpy` size (L194) | `0`, `1`, small, large |

**`cp_fixed` (L200-204) / `cp_build` (L136-166)**

| axis | source | values |
|---|---|---|
| `s` non-NULL | `if (s) memset(s->lookup…)`, `if (s && len <= 9)` (L149, L157) | `lit`/CL tree built **with** lookup, `dst` tree built **without** |
| code length | `if (s && len <= 9)` (L157) | `len <= 9` (fills `lookup`) vs `len > 9` |

**`cp_decode` (L206-221)** — binary search over `tree[0..hi)`, reads `tree[lo-1]`

| axis | source | values |
|---|---|---|
| `hi` | `s->nlit` / `s->ndst` / `s->nlen` | 1 (single-symbol tree) … 288 |

**`cp_block` (L253-313)**

| axis | source | values |
|---|---|---|
| symbol class | `if (symbol < 256) … else if (symbol > 256) … else break` (L256, L269, L310) | literal, match, end-of-block |
| length code | `cp_len_extra_bits[symbol]` (L272) | all 29 codes, extra bits `0…5`, each with min and max extra value ⇒ lengths `3…258` |
| distance code | `cp_dist_extra_bits[distance_symbol]` (L276) | all 30 codes, extra bits `0…13` ⇒ distances `1…32768` |
| copy strategy | `switch (backwards_distance) { case 1: memset … default: bytewise }` (L303) | `distance == 1` (memset) vs `distance > 1` |
| overlap | bytewise loop copies forward through freshly written bytes | `distance >= length` (no overlap) vs `distance < length` (self-propagating) |

**`cp_dynamic` (L223-251)**

| axis | source | values |
|---|---|---|
| `nlit` | `257 + read_bits(5)` (L225) | `257` … `288` |
| `ndst` | `1 + read_bits(5)` (L226) | `1` … `32` |
| `nlen` | `4 + read_bits(4)` (L227) | `4` … `19` |
| CL symbol | `switch (sym) { case 16 / 17 / 18 / default }` (L232) | literal length, repeat-prev `3…6`, repeat-zero `3…10`, repeat-zero `11…138` |
| empty distance tree | zlib emits `HDIST=1` with length `0` when a block has no matches ⇒ `ndst == 0`; legal because `cp_decode(s->dst,…)` is never reached | present |

**`unfilter` (L417-478)**

| axis | source | values |
|---|---|---|
| `h` sign | `if (h > 0)` (L421), `for (y = 1; y < h; …)` (L451) | `h < 0`, `h == 0`, `h == 1`, `h >= 2` |
| row-0 filter | `switch (*raw++)` (L422) — cases `1`, `3`, `4` have **no** `prev`, cases `0` and `2` are both no-ops | `0, 1, 2, 3, 4` |
| row-`y` filter | `switch (*raw++)` (L452) — case `2`'s two loops are identical, case `1`'s prologue adds `0` | `0, 1, 2, 3, 4` |
| `len = w * bpp` | loop bounds (L418) | `len == 0`, `len == bpp`, `len > bpp`, `len < bpp` |
| `bpp` | prologue/main-loop split | `1, 2, 3, 4, 8` |
| byte values | `cp_paeth` three-way select (L383), `/2` truncation, `wrapping_add` | `0x00`, `0xFF`, random (all three `paeth` outcomes) |

**Exported tables** — all five are non-`static`, so they are part of the
configuration surface: `cp_fixed_table` feeds `cp_fixed`, `cp_permutation_order`
feeds `cp_dynamic`, and `cp_len_*` / `cp_dist_*` feed `cp_block`. Their contents
must be byte-identical or every derived decode diverges.

## Row table

Each row is exercised with many randomized inputs (fixed-seed xorshift64\*, see
`tests/common/mod.rs`), C and Rust called through their own `.so` exports, and
compared byte-for-byte on: return value, the whole `out` buffer, the whole `raw`
buffer, and the `cp_error_reason` string.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `cp_inflate` | `btype=0` stored, single final block, `in` align `0`, `LEN` random `0…4096` (sweeps `last_bytes` 0-3). See the `cp_ptr` note below | [x] |
| C2 | `cp_inflate` | `btype=0` stored, `in` align `1` | [x] |
| C3 | `cp_inflate` | `btype=0` stored, `in` align `2` | [x] |
| C4 | `cp_inflate` | `btype=0` stored, `in` align `3` | [x] |
| C5 | `cp_inflate` | `btype=0` stored, `LEN ∈ {0, 1, 2, 3, 4}` boundary sizes, all 4 alignments | [x] |
| C6 | `cp_inflate` | `btype=1` fixed, literals only, all 4 `in` alignments × all 4 `last_bytes`, random payload `1…3000` bytes | [x] |
| C7 | `cp_inflate` | `btype=1` fixed, literal payload restricted to `0…143` (8-bit codes) then to `144…255` (9-bit codes), forcing both `cp_build` lookup branches | [x] |
| C8 | `cp_inflate` | `btype=1` fixed, matches with `distance == 1` (the `memset` path), lengths `3…258` | [x] |
| C9 | `cp_inflate` | `btype=1` fixed, matches with `distance > 1` and `distance >= length` (non-overlapping bytewise copy) | [x] |
| C10 | `cp_inflate` | `btype=1` fixed, overlapping matches `1 < distance < length` (self-propagating bytewise copy) | [x] |
| C11 | `cp_inflate` | `btype=1` fixed, sweep of **every** length code `0…30` at min and max extra-bit value. Codes 29 and 30 have `cp_len_base == 0`, so they decode to a *zero-length* copy that still consumes a distance code — reachable through fixed literal symbols 286/287 | [x] |
| C12 | `cp_inflate` | `btype=1` fixed, sweep of **every** distance code `0…29` at min and max extra-bit value (output ≥ 32 KiB where needed) | [x] |
| C13 | `cp_inflate` | `btype=1` fixed, random mix of literals + matches, `out_bytes` exactly equal to the decoded size | [x] |
| C14 | `cp_inflate` | `btype=1` fixed, same stream, `out_bytes` with slack | [x] |
| C15 | `cp_inflate` | `btype=2` dynamic, minimum *reachable* `nlen`. `nlen = 4` carries only permutation slots `{16,17,18,0}`, so every literal length would be 0 and the literal tree empty — an error path (`ERRORS.md` A10), not a valid one. The smallest decodable HCLEN is **5**: a flat 256-symbol length-8 literal tree uses only code-length symbols `{0, 8}` (permutation indices 3 and 4). Asserted with `dynamic_hclen()` | [x] |
| C16 | `cp_inflate` | `btype=2` dynamic, `nlen = 19` (maximum HCLEN) | [x] |
| C17 | `cp_inflate` | `btype=2` dynamic, `nlit = 257`, `ndst = 1` (both minima) | [x] |
| C18 | `cp_inflate` | `btype=2` dynamic, `nlit = 288`, `ndst = 32` (both maxima) | [x] |
| C19 | `cp_inflate` | `btype=2` dynamic, code-length symbol `16` (repeat previous, `3…6`) present | [x] |
| C20 | `cp_inflate` | `btype=2` dynamic, code-length symbol `17` (repeat zero, `3…10`) present | [x] |
| C21 | `cp_inflate` | `btype=2` dynamic, code-length symbol `18` (repeat zero, `11…138`) present | [x] |
| C22 | `cp_inflate` | `btype=2` dynamic, all three of `16`/`17`/`18` in one header, code lengths up to 15 | [x] |
| C23 | `cp_inflate` | `btype=2` dynamic, empty distance tree (`ndst == 0` from `cp_build`), literals only | [x] |
| C24 | `cp_inflate` | `btype=2` dynamic, matches with `distance == 1`, `> 1`, and overlapping | [x] |
| C25 | `cp_inflate` | multi-block: `N ∈ 2…6` fixed blocks, only the last with `bfinal` | [x] |
| C26 | `cp_inflate` | multi-block: random mix of fixed and dynamic blocks | [x] |
| C27 | `cp_inflate` | multi-block ending in a stored block (the only position `ERRORS.md` E2 permits) | [x] |
| C28 | `cp_inflate` | real zlib-produced raw-deflate streams (`libz`, `windowBits=-15`), levels `0…9`, strategies default/filtered/huffman-only/RLE/fixed, random and highly-repetitive payloads `0…64 KiB` | [x] |
| C29 | `cp_inflate` | `out` buffer alignment `0…3` with an otherwise fixed stream | [x] |
| C30 | globals | byte-compare `cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`, `cp_dist_extra_bits`, `cp_dist_base` across the two `.so`s | [x] |
| C31 | globals | `cp_error_reason` is `NULL` in a freshly loaded `.so`, and is left untouched by a successful `cp_inflate` | [x] |
| C32 | internal layout | `cp_state_t` field offsets identical (`lookup[511]`/`lit[0]`, `lit[287]`/`dst[0]`, `dst[31]`/`len[0]` adjacency required by `cp_decode`'s `tree[-1]`) | [x] |
| C33 | `unfilter` | `h == 0`, random `w`/`bpp`, `raw` must be returned untouched, result `1` | [x] |
| C34 | `unfilter` | `h < 0` (`-1`, `-100`, `INT_MIN/2`), `raw` untouched, result `1` | [x] |
| C35 | `unfilter` | `h == 1`, row-0 filter `0`, `bpp ∈ {1,2,3,4,8}`, `w ∈ {0,1,2,7,16,64}` | [x] |
| C36 | `unfilter` | `h == 1`, row-0 filter `1` (Sub, no `prev`), same `bpp`/`w` grid | [x] |
| C37 | `unfilter` | `h == 1`, row-0 filter `2` (Up ⇒ no-op on row 0), same grid | [x] |
| C38 | `unfilter` | `h == 1`, row-0 filter `3` (Average, `raw[x-bpp]/2` only), same grid | [x] |
| C39 | `unfilter` | `h == 1`, row-0 filter `4` (Paeth with `b=c=0`), same grid | [x] |
| C40 | `unfilter` | `h >= 2`, **uniform** filter `0` on every row, `bpp`/`w` grid | [x] |
| C41 | `unfilter` | `h >= 2`, uniform filter `1` (Sub, with the `x < bpp` `+= 0` prologue) | [x] |
| C42 | `unfilter` | `h >= 2`, uniform filter `2` (Up, both identical loops) | [x] |
| C43 | `unfilter` | `h >= 2`, uniform filter `3` (Average, `prev[x]/2` prologue then `(raw[x-bpp]+prev[x])/2`) | [x] |
| C44 | `unfilter` | `h >= 2`, uniform filter `4` (Paeth, `prev[x]` prologue then full 3-arg paeth) | [x] |
| C45 | `unfilter` | `h >= 2`, random per-row filter mix from `{0,1,2,3,4}`, random `w`/`h`/`bpp` | [x] |
| C46 | `unfilter` | `len == 0` (`w == 0` with `bpp > 0`, and `bpp == 0` with `w > 0`) — every inner loop empty, one filter byte still consumed per row | [x] |
| C47 | `unfilter` | `len == bpp` (`w == 1`) — row-0 loops don't run; row-`y` runs only the prologue | [x] |
| C48 | `unfilter` | `bpp > len`. For `w >= 1`, `len = w*bpp >= bpp`, so this is only reachable as `len == 0` (`w == 0`); tested together with the neighbouring `bpp > w` shapes and `bpp ∈ {1,3,5,8,16,33}` | [x] |
| C49 | `unfilter` | payload all `0x00`, all `0xFF`, alternating `0x00/0xFF`, and random — covers `wrapping_add` carry and all three `cp_paeth` outcomes | [x] |
| C50 | `unfilter` | large image (`w=257`, `h=129`, `bpp=4`) with random filters and data, single call | [x] |

## Notes on behaviour these rows pin down

### `cp_ptr` mis-accounts for the trailing partial word (rows C1-C5)

`cp_stored` locates the uncompressed payload with

```c
static char *cp_ptr(cp_state_t *s) {
  return (char *)(s->words + s->word_index) - (s->count / 8);
}
```

which assumes every buffered bit arrived via a full 32-bit `s->words[]` load. When
`cp_peak_bits` has folded in the trailing partial word instead, `s->count` grew by
`s->bits_left` rather than by 32, and the pointer lands *short* of the payload —
so `memcpy` copies bytes of the `LEN`/`NLEN` header (and whatever follows the
input) instead of the stored data. For a 1-byte stored block at 4-byte alignment
the offset is `in + 2` where the payload is at `in + 5`.

This is what the C does, so the tests assert the bytes `cp_ptr` actually selects,
reproduced by a model of the header's `[1, 2, count & 7, 16, 16]` read sequence
(`stored_memcpy_offset` in `tests/phase_b_inflate.rs`), rather than the bytes a
correct implementation would have copied. Each row also asserts it reaches the
offsets where `cp_ptr` *is* correct, so the ordinary path is covered too.

### `cp_stored`'s `memcpy` is unbounded in both directions

`LEN` is a `uint16_t` and neither the source nor the destination is range-checked
(`s->out_end` is not consulted at all). A stored block can therefore read up to
65535 bytes past `in_bytes` and write up to 65535 bytes past `out_bytes`. The
harness pads both buffers by more than that (`OVERRUN_PAD`) so the over-read hits
deterministic zeros in *both* libraries, and compares the padding as well as the
requested output so an over-write is caught rather than ignored.

### `unfilter` with `len == 0` rewrites its own filter bytes (rows C46-C48)

When `w * bpp == 0` the row stride is 1, so row `y`'s `for (x = 0; x < bpp; x++)
raw[x] += prev[x]` prologue writes into row `y+1`'s *filter byte*. Filter values
therefore mutate as the loop proceeds and a run of valid rows can turn itself into
an invalid one, making `unfilter` return 0 for input that started out entirely
valid. The tests only require success where `len >= 1`; for `len == 0` the
differential comparison is the assertion.
