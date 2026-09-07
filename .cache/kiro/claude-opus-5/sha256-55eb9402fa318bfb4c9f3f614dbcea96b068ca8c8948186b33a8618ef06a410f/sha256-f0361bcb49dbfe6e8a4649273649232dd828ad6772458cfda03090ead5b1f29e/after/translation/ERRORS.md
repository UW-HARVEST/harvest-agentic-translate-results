# ERRORS.md — Phase C error-surface table

## How this table was derived

Mechanical grep over the entire C source (`c_src/src/lib.c`, `c_src/include/lib.h`):

| grep | matches |
|------|---------|
| `return` | **1** — `src/lib.c:50: return b;` (a success value, not an error) |
| `assert` | 0 |
| `NULL` | 0 |
| `errno` | 0 |
| `RETURN_ERROR` / `*_ERROR` | 0 |
| `enum` | 0 |
| `goto` | 0 |
| `return -1` / `return NULL` | 0 |
| `if` | **1** — `src/lib.c:24: if (m->pos >= 64)` (a control branch, not a rejection) |
| `switch` / `?:` / `#if` | 0 |

**The C library performs no input validation whatsoever.** There is no error
code, no sentinel return, no null check, no range check and no assertion. So the
rejection surface is not "which error does it return" but "what does it do
instead of rejecting": it either wraps arithmetic (well-defined for the unsigned
types used) or performs an unchecked memory access.

Each row below is therefore one distinct *invalid* input the C accepts anyway.
The differential requirement is that the Rust must produce the **same** wrapped
value / same bytes / same fatal signal — not that either one reports an error.

There are **no enum-typed parameters** anywhere in the public API (`grep enum` →
0 matches), so the "out-of-range enum value" class is instead covered by the
out-of-range values of the two unconstrained scalar parameters the code does
branch on: `bits` (rows 7–11) and the `pos` field (rows 12–13).

Out-of-bounds rows are made deterministic by embedding the struct at offset 0 of
a larger, identically-initialised backing buffer for both libraries, so the
stray accesses land in memory the test controls.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `tflac_pack_u64le` | `d == NULL` | no check; store to address 0 ⇒ fatal `SIGSEGV` | [x] |
| 2 | `tflac_pack_u64le` | `d` unaligned (odd address) | no check; 8 independent byte stores, succeeds | [x] |
| 3 | `tflac_pack_u64le` | `d` = `&buffer[64+8-8]` = last legal 8-byte slot (`buffer[64..72]`), i.e. one step from overflowing the array | no check; writes exactly the final 8 bytes of `buffer`, succeeds | [x] |
| 4 | `tflac_pack_u64le` | `d` = `&buffer[65]`, i.e. one step *past* the last legal slot | no check; writes 1 byte past `buffer`, into `tflac`'s tail padding — succeeds silently | [x] |
| 5 | `tflac_md5_addsample` | `m == NULL` | no check; `m->total += bits` stores to address 8 ⇒ fatal `SIGSEGV` | [x] |
| 6 | `tflac_md5_addsample` | `bits == 0` (zero length) | no rejection: `bytes = 0`, `total += 0`, still packs 8 bytes at `buffer[pos%64]`, `pos` unchanged; carry-down branch taken iff `pos` was already ≥ 64 | [x] |
| 7 | `tflac_md5_addsample` | `bits` not a multiple of 8 (1..7, 9, 63, 65) | no rejection: `bytes = bits/8` truncates toward zero, but `total += bits` keeps the un-truncated value ⇒ `total` and `pos` disagree | [x] |
| 8 | `tflac_md5_addsample` | `bits == 0xFFFFFFFF` (oversized length) | no rejection: `bytes = 0x1FFFFFFF`, `total += 0xFFFFFFFF`, `pos += 0x1FFFFFFF` then `pos %= 64` ⇒ carry-down runs with a large `bytes` | [x] |
| 9 | `tflac_md5_addsample` | `m->total` near `UINT64_MAX` so `total + bits` overflows | no check; unsigned wrap modulo 2^64 | [x] |
| 10 | `tflac_md5_addsample` | `m->pos` near `UINT32_MAX` so `pos + bytes` overflows | no check; unsigned wrap modulo 2^32, then the `>= 64` test sees the *wrapped* value | [x] |
| 11 | `tflac_md5_addsample` | `m->pos == 63` with `bits == 64` ⇒ `pack_u64le(&buffer[63], …)` writes `buffer[63..71]` — the boundary case one step from overflowing `buffer` | no check; last write lands on `buffer[70]`, still inside the 72-byte array | [x] |
| 12 | `tflac_md5_addsample` | `m->pos ≥ 64` on entry (outside the documented 0..63 ring range), e.g. `pos = 64`, `100`, `1000`, `0xFFFFFFFF` | no check; `pos2 = pos % 64` is used for the write, but the *unreduced* `pos` is what `bits/8` is added to, then reduced | [x] |
| 13 | `tflac_md5_addsample` | carry-down loop `m->buffer[bytes] = m->buffer[64 + bytes]` with final `pos` in 1..63 ⇒ source index up to `64+62 = 126` | no bounds check; reads up to 55 bytes past the end of the 72-byte `buffer` (54 past the 88-byte `tflac_md5`), emitted as a plain unchecked `movzbl 0x10(%rax,%rdx,1)` | [x] |
| 14 | `update_md5` | `t == NULL` | no check; load of `0x58(%rdi)` (`cur_blocksize`) ⇒ fatal `SIGSEGV` | [x] |
| 15 | `update_md5` | `samples == NULL` | no check; load of `(%rax)` ⇒ fatal `SIGSEGV` | [x] |
| 16 | `update_md5` | `cur_blocksize * channels < 40`, e.g. `0*0`, `1*1`, `4*8`, `39` | no rejection and no clamp: `b -= 8` five times underflows ⇒ returns `product - 40` modulo 2^32 (e.g. `0` ⇒ `0xFFFFFFD8`) | [x] |
| 17 | `update_md5` | `cur_blocksize * channels == 40` exactly (boundary: result is exactly 0) | returns `0` — indistinguishable from "no samples left" | [x] |
| 18 | `update_md5` | `cur_blocksize * channels` overflows `tflac_u32`, e.g. `0x10000 * 0x10000`, `0xFFFFFFFF * 3` | no check; unsigned wrap modulo 2^32, then `-40` | [x] |
| 19 | `update_md5` | `samples` array shorter than the 136 `tflac_s32` the fixed 5-iteration loop reads (the pointer advances 32 elements per step, so it touches `[0..8) ∪ [32..40) ∪ [64..72) ∪ [96..104) ∪ [128..136)`) | no length parameter exists, so no check; reads whatever follows the caller's array | [x] |
| 20 | `update_md5` | negative `samples[i]` (e.g. `INT32_MIN`, `-1`) — the `(tflac_uint)` cast sign-extends to 64 bits *before* the `& 0xFF` | no rejection; only the low byte survives the mask, so `-1` and `0xFF` are indistinguishable | [x] |
| 21 | `update_md5` | `t->md5_ctx.pos` already ≥ 56 so that the carry-down of row 13 fires *inside* the 5-iteration loop | no check; combines rows 12–13 with the loop's `pos += 8` per iteration | [x] |

## Finding fixed during Phase C/D (rows 1 and 15)

Rows 1 and 15 initially **diverged in the `debug` profile only**:

| | C | Rust (before fix) |
|---|---|---|
| `tflac_pack_u64le(NULL, n)` | `SIGSEGV` (11) | `SIGABRT` (6) |
| `update_md5(t, NULL)` | `SIGSEGV` (11) | `SIGABRT` (6) |

Cause: Rust's debug-assertions build inserts a runtime null-pointer check on
plain `*ptr` dereferences (`"null pointer dereference occurred"`, at
`src/lib.rs:61` and `:153`). That check panics, and a panic crossing an
`extern "C"` boundary aborts. The C emits an unchecked store/load and faults.

Only the two offset-0 accesses were affected. `tflac_md5_addsample(NULL, …)`
already matched because its first access is `(*m).total` at address 8, and
`update_md5(NULL, …)` already matched because its first access is
`(*t).cur_blocksize` at address 88 — both non-null, so the check passes and the
access faults naturally, exactly as in C.

Converting to `core::ptr::read_unaligned`/`write_unaligned` did **not** fix it:
those trip a different debug-only assertion (`unsafe precondition(s) violated:
ptr::copy_nonoverlapping requires … non-null`). Probing every primitive showed
that only `core::ptr::read_volatile`/`write_volatile` carry no instrumentation
and fault with signal 11 like the C:

| primitive | debug-profile outcome on a null pointer |
|-----------|----------------------------------------|
| `*ptr` | SIGABRT — "null pointer dereference occurred" |
| `read_unaligned` / `write_unaligned` | SIGABRT — `copy_nonoverlapping` precondition |
| `read_volatile` / `write_volatile` | **SIGSEGV — matches C** |

Fix: all 27 raw-pointer accesses in `src/lib.rs` now use
`read_volatile`/`write_volatile`. This is a change of access *primitive* only —
no arithmetic, cast, mask, shift, offset or loop bound was touched — and it makes
the fault behaviour profile-independent. Alignment is unaffected: `pos` (u32 @ 0),
`total` (u64 @ 8) and `buffer` (u8, align 1) are naturally aligned in any
correctly allocated `tflac_md5`.

Verified after the fix: all six NULL cases (`pack_null`, `addsample_null`,
`addsample_null_bits0`, `update_t_null`, `update_samples_null`,
`update_both_null`) die with signal 11 in **both** libraries under **both**
profiles.
