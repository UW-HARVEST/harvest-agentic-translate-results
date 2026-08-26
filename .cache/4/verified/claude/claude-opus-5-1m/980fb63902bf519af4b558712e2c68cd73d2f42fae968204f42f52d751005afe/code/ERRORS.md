# ERRORS.md — Error / rejection surface of `c_src/src/lib.c`

## Mechanical derivation

Every rejection-shaped construct was grepped for in the *entire* C source
(`c_src/src/lib.c`, `c_src/include/lib.h`):

```
grep -nE "return|assert|NULL|errno|ERROR|error|RETURN_ERROR|goto|exit|
          if *\(|while *\(|switch|#if|#ifdef|MAX|MIN|<|>" c_src/src/lib.c c_src/include/lib.h
```

Result:

| construct | count in C source | detail |
|-----------|-------------------|--------|
| `return` statements | **1** | `lib.c:50` — `return b;` (a *value*, never a sentinel/error code) |
| `assert` / `NDEBUG` | 0 | `<assert.h>` is not included |
| `NULL` / null checks | 0 | no pointer is ever validated |
| `errno`, `RETURN_ERROR`, error enums, error typedefs | 0 | none exist |
| `goto` / `exit` / `abort` | 0 | none |
| `if` statements | **1** | `lib.c:24` — `if (m->pos >= 64)` (a *state* branch, not a rejection) |
| loops | **2** | `lib.c:27` `while (bytes--)`, `lib.c:37` `for (int i = 0; i <= 4; i++)` |
| `switch` / preprocessor conditionals | 0 | none |
| min/max constants, range checks | 0 | the only literal bound is the modulus/threshold `64` |

**Conclusion: the library has no error-reporting surface.** There is no error
code, no sentinel return, no `NULL` return, no validation and no assertion.
`update_md5` returns a *computed count*, `tflac_pack_u64le` and
`tflac_md5_addsample` return `void`. Consequently there is no "same error code"
to compare — the observable contract on invalid input is **the exact byte-level
side effects and the exact returned `tflac_u32`**, and that is what every row
below asserts.

Rows below therefore enumerate, one row per distinct thing the C *can* be
handed that a caller would consider degenerate/invalid or out-of-range, plus the
generic FFI boundaries the prompt mandates (null pointers, zero/oversized
lengths, one-past-range values, out-of-range enum values). "expected C result"
is what the *compiled C at `-O0`* actually does, established by running it.

Legend for the checkbox column: ✅ = a differential test exists, constructs that
exact condition, calls both `.so`s and asserts identical
return value **and** identical post-state of the whole shared byte arena.

## ERROR-SURFACE TABLE

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|---------------------------------------------|-------------------|------|----|
| E1 | `tflac_pack_u64le` | `d == NULL` | dereferences null ⇒ `SIGSEGV`. Not a recoverable error; Rust must fault identically (both are a raw `*d = ..` store). Verified out-of-process. | `err_e1_e2_e12_null_pointers_crash_identically` | ✅ |
| E2 | `tflac_md5_addsample` | `m == NULL` | `m->total += bits` dereferences null ⇒ `SIGSEGV`, same for Rust. Verified out-of-process. | `err_e1_e2_e12_null_pointers_crash_identically` | ✅ |
| E3 | `tflac_md5_addsample` | `bits == 0` (zero "length") | `total += 0`; `bytes = 0`; 8 bytes still written at `buffer[pos%64]`; `pos` unchanged; `if (pos>=64)` false for `pos<64` ⇒ no copy loop. Returns void. **No rejection.** | `err_e3_bits_zero` | ✅ |
| E4 | `tflac_md5_addsample` | `bits` not a multiple of 8 (1..7, 9, 63, 65, …) — remainder silently discarded | `total += bits` (full value, *not* rounded); `bytes = bits/8` (truncating). The low bits are lost with no diagnostic. | `err_e4_bits_not_multiple_of_8` | ✅ |
| E5 | `tflac_md5_addsample` | `bits` "oversized": `bits = 0xFFFFFFFF` ⇒ `bytes = 0x1FFFFFFF` (536 870 911), far past the 72-byte buffer | `total += 0xFFFFFFFF`; `pos += 0x1FFFFFFF`; `pos >= 64` ⇒ `pos %= 64`; copy loop runs `pos` (<64) times. No bounds check, no error. | `err_e5_bits_max_u32` | ✅ |
| E6 | `tflac_md5_addsample` | `m->pos` already `>= 64` on entry (out of its documented 0..63 range) — one step past (`pos == 64`) and far past (`pos == 0xFFFF_FFFF`) | `pos2 = pos % 64` is used for the write, but the `pos += bytes` / `pos >= 64` test uses the *unreduced* `pos`. `pos == 64, bits == 64` ⇒ `pos2 = 0`, `pos = 72`, `pos %= 64 = 8`, copy 8 bytes from `buffer[64..72]`. | `err_e6_pos_out_of_range` | ✅ |
| E7 | `tflac_md5_addsample` | `m->pos + bits/8` overflows `tflac_u32` (e.g. `pos = 0xFFFF_FFFF`, `bits = 64` ⇒ `bytes = 8`, sum wraps to `7`) | unsigned wraparound: `pos` becomes `7`, `7 >= 64` is **false** ⇒ the copy loop is skipped entirely. | `err_e7_pos_wraparound` | ✅ |
| E8 | `tflac_md5_addsample` | `m->total + bits` overflows `tflac_u64` (`total = 0xFFFF_FFFF_FFFF_FFFF`, `bits = 64`) | unsigned wraparound: `total` becomes `63`. No saturation, no error. | `err_e8_total_wraparound` | ✅ |
| E9 | `tflac_md5_addsample` | `pos % 64` in `57..=63` ⇒ `tflac_pack_u64le` writes past the first 64 bytes, into the 8-byte tail `buffer[64..72]` (out of the "logical" 64-byte MD5 block) | writes land in the tail slack; still inside the 72-byte array, so no fault. Bytes at `buffer[64+k]` are clobbered *before* the copy loop reads them. | `err_e9_write_spills_into_tail` | ✅ |
| E10 | `tflac_md5_addsample` | copy loop reads **past the end of `buffer[72]`**: needs `pos%64 > 8` after reduction, e.g. `pos = 40, bits = 0xC0` ⇒ `bytes = 24`, `pos = 64`→`0`… (see test for exact triples). With `pos` reduced to 63 it reads `buffer[64+62] = buffer[126]`, 54 bytes past the array and 38 bytes past `sizeof(tflac_md5)` | out-of-bounds *read* of adjacent memory, copied into `buffer[0..pos)`. C emits a literal byte-by-byte load/store at `-O0`; Rust must reproduce the same bytes. Tested over a shared, deterministically-filled arena so the OOB source bytes are defined and comparable. | `err_e10_copy_loop_reads_past_buffer` | ✅ |
| E11 | `tflac_md5_addsample` | `bytes == 0` after `pos %= 64` (i.e. reduced `pos == 0`, e.g. `pos = 56, bits = 64`) ⇒ `while (bytes--)` must run **zero** times while leaving `bytes == 0xFFFF_FFFF` | loop body never executes (`0` is falsy); the post-decrement underflow is discarded because `bytes` is dead afterwards. Buffer's low bytes are **not** touched. | `err_e11_copy_loop_zero_iterations` | ✅ |
| E12 | `update_md5` | `t == NULL` | `t->cur_blocksize` dereferences null ⇒ `SIGSEGV`; same in Rust. Verified out-of-process. | `err_e1_e2_e12_null_pointers_crash_identically` | ✅ |
| E13 | `update_md5` | `samples == NULL` | `samples[0]` dereferences null ⇒ `SIGSEGV`; same in Rust. Verified out-of-process. | `err_e1_e2_e12_null_pointers_crash_identically` | ✅ |
| E14 | `update_md5` | `samples` buffer shorter than what the function reads. The loop reads elements `0..8`, `32..40`, `64..72`, `96..104`, `128..136` ⇒ it **requires ≥ 136 `tflac_s32`** (544 bytes) regardless of `cur_blocksize`/`channels`. Handing it a "correctly sized" `cur_blocksize*channels` buffer (e.g. `1*1`) is an out-of-bounds read the C never checks. | reads whatever follows the buffer; iteration count is a hard-coded 5 and is *not* derived from `b`. Tested over a shared oversized arena so the reads are defined and comparable. | `err_e14_samples_shorter_than_read_span` | ✅ |
| E15 | `update_md5` | `b = cur_blocksize * channels` underflows: `b < 40` (5 iterations × `step = 8`). Includes the "empty" case `cur_blocksize == 0` and/or `channels == 0` ⇒ `b == 0` ⇒ return value `0 - 40 = 0xFFFF_FFD8` | unsigned wraparound; a huge `tflac_u32` is returned instead of an error. Every `b` in `0..=39` and the exact boundary `b == 40` (returns `0`) are covered. | `err_e15_b_underflow` | ✅ |
| E16 | `update_md5` | `cur_blocksize * channels` overflows `tflac_u32` (e.g. `0x10000 * 0x10000`, `0xFFFF_FFFF * 3`) | unsigned wraparound in the multiply, *then* `-40`. No overflow diagnostic. | `err_e16_b_multiply_overflow` | ✅ |
| E17 | `update_md5` | `t->md5_ctx.pos` out of range on entry (`>= 64`, incl. `0xFFFF_FFFF`) — reaches E6/E7 through the public header's only entry point | as E6/E7, five times in a row. `pos = 0xFFFF_FFFF` ⇒ after the 1st iteration `pos = 7`, then 15, 23, 31, 39. | `err_e17_update_md5_pos_out_of_range` | ✅ |
| E18 | `update_md5` | `t->md5_ctx.total` near `u64::MAX` ⇒ wraps during the 5 × `+64` | `total` wraps mod 2⁶⁴. | `err_e18_update_md5_total_wraparound` | ✅ |
| E19 | *all three* | **out-of-range "enum" values across the FFI boundary.** The C API declares no `enum`; the only integer-typed *mode-like* parameter is `tflac_md5_addsample`'s `bits`. Every one of the 2³² `tflac_u32` bit patterns is accepted, including values with no sensible meaning (`0`, non-multiples of 8, `> 576` = the buffer's bit capacity, `u32::MAX`). Fuzzed over the full `u32` range. | no validation whatsoever: `bits` is only ever used as `total += bits` and `bytes = bits/8`. | `err_e19_bits_full_u32_range_fuzz` | ✅ |
| E20 | `tflac_pack_u64le` | misaligned / one-past-the-end destination `d` (`d = arena + 1`, and `d` such that `d[7]` is the arena's last byte) | writes exactly 8 bytes, byte-at-a-time, no alignment requirement, no bound check. | `err_e20_pack_misaligned_and_last_8_bytes` | ✅ |

## Notes on the null-pointer rows (E1, E2, E12, E13)

These are the only inputs for which the C library's response is a *signal*
rather than a value. They are still differentially tested, but out-of-process:
the test re-executes itself as a child (`fork`-free approach: a child `cargo`
test binary invocation guarded by an env var), passes a null pointer to first
the C symbol and then the Rust symbol, and asserts both children die with the
**same** signal (`SIGSEGV`, 11). This proves the Rust does not, e.g., panic with
a Rust index-out-of-bounds message (unwinding / `abort` with a different signal)
where the C faults.

## Status

**20 / 20 rows verified.** `cargo test --test phase_c_errors` →
`18 passed; 0 failed; 1 ignored` (the ignored test is the deliberately
subprocess-only `helper_null_deref` worker). Rows E1/E2/E12/E13 share one
driver test because they are all the same null-pointer condition. Verified
under every feature combination (there is exactly one) and under both the `dev`
and `release` Rust profiles — see `run_all.sh`.
