# ERRORS.md — Phase A: error-surface table

Mechanically derived by grepping `c_src/` for **every** rejection construct:

```
$ grep -nE "return|assert|NULL|if|else|switch|while|for|#if|#ifdef|ERROR|errno|-1|MIN|MAX|enum" \
      c_src/src/lib.c c_src/include/lib.h
(no matches)
```

## The C error surface is EMPTY — and that is itself the contract

`md5_digest` is the only entry point. Facts established from the source:

* return type is `void` — there is **no** error code, sentinel, or `errno` use;
* there are **zero** `return` statements (implicit fall-off-the-end only);
* there are **zero** `if` / `switch` / loops — it is 16 straight-line stores;
* there are **zero** `assert`s, null checks, or range checks;
* there are **zero** `#ifdef`s / compile-time configuration macros;
* there are **zero** named constants, min/max bounds, or `enum`s, so there is
  no "one past a valid range" value and **no out-of-range enum variant can be
  passed across the FFI boundary** (the API has no enum parameter at all);
* both parameters are unvalidated raw pointers; `out` has no length parameter
  (the `tflac_u8 out[16]` array-parameter syntax decays to `tflac_u8 *` and is
  **not** checked at runtime).

Therefore the correctness obligation for the Rust port is: it must **not invent
an error surface**. It must not add null checks, not add alignment checks, not
panic, and not abort where C quietly proceeds. Every row below asserts that the
Rust reproduces the C's *unchecked* behaviour, byte-for-byte or signal-for-signal.

## Table

Each row = one distinct way the API can be handed invalid/degenerate input.
"expected C result" is the empirically-verified behaviour of the built `.so`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `md5_digest` | `m == NULL`, `out` valid → deref of null at offset 0 | fatal `SIGSEGV`, no bytes written | `err_null_m_segv_both` | [x] |
| E2 | `md5_digest` | `m` valid, `out == NULL` → store to null | fatal `SIGSEGV` | `err_null_out_segv_both` | [x] |
| E3 | `md5_digest` | `m == NULL && out == NULL` | fatal `SIGSEGV` | `err_both_null_segv_both` | [x] |
| E4 | `md5_digest` | `out` points to a read-only page (writable-length 0) | fatal `SIGSEGV` on first store | `err_readonly_out_segv_both` | [x] |
| E5 | `md5_digest` | `out` buffer shorter than 16 (last byte at page end, next page unmapped) → write past 16 | writes exactly 16 bytes then `SIGSEGV` **only if** <16 mapped; with exactly 16 mapped: **no fault**, proving it never writes a 17th byte | `err_out_exactly_16_no_overrun`, `err_out_15_bytes_segv_both` | [x] |
| E6 | `md5_digest` | `m` truncated: only 15 of 16 source bytes mapped → read past end | fatal `SIGSEGV`; with exactly 16 mapped: no fault, proving it never reads a 17th byte | `err_m_exactly_16_no_overread`, `err_m_15_bytes_segv_both` | [x] |
| E7 | `md5_digest` | `m` misaligned (odd address, not 4-byte aligned) — UB in C, works on x86 | **no** fault, correct little-endian bytes | `err_misaligned_m_no_fault`, `cfg_c9_misaligned_m` | [x] |
| E8 | `md5_digest` | `out` misaligned (odd address) | **no** fault, correct bytes | `err_misaligned_out_no_fault`, `cfg_c8_misaligned_out` | [x] |
| E9 | `md5_digest` | `m` non-null but wildly invalid / never-mapped address (e.g. `0x1`) | fatal `SIGSEGV` | `err_wild_m_segv_both` | [x] |
| E10 | `md5_digest` | `out` non-null but never-mapped address (e.g. `0x1`) | fatal `SIGSEGV` | `err_wild_out_segv_both` | [x] |
| E11 | `md5_digest` | all-zero input struct (degenerate but legal) — must NOT be treated as "empty/error" | 16 zero bytes written, no error | `err_all_zero_is_not_an_error` | [x] |
| E12 | `md5_digest` | `out == (tflac_u8*)m` (full self-overlap; legal C, no `restrict`) | **no** error; C reloads each field before each store, so output is defined and equals a byte-wise ascending copy | `err_exact_overlap_defined`, `cfg_c11_overlap_exact` | [x] |
| E13 | `md5_digest` | called twice in a row on the same `out` (no reset/idempotency check) | second call overwrites; identical result | `err_repeat_call_idempotent` | [x] |

Rows E1–E4, E9, E10 are verified by **forking a child process** and comparing
the exact termination signal from the C `.so` and the Rust `.so` — i.e. the same
rejection, not merely "both failed somehow". Rows E5/E6 use `mmap` with an
adjacent `PROT_NONE` guard page so an off-by-one read/write is turned into a
deterministic, observable `SIGSEGV`.

## Note on "no error surface" ≠ "nothing to test"

The absence of validation means the *only* observable "error" behaviour is the
hardware fault, and the failure mode a naive Rust port exhibits is the
*opposite* one: adding a check and returning quietly, or panicking with a Rust
message instead of faulting. Rows E1–E4/E9/E10 exist specifically to catch that.
