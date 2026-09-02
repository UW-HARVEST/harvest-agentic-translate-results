# ERRORS.md — Phase A error-surface table

Derived mechanically from `c_src/src/lib.c`. The file contains **11 `return -1`
statements**, **0 `assert`**, **0 `NULL` checks**, **0 error enums**, and one
implicit trap (unchecked pointer dereference). Every one gets a row.

Constants that bound the input space (all literal in the C):
`blocksize ∈ [16, 65535]`, `samplerate ∈ [1, 655350]`, `channels ∈ [1, 8]`,
`bitdepth ∈ [1, 32]`, `max_rice_value ∈ {0} ∪ [1, 30]`,
`max_partition_order ∈ [0, 15]`, `min_partition_order ≤ max_partition_order`.

`tflac_size_memory` has **no** rejection path: no checks, no asserts, no error
return. Its only "edge" behaviour is unsigned 32-bit wraparound (covered in
`CONFIGS.md` rows S6–S9), so it contributes only row 13 below (a no-error
assertion) rather than an error row.

Side-effect note: rows 9, 10 and 11 return `-1` *after* the function has
already mutated `channel_mode` and/or `max_rice_value`. Every error-path test
therefore compares the **full 28 struct bytes** after the call, not just the
return value — "same error code" alone would hide a divergent partial mutation.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `flac_validate` | `blocksize < 16` (line 16) — e.g. 0, 1, 15 | returns `-1`, struct **unmodified** | [x] |
| 2 | `flac_validate` | `blocksize > 65535` (line 18) — e.g. 65536, 0xFFFFFFFF | returns `-1`, struct unmodified | [x] |
| 3 | `flac_validate` | `samplerate == 0` (line 20) | returns `-1`, struct unmodified | [x] |
| 4 | `flac_validate` | `samplerate > 655350` (line 22) — e.g. 655351, 0xFFFFFFFF | returns `-1`, struct unmodified | [x] |
| 5 | `flac_validate` | `channels == 0` (line 24) | returns `-1`, struct unmodified | [x] |
| 6 | `flac_validate` | `channels > 8` (line 26) — e.g. 9, 0xFFFFFFFF | returns `-1`, struct unmodified | [x] |
| 7 | `flac_validate` | `bitdepth == 0` (line 28) | returns `-1`, struct unmodified | [x] |
| 8 | `flac_validate` | `bitdepth > 32` (line 30) — e.g. 33, 0xFFFFFFFF | returns `-1`, struct unmodified | [x] |
| 9 | `flac_validate` | `max_rice_value != 0 && max_rice_value > 30` (line 43/44) — e.g. 31, 255 | returns `-1`; `channel_mode` may already have been forced to 0; `max_rice_value` untouched | [x] |
| 10 | `flac_validate` | `max_partition_order > 15` (line 46/47) — e.g. 16, 255 | returns `-1`; `channel_mode` and auto-filled `max_rice_value` (14 or 30) already written | [x] |
| 11 | `flac_validate` | `min_partition_order > max_partition_order` (line 49/50) — e.g. min=5,max=4; min=255,max=15 | returns `-1`; `channel_mode` / auto `max_rice_value` already written; `partition_order`, `cur_blocksize` **not** written | [x] |
| 12 | `flac_validate` | `t == NULL` — no null check exists, first statement dereferences `t` | undefined behaviour: SIGSEGV in **both** C and Rust (Rust `&mut *t` on null). Verified in a forked child so the harness survives; asserted that both die the same way. | [x] |
| 13 | `tflac_size_memory` | *(no rejection path exists)* — any `u32`, incl. 0 and `u32::MAX` | never fails; result is the wrapping `u32` expression. Asserted equal for exhaustive-boundary + randomized `u32` inputs. | [x] |

## Check-ordering rows (the checks are sequential, so precedence is observable)

The C evaluates rows 1→11 strictly in source order. A struct that violates
*several* rules must be rejected by the **earliest** one, and the later
mutations must **not** have happened. These are distinct observable behaviours,
so they are tested as well:

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 14 | `flac_validate` | all-invalid struct (`blocksize=0`, `samplerate=0`, `channels=0`, `bitdepth=0`, `max_rice_value=255`, `max_partition_order=255`, `min_partition_order=255`, `channel_mode=255`) | returns `-1` via row 1; **no** field mutated (in particular `channel_mode` stays 255 and `max_rice_value` stays 255) | [x] |
| 15 | `flac_validate` | valid sizes but `max_rice_value=255` **and** `max_partition_order=255` | rejected by row 9 (rice first), so `max_partition_order` unchanged | [x] |
| 16 | `flac_validate` | valid sizes, `max_rice_value=0`, `max_partition_order=255` | rejected by row 10, but `max_rice_value` was already auto-filled to 14/30 by bitdepth | [x] |
| 17 | `flac_validate` | out-of-range `channel_mode` enum value (5..=255 — no valid `TFLAC_CHANNEL_MODE` variant, incl. `TFLAC_CHANNEL_MODE_COUNT`=4) crossing the FFI boundary | C compares `!= TFLAC_CHANNEL_INDEPENDENT` only, so any nonzero value is *kept* when `channels==2 && bitdepth!=32`, and forced to 0 otherwise. **Not** an error. | [x] |

All 17 rows are covered by `tests/phase_c_errors.rs` (rows 1–11, 14–17) and
`tests/phase_c_null.rs` (row 12) / `tests/phase_b_configs.rs` (row 13).

## Divergence found and fixed

**Row 12 (`flac_validate(NULL)`) diverged.** The original translation opened
with `let t = unsafe { &mut *t };`. Under `-C debug-assertions` rustc inserts a
null/alignment assertion when a reference is formed from a raw pointer; the
resulting panic cannot unwind out of an `extern "C"` function, so the process
aborted:

```
C   : signal=SIGSEGV (11)
Rust: signal=SIGABRT (6)   <-- "null pointer dereference occurred"
```

Rewriting field access as `(*t).field` was *not* sufficient — rustc emits the
same assertion for raw place reads. The fix uses `core::ptr::addr_of!` /
`addr_of_mut!` with `ptr::read` / `ptr::write`, which lower to a bare load/store
and trap identically to the C. Both sides now die with SIGSEGV in the debug and
release builds alike.

No divergence was found on any other row.
