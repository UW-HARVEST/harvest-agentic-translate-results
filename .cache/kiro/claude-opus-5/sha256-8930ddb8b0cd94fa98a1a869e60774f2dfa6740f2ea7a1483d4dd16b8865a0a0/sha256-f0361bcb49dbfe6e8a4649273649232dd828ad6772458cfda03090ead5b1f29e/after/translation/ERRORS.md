# ERRORS.md — Error-surface table (Phase A, gates Phase C)

## Mechanical derivation

Every rejection/error site in the C source was enumerated by grepping
`c_src/src/lib.c` and `c_src/include/lib.h` for:

`return` · `assert` · `NULL` · `errno` · `-1` · `ERROR` · `if (` · `switch` ·
`#if` / `#ifdef` · `MAX` · `MIN` · `goto`

The **only** match in the whole library is:

```
c_src/src/lib.c:19:    return crc16;
```

which is the single success return. Consequently:

- there is **no** error-return macro, no error enum, no sentinel value;
- there are **no** `assert`s;
- there are **no** explicit range checks, no null checks, no min/max constants;
- there are **no** `if` / `switch` / preprocessor branches at all — the only
  control flow is the two loop conditions `while (len >= 8)` and `while (len--)`;
- the return type `tflac_u16` has no reserved out-of-band value: **every** one of
  the 65536 possible return values is a legal CRC result, so "failure" is not
  representable.

`crc16` is therefore a **total function over its declared domain**: it cannot
reject input. The error-surface table below has **zero rejection rows**.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | *(none — the C code contains no rejection site)* | — |

## Boundary / undefined-behaviour rows (tested anyway in Phase C)

These are not C rejections — the C code performs no check for them — but they
are the generic boundaries every C API has, so the Rust must behave identically
where the behaviour is observable. Each row has a differential test.

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| B1 | `crc16` | `d == NULL`, `len == 0` | returns `crc` unchanged; `d` never dereferenced (both loops fall through) | [x] |
| B2 | `crc16` | `len == 0`, valid non-null `d` | returns `crc` unchanged, byte-identical | [x] |
| B3 | `crc16` | `len == 1` (tail loop only, one step past the empty case) | single tail iteration | [x] |
| B4 | `crc16` | `len == 7` (one step below the bulk-loop threshold `len >= 8`) | 0 bulk iterations, 7 tail iterations | [x] |
| B5 | `crc16` | `len == 8` (exactly at the bulk-loop threshold) | 1 bulk iteration, 0 tail iterations | [x] |
| B6 | `crc16` | `len == 9` (one step past the threshold) | 1 bulk iteration, 1 tail iteration | [x] |
| B7 | `crc16` | `crc == 0x0000` (minimum of the seed range) | normal computation | [x] |
| B8 | `crc16` | `crc == 0xFFFF` (maximum of the seed range; both table indices `crc>>8` and `crc&0xFF` = 255, i.e. last table slot) | normal computation, no index overflow | [x] |
| B9 | `crc16` | all data bytes `0x00` (minimum table index) | index 0 of every table | [x] |
| B10 | `crc16` | all data bytes `0xFF` (maximum table index — one step past would be out of range) | index 255 of every table | [x] |
| B11 | `crc16` | data spans **all 256 byte values** at every one of the 8 bulk-loop lanes → every `tflac_crc16_tables[t][i]` slot reachable | no out-of-range table index | [x] |
| B12 | `crc16` | `len` "oversized" relative to the buffer is UB in C and cannot be tested safely; instead `len` is driven to large in-buffer values (1 MiB) and to values whose low 3 bits cover 0..7 | identical results | [x] |
| B13 | `crc16` | unaligned `d` (odd byte offset into the buffer) — the C reads byte-wise so alignment is irrelevant, but the Rust must not assume alignment | identical results | [x] |
| B14 | `crc16` | out-of-range enum value across the FFI boundary | **N/A — the API has no enum parameter.** All three parameters (`const uint8_t*`, `uint32_t`, `uint16_t`) accept their full bit-pattern range with no invalid encodings; every `u16` seed and every `u8` byte is a valid input, and all of them are covered by B7–B11 and the randomized Phase B rows. | [x] |

All rows above are covered by `translation/tests/differential.rs`
(`phase_c_*` tests) and pass against both `.so`s.
