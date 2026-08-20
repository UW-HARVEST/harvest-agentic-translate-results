# CONFIGS.md — Configuration-surface table (Phase A → gates Phase B)

Mechanically derived from `c_src/include/lib.h` + `c_src/src/lib.c`.

## Public entry points (complete set)

`c_src/include/lib.h` declares exactly **one** external function, and
`nm -D --defined-only` on the C `.so` confirms `bin2hex` is the only defined
export. There is no convenience/one-shot wrapper layered on a lower-level API —
`bin2hex` *is* the lowest-level entry point, so every row below drives it
directly.

```c
char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len);
```

## Axes the C code actually branches on

There are no runtime option/mode/flag parameters, no global state, no
`#ifdef`/`#if` in `lib.c` or `lib.h`, and no `switch`. The branch surface is
therefore entirely driven by the four arguments:

| axis | source of the branch | values that are treated differently |
|------|----------------------|--------------------------------------|
| A1 `bin_len` | `while (i < bin_len)` loop trip count | `0` (loop body never runs), `1`, `2`, odd vs even, `255/256/257`, `1024/4096/65536` |
| A2 `hex_maxlen` slack | `hex_maxlen <= bin_len * 2U` | exactly `2*bin_len+1` (minimum accepted), `+slack`, `SIZE_MAX` |
| A3 low nibble `c = bin[i] & 0xf` | `(((c - 10U) >> 8) & ~38U)` → non-zero correction iff `c < 10` | `c < 10` (digit `'0'..'9'`) vs `c >= 10` (letter `'a'..'f'`) |
| A4 high nibble `b = bin[i] >> 4` | same expression on `b` | `b < 10` vs `b >= 10` |
| A5 byte value boundaries | edges of A3/A4 | `0x00 09 0A 0F 90 99 9A 9F A0 A9 AA AF F0 F9 FA FF`, plus the full `0x00..0xFF` sweep |
| A6 `bin` pointer | byte-at-a-time reads, no alignment requirement | aligned start vs offsets `1..8` into an allocation; `NULL` when `bin_len == 0` |
| A7 `hex` pointer / buffer pre-state | `hex[i*2]`, `hex[i*2+1]`, `hex[i*2] = 0` — writes exactly `2*bin_len+1` bytes | zero-filled vs `0xAA` sentinel-filled buffer; `hex` at a non-zero offset inside a larger allocation (checks no write before `hex` or after `hex[2*bin_len]`) |
| A8 result | `return hex;` | returned pointer must be identical to the `hex` argument |
| A9 statelessness | function has no `static`/global state | repeated and C↔Rust-interleaved calls on the same buffer |

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is exercised through **both** `.so`s via `libloading` and compared
byte-for-byte over the whole output buffer (not just the NUL-terminated prefix),
plus the returned pointer. Randomized rows use a fixed-seed SplitMix64 PRNG so
runs are reproducible.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `bin2hex` | A1=`0`, A2=exact min (`hex_maxlen=1`), A7=sentinel buffer → only `hex[0]=0` written | [x] |
| C2 | `bin2hex` | A1=`0`, A2=generous, A6=`bin == NULL` (legal: never dereferenced) | [x] |
| C3 | `bin2hex` | A1=`1`, A2=exact min (`3`), A5=**exhaustive** all 256 byte values (covers A3×A4 fully) | [x] |
| C4 | `bin2hex` | A1=`1`, A2=generous slack, A7=sentinel, randomized byte | [x] |
| C5 | `bin2hex` | A1=`2`, A2=exact min (`5`), **exhaustive** all 65 536 two-byte inputs | [x] |
| C6 | `bin2hex` | A1=`3` (odd), A2=min+1, A7=sentinel, randomized | [x] |
| C7 | `bin2hex` | A3/A4 combo `b<10, c<10` (bytes `0x00..0x99` with both nibbles ≤ 9), randomized within class | [x] |
| C8 | `bin2hex` | A3/A4 combo `b<10, c>=10` (high nibble ≤ 9, low nibble ≥ 0xA), randomized within class | [x] |
| C9 | `bin2hex` | A3/A4 combo `b>=10, c<10`, randomized within class | [x] |
| C10 | `bin2hex` | A3/A4 combo `b>=10, c>=10`, randomized within class | [x] |
| C11 | `bin2hex` | A5 boundary-byte set (`00 09 0A 0F 90 99 9A 9F A0 A9 AA AF F0 F9 FA FF`) as one buffer, and each byte alone at A1=`1` | [x] |
| C12 | `bin2hex` | A1=random `1..64`, A2=exact min, 2 000 randomized trials | [x] |
| C13 | `bin2hex` | A1=random `1..64`, A2=min + random slack `1..64`, A7=sentinel, 2 000 randomized trials | [x] |
| C14 | `bin2hex` | A1 ∈ {`254`,`255`,`256`,`257`,`258`} (byte/loop boundaries), randomized contents | [x] |
| C15 | `bin2hex` | A1 ∈ {`1024`,`4096`,`65536`} (large), randomized contents, A2=exact min | [x] |
| C16 | `bin2hex` | A2=`SIZE_MAX` (oversized but accepted) with small `bin_len` | [x] |
| C17 | `bin2hex` | A6=`bin` at offsets `1..8` inside an allocation (unaligned source), randomized | [x] |
| C18 | `bin2hex` | A7=`hex` at offset `1..8` inside a larger sentinel buffer; assert bytes before `hex` and after `hex[2*bin_len]` are untouched | [x] |
| C19 | `bin2hex` | A8 return-pointer identity, checked in every row (`ret == hex` for C and Rust) | [x] |
| C20 | `bin2hex` | A9 statelessness: 100 repeated calls on the same buffer, and C→Rust→C interleaving into one shared buffer | [x] |
| C21 | `bin2hex` | value extremes: all-`0x00` and all-`0xFF` inputs at A1 ∈ {1,2,7,8,63,64} | [x] |
| C23 | `bin2hex` | concurrent use from 8 threads on private buffers (the C function keeps no state, so results must be identical to the single-threaded ones) | [x] |
| C22 | `bin2hex` | aliasing input shape: `hex` and `bin` inside the **same** allocation (`hex == bin`, `hex == bin+1`, `hex == bin-k`) — the C reads `bin[i]` *after* earlier iterations may have overwritten it, so the per-iteration read/write order is observable | [x] |
