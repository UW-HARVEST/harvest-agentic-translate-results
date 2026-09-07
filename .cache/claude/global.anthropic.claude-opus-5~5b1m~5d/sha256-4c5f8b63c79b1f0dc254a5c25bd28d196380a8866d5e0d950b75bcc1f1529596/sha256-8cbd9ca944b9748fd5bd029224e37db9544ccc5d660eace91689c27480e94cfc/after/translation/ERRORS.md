# ERRORS.md — Phase C error-surface table

## Mechanical grep of the C source for rejection paths

```
$ grep -nE 'RETURN_ERROR|return +-|return +NULL|assert|errno|ERR|_MAX|_MIN|if *\(|switch' c_src/src/lib.c
24:    if (m->pos >= 64) {
50:    return b;                 # <- plain value, never a sentinel
$ grep -c 'assert' c_src/src/lib.c        -> 0
$ grep -c 'return NULL' c_src/src/lib.c   -> 0
$ grep -c 'return -' c_src/src/lib.c      -> 0
$ grep -nE 'enum|#ifdef|#if ' c_src/src/lib.c c_src/include/lib.h -> (none)
```

**Result: the C library has NO error codes, NO sentinels, NO asserts, NO enums,
NO null checks and NO explicit range validation.** Two of three functions
return `void`; `update_md5` returns an unconstrained `tflac_u32` byte counter
(never a sentinel). Every "invalid" input is therefore either (a) handled
silently by defined unsigned-wraparound / modulo arithmetic, which the Rust
MUST reproduce bit-exactly, or (b) undefined behaviour in C that no conforming
test can compare.

The table below is the exhaustive list of *distinct* rejection / boundary
conditions the C code actually contains, plus the generic C-API boundaries
required by Phase C.

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 1 | `tflac_md5_addsample` | the only branch in the library: `m->pos + bits/8 >= 64` after the add | takes the fold-down branch: `pos %= 64`, then copies `bytes = pos` bytes from `buffer[64+i]` down to `buffer[i]`. No error. | [x] |
| 2 | `tflac_md5_addsample` | `m->pos + bits/8 < 64` (branch NOT taken) | `pos` left as the raw sum, buffer tail untouched. No error. | [x] |
| 3 | `tflac_md5_addsample` | fold-down with `pos % 64 == 0` — `while (bytes--)` with `bytes == 0` | loop body never executes; the post-decrement wrap of the unsigned counter to `0xFFFFFFFF` is *not* observable (no out-of-range buffer access). No error. | [x] |
| 4 | `tflac_md5_addsample` | `bits == 0` (zero length) | `total += 0`, `bytes = 0`, still writes 8 bytes of `val` at `buffer[pos%64]`, `pos` unchanged. No error. | [x] |
| 5 | `tflac_md5_addsample` | `bits` not a multiple of 8 (e.g. 1, 7, 9, 63) | integer-truncating `bits/8` silently discards the remainder; `total` still gains the full un-truncated `bits`. No error. | [x] |
| 6 | `tflac_md5_addsample` | `bits` oversized: `bits == 0xFFFFFFFF` (`bytes == 0x1FFFFFFF`) | `pos += 0x1FFFFFFF` (u32 wrap), `>= 64` branch taken, `pos %= 64`. No error. | [x] |
| 7 | `tflac_md5_addsample` | `m->pos` already out of the documented `[0,63]` range: `pos == 64`, `65`, `127`, `0xFFFFFFFF` | `pos2 = pos % 64` clamps the *write* offset to `[0,63]`; `pos += bytes` wraps mod 2^32. The fold-down then *reads* `buffer[64 + i]` for `i < pos % 64`, which for `pos % 64 > 8` goes **out of bounds** (up to `buffer[126]`) — see the note below; Rust performs the identical load. No error. | [x] |
| 8 | `tflac_md5_addsample` | `m->pos + bits/8` overflows `tflac_u32` (e.g. `pos = 0xFFFFFFFF`, `bits = 64`) | defined unsigned wraparound; result `0x00000007`, `>= 64` false, no fold-down. No error. | [x] |
| 9 | `tflac_md5_addsample` | `m->total` overflows `tflac_u64` (`total = 0xFFFF...FF`) | defined unsigned wraparound of `total += bits`. No error. | [x] |
| 10 | `tflac_pack_u64le` | boundary values `n == 0` and `n == UINT64_MAX` | writes 8 bytes `00..00` / `FF..FF`. No error. | [x] |
| 11 | `tflac_pack_u64le` | write offset at the very end of the 72-byte buffer (`d = &buffer[64]`) — the largest in-bounds offset the library itself can produce | writes `buffer[64..71]`, exactly filling the buffer. No error / no overrun. | [x] |
| 12 | `update_md5` | `cur_blocksize == 0` and/or `channels == 0` → `b == 0` before the loop | `b` underflows five times by `step == 8`: returns `0 - 40 == 0xFFFFFFD8`. NOT an error sentinel. | [x] |
| 13 | `update_md5` | `b < 40` (any `cur_blocksize*channels` in `1..39`) — partial underflow | returns `b - 40` mod 2^32. No error. | [x] |
| 14 | `update_md5` | `cur_blocksize * channels` itself overflows `tflac_u32` (e.g. `0x10000 * 0x10000`) | defined unsigned wraparound of the product, then `-40`. No error. | [x] |
| 15 | `update_md5` | samples containing negative values (`-1`, `INT32_MIN`) — sign-extension on the `(tflac_uint)` cast | each sample is sign-extended to 64 bits then masked `& 0xFF`, so only the low byte survives (`-1` → `0xFF`, `INT32_MIN` → `0x00`). No error. | [x] |
| 16 | `update_md5` | `update_md5` reads `samples[0..7]` at stride `8*sizeof(tflac_s32) == 32` **elements** for 5 iterations → highest index read is `4*32+7 == 135`, i.e. it requires **≥136** samples | reads 136 elements; supplying fewer is C UB (out-of-range read). Tested with an exactly-136-element buffer and with guard padding to confirm no read past index 135. | [x] |
| 17 | all three | `NULL` pointer for `d` / `m` / `t` / `samples` | **Undefined behaviour in C** — the code dereferences unconditionally with no null check. Compared in a forked child: both the C `.so` and the release Rust `.so` die with the **same** fatal signal (`SIGSEGV`, 11) for all four null cases. (An *unoptimised* Rust cdylib has `debug_assertions` on and aborts with `SIGABRT` instead — debug instrumentation, not an ABI difference; the test asserts exact signal equality only for the optimised artifact.) | [x] |
| 18 | all three | out-of-range *enum* values across the FFI boundary | **N/A — the library declares no enums** (`grep -c 'enum' == 0`). The nearest analogue is the unconstrained `tflac_u32 bits` parameter, covered by rows 4–6. | [x] (N/A) |

## Additional finding: the fold-down loop reads OUT OF BOUNDS

`tflac_md5_addsample`'s fold-down is

```c
if (m->pos >= 64) { m->pos %= 64; bytes = m->pos;
                    while (bytes--) m->buffer[bytes] = m->buffer[64 + bytes]; }
```

`bytes = m->pos % 64` can be up to **63**, so the load can reach
`m->buffer[64 + 62] == m->buffer[126]` — 55 bytes past the 72-byte array, i.e.
past the end of `struct tflac_md5` itself.

* In the library's own usage `pos` only ever grows by `bits/8 == 8` from a value
  `< 64`, so `pos % 64 <= 7` and the highest load is `buffer[70]` — in bounds.
* A caller that passes `pos > 64` or a `bits` whose `bits/8` is not 8 **does**
  drive the out-of-bounds load. The Rust translation performs the *identical*
  pointer arithmetic (`*buffer.add(bytes) = *buffer.add(64 + bytes)`), so it
  reads the same address and produces the same result.

This was initially reported as a divergence by the test harness only because
each implementation was handed a *separately allocated* 88-byte context, so the
two out-of-bounds loads picked up different adjacent stack bytes. The harness
now embeds every context in a 192-byte deterministically pre-filled guard
region (`tests/common/mod.rs`, `GUARD`), which makes the C's out-of-bounds read
reproducible and *also* proves neither implementation ever **writes** out of
bounds (`guard_intact()` is asserted after every call). With that, all
`ERRORS.md` rows pass.

## Test mapping

| ERRORS.md row | test in `tests/phase_c_errors.rs` |
|---|---|
| 1 | `err01_folddown_branch_taken` |
| 2 | `err02_folddown_branch_not_taken` |
| 3 | `err03_folddown_zero_length_copy` |
| 4 | `err04_bits_zero` |
| 5 | `err05_bits_not_multiple_of_8` |
| 6 | `err06_bits_oversized` |
| 7 | `err07_pos_out_of_range` (incl. exhaustive `pos` sweep `0..=256`) |
| 8 | `err08_pos_add_overflows_u32` |
| 9 | `err09_total_overflows_u64` |
| 10 | `err10_pack_boundary_values` |
| 11 | `err11_pack_at_buffer_end` (+ explicit 32-byte guard assertion) |
| 12 | `err12_update_b_zero_returns_wrapped` |
| 13 | `err13_update_partial_underflow` |
| 14 | `err14_update_product_overflow` |
| 15 | `err15_update_negative_samples_sign_extension` |
| 16 | `err16_update_reads_exactly_136_samples` |
| 17 | `err17_null_pointers_behave_identically` (fork + signal comparison) |
| 18 | `err18_no_enums_bits_is_the_unconstrained_int_parameter` |
| generic boundaries | `errgen_exhaustive_pos_times_bytecount_grid`, `errgen_update_md5_full_field_grid` |

**Status: 20/20 error-path tests pass** against the C `.so` at `-O0` and `-O2`,
with the Rust `.so` built both `--release` and debug.
