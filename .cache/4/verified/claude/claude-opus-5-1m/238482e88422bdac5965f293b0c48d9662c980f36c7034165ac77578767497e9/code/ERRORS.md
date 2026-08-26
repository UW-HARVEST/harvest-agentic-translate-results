# ERRORS.md — Phase A: error-surface table

## Mechanical derivation

Every line of `c_src/src/lib.c` and `c_src/include/lib.h` was grepped for every
rejection mechanism a C API can use:

```
$ grep -nE "return|assert|NULL|if|switch|while|for|error|ERROR|enum|#if|<|>|\?" \
        c_src/src/lib.c c_src/include/lib.h
```

The only matches are the 16 assignment statements themselves (they match on
`>` because of the `>>` shift operator) and `#include <stdint.h>`.

Result of the mechanical scan:

| rejection mechanism | occurrences in C |
|---------------------|------------------|
| error-return macro (`RETURN_ERROR`, ...) | 0 |
| `return <value>` / `return NULL` / `return -1` | 0 (function is `void`, no `return` at all) |
| error enum / status code / `errno` set | 0 (no `enum`, no `int` return, no out-param status) |
| `assert` / `abort` / `exit` | 0 |
| explicit range / bounds / size check | 0 |
| NULL check | 0 |
| `min`/`max`/limit constants | 0 |
| conditional branch of any kind (`if`/`switch`/`?:`/loop) | 0 — the body is straight-line code |
| function parameters that are enums or flags | 0 — parameters are `const tflac_md5 *` and `tflac_u8[16]` |

**Conclusion: `md5_digest` has no in-band error surface.** It cannot reject any
input, cannot report failure, and returns `void`. All `2^128` values of
`struct tflac_md5` are valid inputs and are covered by `CONFIGS.md`. There is no
enum parameter, so "out-of-range enum value across FFI" is *not applicable*
(documented as row E8 for completeness).

The only way to make this function fail is to violate its pointer contract.
Those are the rows below: they are memory-fault ("rejection by the MMU")
behaviours, and the requirement is that the Rust `.so` fault **identically** to
the C `.so` — same signal, and the same bytes committed to the output buffer
before the fault. Each row is tested differentially by forking a child per
implementation and comparing (exit-signal, bytes written into a `MAP_SHARED`
buffer).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `md5_digest` | `m == NULL`, `out` valid (16 writable bytes) | SIGSEGV (11) on the first 4-byte load `mov (%rax),%eax` from address 0; 0 bytes written to `out` |
| E2 | `md5_digest` | `m` valid, `out == NULL` | SIGSEGV (11) on the first byte store `mov %dl,(%rax)` to address 0; 0 bytes of output produced |
| E3 | `md5_digest` | `m == NULL` and `out == NULL` | SIGSEGV (11) — fault on the load of `m->a`, which is sequenced before the first store |
| E4 | `md5_digest` | `m` = non-NULL wild pointer into a `PROT_NONE` page (unreadable but mapped) | SIGSEGV (11) on the load of `m->a`; 0 bytes written to `out` |
| E5 | `md5_digest` | `out` = non-NULL pointer into a `PROT_READ`-only page (unwritable) | SIGSEGV (11) on the store to `out[0]`; 0 bytes written |
| E6 | `md5_digest` | `out` buffer shorter than 16 bytes: only `k` writable bytes (`k = 0..15`), followed by a `PROT_NONE` guard page | SIGSEGV (11) on the store to `out[k]`; exactly the first `k` bytes of the little-endian digest are committed before the fault |
| E7 | `md5_digest` | `m` readable for only `k` bytes (`k = 0..15`), followed by a `PROT_NONE` guard page | SIGSEGV (11) on the *4-byte* load of the first field that touches or crosses the boundary (so the fault happens at store index `4*floor(k/4)`, i.e. a partially readable field yields **no** partial bytes for that field); the bytes for wholly readable fields are committed to `out` first |
| E8 | `md5_digest` | out-of-range enum value passed across FFI | **N/A — no enum, flag or `int` parameter exists in the C API.** Documented so the omission is deliberate, not an oversight. The whole `u32` value space of the struct fields is exercised in `CONFIGS.md` rows C2–C6 instead. |
| E9 | `md5_digest` | zero-length / oversized length argument | **N/A — the C API takes no length argument.** The output length is fixed by the prototype (`tflac_u8 out[16]`, which C decays to a bare pointer, so it is unchecked). Under/over-sized output buffers are covered by E6 and by the exactness rows C13/C14 in `CONFIGS.md`. |
| E10 | `md5_digest` | `m` and `out` alias / overlap (undefined for a `const`-qualified param in the strictest reading, but well-defined for the emitted `-O0` code) | No fault. Not an error at all: the emitted code re-loads the field before each of the 16 stores, so overlapping writes are observed by subsequent loads. Byte-exact match required. Covered by `CONFIGS.md` C10–C12. |

Rows E1–E7 have differential tests in `tests/error_paths.rs`; E8–E10 are
resolved as noted (E10 is additionally tested on the valid path).

## Status

| row | test | dev | release |
|-----|------|-----|---------|
| E1 | `error_paths::e01_null_m` | PASS | PASS |
| E2 | `error_paths::e02_null_out` | PASS | PASS |
| E3 | `error_paths::e03_both_null` | PASS | PASS |
| E4 | `error_paths::e04_unreadable_m` | PASS | PASS |
| E5 | `error_paths::e05_readonly_out` | PASS | PASS |
| E6 | `error_paths::e06_short_out_buffer` (k = 0..15) | PASS | PASS |
| E7 | `error_paths::e07_short_m_buffer` (k = 0..15) | PASS | PASS |
| E8 | N/A, pinned by `error_paths::e08_e09_no_enum_or_length_parameters_in_api` | PASS | PASS |
| E9 | N/A, pinned by the same test | PASS | PASS |
| E10 | not an error; covered on the valid path by `CONFIGS.md` C10–C12 | PASS | PASS |

All rows compare the terminating signal AND the bytes committed to a
`MAP_SHARED` output buffer before the fault, so store order and the exact fault
boundary are compared — not merely "both crashed".

Two real divergences were found by these rows and fixed in the Rust (see
`VERIFICATION.md`): NULL pointers aborted with SIGABRT instead of faulting with
SIGSEGV (E1–E3, `dev`), and the release optimizer narrowed the 4-byte field load
so the Rust committed a byte where the C faults first (E7, k=1..3, `release`).
