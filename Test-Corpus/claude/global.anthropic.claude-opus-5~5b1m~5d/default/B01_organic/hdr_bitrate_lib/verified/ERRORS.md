# ERRORS.md — Phase A: error-surface table

## Mechanical derivation

Grep of the entire library (`c_src/src/lib.c`, `c_src/include/lib.h`) for every
rejection mechanism:

```
$ grep -nE 'return|assert|NULL|-1|if|switch|#if|error|ERROR|max|min|MAX|MIN' \
      c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:12:    return 2 *
```

Result of the grep, itemised:

| mechanism searched for | occurrences |
|------------------------|------------:|
| `RETURN_ERROR` / error macro | 0 |
| `return -1` / `return 0` sentinel / early return | 0 (one single unconditional `return`) |
| `return NULL` | 0 (return type is `unsigned`) |
| error enum / status code | 0 |
| `assert` / `static_assert` | 0 |
| explicit range check (`if`, `switch`, `? :`, `&&`, `||`) | 0 |
| null-pointer check | 0 |
| min/max constant | 0 |
| `#if` / `#ifdef` compile-time branch | 0 |

**The C library has no error surface at all.** `hdr_bitrate` is a single
unconditional expression: it validates nothing, has no sentinel value, and
cannot fail by its own logic. Every input either produces a bitrate or produces
undefined behaviour in the C abstract machine.

That makes the error surface entirely *implicit*: the rows below enumerate every
input for which the C code performs an operation it does not guard, i.e. every
distinct out-of-bounds table access reachable from the input, plus the generic
FFI boundary conditions required by Phase C. For each, "expected C result" is
what the **compiled C `.so` actually returns** — that is the ground truth the
Rust must reproduce bit-for-bit, per the task rules (replicate, never "fix").

## Index algebra (needed to derive the rows)

```c
static const uint8_t halfrate[2][3][15];              /* 90 bytes, contiguous */
return 2 * halfrate[ !!(h[1] & 0x8) ]                 /* plane: 0..1   (safe)  */
                   [ ((h[1] >> 1) & 3) - 1 ]          /* layer: -1..2  (UNGUARDED) */
                   [ h[2] >> 4 ];                     /* rate:  0..15  (UNGUARDED) */
```

Flat byte offset from `&halfrate[0][0][0]`:

```
offset = plane*45 + layer*15 + rate      with plane in 0..1, layer in -1..2, rate in 0..15
       => offset in -15 ..= 90           (the object itself only covers 0 ..= 89)
```

Two independent unguarded fields therefore exist:

* `layer == -1` — the MPEG "reserved layer" encoding `h[1] & 0x6 == 0`. Steps the
  index back one whole row (`-15` bytes).
* `rate == 15` — the MPEG "bad bitrate" encoding `h[2] >> 4 == 0xF`. Steps one
  element past the end of the row.

Most (`layer`, `rate`) combinations that leave a *row* still land inside the
90-byte object, so they are in-bounds for the C object even though they are
out-of-bounds for the declared inner dimension. Exactly **16** input classes
escape the object entirely. Verified against the linked image
(`objdump`: table is `halfrate.0` at vaddr `0x2000`; `od -j 0x1ff0`):

* offsets `-15 ..= -1` -> vaddrs `0x1ff1..0x1fff`, page padding of the preceding
  `R E` LOAD segment (file size `0x191`, page-rounded) — all `0x00`.
* offset `90` -> vaddr `0x205a`, the 2 bytes of alignment padding between
  `.rodata` (size `0x5a`) and `.eh_frame_hdr` (vaddr `0x205c`, align 4) — `0x00`.

So every escaping read yields `0`, hence a returned bitrate of `0`. The Rust
translation models this with an explicitly zero-padded flat table, which is why
it agrees. Each row below is nonetheless asserted differentially against the C
`.so` rather than against this analysis.

## Error-surface table

`h[1]` / `h[2]` columns give the bit patterns that trigger the row; `x` = don't
care. Legend for "layer bits" = `(h[1] >> 1) & 3`, "rate nibble" = `h[2] >> 4`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `hdr_bitrate` | Unguarded negative inner index: layer bits `= 00` (reserved layer) with plane `= 0` (`h[1] & 0x8 == 0`) and rate nibble `0`, i.e. flat offset `-15`. Read *before* the whole `halfrate` object. | `0` | [x] |
| 2 | `hdr_bitrate` | Same, rate nibble `1..14` — flat offsets `-14 ..= -1`, all *before* the object (14 distinct offsets, all enumerated). | `0` for every one | [x] |
| 3 | `hdr_bitrate` | Unguarded index one past the object end: plane `= 1`, layer bits `= 11` (layer index `2`), rate nibble `= 15` — flat offset `90`, one byte past `halfrate`. | `0` | [x] |
| 4 | `hdr_bitrate` | layer bits `= 00` with plane `= 0` and rate nibble `= 15`: offset `-15 + 15 = 0` — index arithmetic cancels back into the object at `halfrate[0][0][0]`. | `0` (= `2 * 0`) | [x] |
| 5 | `hdr_bitrate` | layer bits `= 00` with plane `= 1` (offsets `30..44`): reads `halfrate[0][2][rate]`, i.e. the *wrong plane's* last row. Not a crash but a silently wrong table row the C does not reject. | `2 * halfrate[0][2][rate]` (`0,32,48,…,256`) | [x] |
| 6 | `hdr_bitrate` | layer bits `= 00`, plane `= 1`, rate nibble `= 15`: offset `45` — reads `halfrate[1][0][0]`. | `0` | [x] |
| 7 | `hdr_bitrate` | rate nibble `= 15` ("bad" bitrate) for plane `0`, layer bits `01`/`10`/`11` — offsets `15`, `30`, `45`: each lands on element `0` of the following row. | `0` for all three | [x] |
| 8 | `hdr_bitrate` | rate nibble `= 15` for plane `1`, layer bits `01`/`10` — offsets `60`, `75`: element `0` of the following row. | `0` for both | [x] |
| 9 | `hdr_bitrate` | rate nibble `= 0` (the "free-format" bitrate encoding) for any plane/layer: the C returns the table's `0` entry rather than signalling. | `0` for all 8 plane×layer combos | [x] |
| 10 | `hdr_bitrate` | Highest in-range value: plane `1`, layer bits `11`, rate nibble `14` — `2 * 224 = 448`, which does **not** fit in `uint8_t`; verifies the `unsigned` (not truncated) return width across the FFI boundary. | `448` | [x] |
| 11 | `hdr_bitrate` | **Out-of-range "enum" values across the FFI boundary.** The library has no `enum` parameter; every one of the 8 discrete fields it decodes (`plane`, `layer`, `rate`) is fed by raw bits, so *every* bit pattern is already a reachable "out-of-range variant". Exhaustively: all 256 x 256 = 65536 `(h[1], h[2])` pairs — including all 4 layer encodings (one of which, `00`, has no valid variant) and all 16 rate encodings (`0` and `15` have no valid variant). | identical value from C and Rust for all 65536 | [x] |
| 12 | `hdr_bitrate` | Ignored bits must stay ignored: `h[1]` bits `0` and `4..7`, and `h[2]` bits `0..3`, are never read. Flipping them must not change the result (subsumed by the exhaustive sweep of row 11 for `h[1]`/`h[2]`). | result depends only on `h[1] & 0xE`, `h[2] & 0xF0` | [x] |
| 13 | `hdr_bitrate` | Bytes outside `h[1..=2]` must not be read: `h[0]` and `h[3..]` are irrelevant. Verified by placing the 3-byte window at the very **end** of a `mmap`ed page with the next page unmapped — reading `h[3]` would `SIGSEGV`. Both libraries must return normally. | no read past `h[2]`; both return the same value | [x] |
| 14 | `hdr_bitrate` | `h` == `NULL`. The C does not null-check; it dereferences `NULL+1`. | `SIGSEGV` in *both* libraries (asserted in a forked child: same fatal signal, no silent divergence) | [x] |
| 15 | `hdr_bitrate` | Unaligned / arbitrary pointer position. `uint8_t` loads have no alignment requirement, so every byte offset within a buffer must behave identically. | identical for every alignment 0..63 | [x] |
| 16 | `hdr_bitrate` | "Zero and oversized lengths": `hdr_bitrate` takes no length argument, so the only analogue is buffer extent. A buffer of exactly 3 bytes (minimum sufficient) and a huge buffer must give identical results. | identical | [x] |

Rows 1–16 = the complete rejection/undefined-input surface. Every row is
covered by a test in `tests/error_path.rs`; the `[x]` marks are set only after
that test passed against **both** `.so`s.

## Robustness of the out-of-bounds rows

Rows 1–3 depend on what the *linked image* holds outside the 90-byte
`halfrate` object, which is in principle build-dependent. That was checked
rather than assumed: the whole 65536-entry `(h[1], h[2])` domain was dumped from
the shared objects through `dlopen`/`dlsym` and compared line by line:

| build | vs Rust `.so` |
|-------|---------------|
| C, `CMakeLists.txt` default (no `-O`) | **identical, 65536/65536** |
| C, `-DCMAKE_BUILD_TYPE=Release -DCMAKE_C_FLAGS=-O2` | **identical, 65536/65536** |

So the zero-padding model in `src/lib.rs` reproduces the C at both optimization
levels, not only the one the default CMake configuration produces. (The
optimized build was made in a scratch directory; `c_src/` is never modified.)
