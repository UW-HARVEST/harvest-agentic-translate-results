# ERRORS.md — Error-surface table (Phase C gate)

## Derivation method

Mechanically grepped every rejection/error construct in `c_src`:

```
grep -nE 'return|assert|NULL|errno|ERROR|if\s*\(|else|switch|case|goto|exit|abort|MAX|MIN|#if' c_src/src/lib.c
grep -nE 'if|assert|return|NULL|#if|switch|enum|define'  c_src/include/lib.h   # (table data excluded)
```

Result — the **only** matches in `c_src/src/lib.c` are the two loop conditions
(`while (len >= 8)`, `while (len--)`), the table lookups, and the single
`return crc16;`. The header contributes no branches at all.

Therefore the C library has:

* **no** error-return macros (`RETURN_ERROR` and friends): none exist
* **no** `return -1` / `return NULL` / error enums: `crc16` returns
  `tflac_u16` and has exactly one `return` statement, on the success path
* **no** `assert()` of any kind
* **no** explicit range checks, no `NULL` checks, no min/max constants
* **no** enum parameters at all, hence no out-of-range-enum surface
* **no** allocation, no I/O, no global state, hence no failure modes there

`crc16` is a **total function** over its documented domain: for every
`(d, len, crc)` where `d` points to `len` readable bytes, it returns a value and
cannot fail. There is consequently no error code or sentinel to compare.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `crc16` | `len == 0`, `d` = valid non-null pointer. `while (len >= 8)` is false; `while (len--)` evaluates `0` → false, so `d` is never dereferenced. | returns the seed `crc` unchanged; no read of `*d` | `e1_len_zero_valid_ptr_returns_seed` | [x] |
| E2 | `crc16` | `len == 0`, `d == NULL`. Same as E1 — the null pointer is never dereferenced because both loop guards fail before any access. | returns the seed `crc` unchanged; **no** crash | `e2_len_zero_null_ptr_returns_seed` | [x] |
| E3 | `crc16` | `len` deliberately **smaller** than the real buffer (caller under-reports length). C reads exactly `len` bytes and ignores the rest. | result depends only on the first `len` bytes; trailing bytes must not affect it | `e3_len_shorter_than_buffer_ignores_tail` | [x] |
| E4 | `crc16` | `crc` seed at the extremes of `tflac_u16` (`0x0000`, `0xFFFF`) — probes the un-masked `crc16 >> 8` / `crc16 & 0xFF` table indices and the `crc16 << 8` truncation in the tail loop. `0xFFFF >> 8 == 0xFF` is the last valid table index; one more would be out of bounds. | well-defined `tflac_u16`; index never exceeds 255 | `e4_seed_boundary_values` | [x] |
| E5 | `crc16` | Byte values at the extremes (`0x00`, `0xFF`) used as table indices in both the slice-by-8 body (`d[2]..d[7]`) and the tail (`(crc16>>8) ^ *d++`). `0xFF ^ 0xFF` and `0x00 ^ 0xFF` are the boundary indices of `tflac_crc16_tables[0]`. | well-defined `tflac_u16`; no out-of-bounds index | `e5_byte_boundary_values` | [x] |
| E6 | `crc16` | `len` one step past each loop-structure boundary: `7` (tail only, `len >= 8` never true), `8` (exactly one block, tail loop runs zero times), `9` (one block + 1 tail byte). Off-by-one in the block/tail split. | block/tail split identical to C | `e6_len_off_by_one_boundaries` | [x] |
| E7 | `crc16` | `len` values whose `int` promotion could differ: `len` with the high bit set is impossible to allocate, but `len` is `tflac_u32` and `len -= 8` / `len--` are unsigned. `len == 0` is the wrap boundary of `while (len--)` (it decrements to `0xFFFFFFFF` but exits). | must exit, not loop 4G times | `e7_len_zero_no_underflow_loop` | [x] |
| E8 | `crc16` | Oversized-but-real `len` (large buffer, 1 MiB) to confirm no 16-bit/32-bit counter truncation in the `len -= 8` accumulation. | identical CRC over the whole buffer | `e8_large_length_no_counter_truncation` | [x] |

### Explicitly out of scope (undefined behaviour in C, not an error surface)

`crc16(NULL, len>0, crc)` and `crc16(short_buf, len > buflen, crc)` dereference
invalid memory. That is UB in the C — the C library performs no check, so there
is no defined result to compare against and no "same error" to assert. These are
not rows in the table; E2 covers the only null-pointer case the C actually
defines (`len == 0`). E3 covers the safe direction of a length mismatch.
