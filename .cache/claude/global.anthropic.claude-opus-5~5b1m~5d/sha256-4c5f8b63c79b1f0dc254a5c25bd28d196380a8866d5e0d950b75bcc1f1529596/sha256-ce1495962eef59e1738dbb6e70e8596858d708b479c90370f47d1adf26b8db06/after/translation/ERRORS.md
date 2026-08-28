# ERRORS.md — error/rejection surface table (Phase A, gate for Phase C)

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

Grep results for every classic rejection construct:

```
grep -nE 'return|assert|NULL|RETURN_ERROR|#ifdef|errno|-1' c_src/src/lib.c
20:    bytes = bits / 8;         (not an error construct)
50:    return b;                 (the single, always-taken return)
```

* `RETURN_ERROR` / error enums / error codes: **none**
* `return -1` / `return NULL` / sentinel returns: **none** (the only `return`
  is `return b;`, a plain value)
* `assert(...)`: **none**
* explicit range / null / bounds checks: **none**
* `#ifdef` gates: **none**
* min/max constants: only the literal `64` (block size), `0xFF` (byte mask),
  `4` (loop bound), `8` (bits→bytes divisor / element counts)
* enum parameters: **none** — so the "out-of-range enum value" class is
  covered by the out-of-range **integer** parameter rows (#5–#10): `bits` is a
  raw `tflac_u32` that the C never validates, and any `int` value is legal at
  the FFI boundary.

So the library has **no explicit error surface at all**: every function
unconditionally performs its work. "Rejection" therefore only exists as
(a) fatal memory faults on invalid pointers, and (b) silent
wraparound/truncation/aliasing on out-of-range integers. Each such distinct
behaviour gets one row below, and each row has a differential test asserting C
and Rust agree on the *same* observable outcome (same fatal signal, or same
returned value + same memory image).

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `tflac_pack_u64le` | `d == NULL` | fatal `SIGSEGV` (write to address 0); no error code exists | `err_null_pack_u64le` | [x] |
| 2 | `tflac_md5_addsample` | `m == NULL` | fatal `SIGSEGV` (`m->total +=` writes address 8) | `err_null_addsample` | [x] |
| 3 | `update_md5` | `t == NULL` | fatal `SIGSEGV` (reads `t->cur_blocksize` at 88) | `err_null_update_md5_t` | [x] |
| 4 | `update_md5` | `samples == NULL` (valid `t`) | fatal `SIGSEGV` (reads `samples[0]`) | `err_null_update_md5_samples` | [x] |
| 5 | `tflac_md5_addsample` | `bits == 0` | no rejection: `total += 0`, `bytes = 0`, still writes 8 bytes at `buffer[pos%64]`, `pos` unchanged, spill loop skipped unless `pos>=64` | `err_bits_zero` | [x] |
| 6 | `tflac_md5_addsample` | `bits` not a multiple of 8 (1..7, 9, 63, 65) | no rejection: integer division truncates, `bytes = bits/8`; `total` still accumulates the raw *bits* value | `err_bits_not_multiple_of_8` | [x] |
| 7 | `tflac_md5_addsample` | `bits == u32::MAX` (and other huge values) → `bytes = 0x1FFF_FFFF` | no rejection: `pos += bytes` (u32 wrap), then `pos %= 64` can make the spill counter up to 63, so `buffer[64+i]` reads **past** the 72-byte buffer — replicated verbatim | `err_bits_huge` | [x] |
| 8 | `tflac_md5_addsample` | `m->pos >= 64` on entry (never sanitised) | no rejection: `pos2 = pos % 64` keeps the store in range, but the spill copy count becomes `(pos+bytes)%64` which may exceed 8 → out-of-buffer reads | `err_pos_out_of_range` | [x] |
| 9 | `tflac_md5_addsample` | `m->pos == u32::MAX` → `pos += bytes` overflows u32 | no rejection: unsigned wraparound, `pos` becomes `bytes-1`; `>= 64` test uses the wrapped value | `err_pos_u32_max` | [x] |
| 10 | `tflac_md5_addsample` | `m->total` near `u64::MAX` → `total += bits` overflows | no rejection: unsigned wraparound | `err_total_overflow` | [x] |
| 11 | `update_md5` | `cur_blocksize * channels` overflows u32 (e.g. `0x1000_0000 * 0x11`) | no rejection: product wraps mod 2^32, then five `-8`s | `err_b_product_overflow` | [x] |
| 12 | `update_md5` | `cur_blocksize * channels < 40` (incl. `0`) → `b -= 8` five times underflows | no rejection: `b` wraps, function returns the huge wrapped u32 (e.g. `0` → `0xFFFF_FFD8`) | `err_b_underflow` | [x] |
| 13 | `update_md5` | `samples` buffer shorter than the 136 `tflac_s32` the fixed 5-iteration/stride-32 loop reads (a *caller* error the C cannot detect) | no rejection: reads `samples[0..8]`, `[32..40]`, `[64..72]`, `[96..104]`, `[128..136]` regardless | `err_samples_stride_reads` | [x] |
| 14 | `tflac_pack_u64le` | `d` unaligned (odd address) | no rejection: byte-at-a-time stores, always succeeds | `err_pack_unaligned` | [x] |
| 15 | `tflac_md5_addsample`, `update_md5` | `m` / `t` **misaligned** (struct pointer offset by 1..8 bytes), plus a misaligned `samples` pointer | no rejection: the C performs ordinary loads/stores that succeed unaligned on this target | `err_misaligned_struct_pointer` | [x] |
| 16 | `tflac_md5_addsample` | full-range `bits` sweep — every `2^k`, `2^k±1` and 64 random `u32` values (the "out-of-range enum value" analogue: `bits` is an unvalidated raw `u32`) | no rejection: `total += bits`, `bytes = bits/8`, wraparound as computed | `err_full_range_bits_sweep` | [x] |
| 17 | all three | fully degenerate state: zeroed buffer, `pos ∈ {0,63,64,u32::MAX}`, `total ∈ {0,u64::MAX}`, `cur_blocksize = channels = 0`, all-zero samples, `bits = u32::MAX` | no rejection: deterministic wrapped results | `err_degenerate_state` | [x] |

Rows 1–4 are compared by running the call in a **child process** and asserting
C and Rust die with the *same* signal. Rows 5–14 are compared by asserting the
returned value **and** the full post-call memory image (struct + surrounding
guard bytes) are byte-identical.

Rows 7, 8, 9 and 13 make the C read outside the declared object. The tests
place the struct inside a large 512-byte pattern-filled arena (and the samples
inside an over-sized array) so that the reads land in defined, identical memory
for both libraries, making the comparison deterministic while still exercising
exactly the C's index arithmetic.

## Divergences found and fixed (Rust changed, C untouched)

1. **Rows 1 and 4 — NULL pointer signal mismatch.** The C died with `SIGSEGV`
   (11) but the Rust `.so` built with debug assertions died with `SIGABRT` (6):
   Rust's debug-only "null pointer dereference occurred" check fires on
   `*ptr = v` / `*ptr` before the fault can happen, and a panic inside an
   `extern "C"` function is a non-unwinding abort. Fixed by performing every
   load/store as a byte-granular *volatile* access (`read_volatile` /
   `write_volatile`), which is exactly what the C compiler emits and which
   faults on address 0 like the C. Both now report signal 11 in *every* profile.
2. **Row 15 — misaligned struct pointer aborted.** `*(p as *mut u64)` on a
   misaligned `tflac_md5*` tripped Rust's debug-only "misaligned pointer
   dereference" check (`SIGABRT`) where the C simply performs the unaligned
   access. The byte-granular accessors (`ld_u32`/`st_u32`/`ld_u64`/`st_u64`
   with `from_ne_bytes`/`to_ne_bytes`) removed the difference, and are
   endian-neutral, so the translation stays correct on big-endian targets too.

No behavioural divergence was found on any valid input (Phase B).
