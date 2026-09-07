# ERRORS.md — Phase A: error-surface table

## Mechanical grep of the whole C source

Commands run over `c_src/` (the entire C tree is `include/lib.h` + `src/lib.c`,
35 lines total):

```
$ grep -nE 'return|RETURN|ERROR|errno|assert|NULL|nullptr|if|else|switch|case|while|for|\?|exit|abort|<|>|==|!=|<=|>=|&&|\|\|' c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:4-19:   out[N] = (tflac_u8)(m->X >> K);   # only ">>" matches; no comparisons
c_src/include/lib.h:14: void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);
```

Findings:

* `md5_digest` returns `void` — there is **no** error return channel at all:
  no error enum, no `int` status, no sentinel pointer, no out-param flag.
* **0** occurrences of `return <value>`, `RETURN_ERROR`, `-1`, `NULL`,
  `assert`, `abort`, `exit`, `errno`.
* **0** occurrences of `if` / `else` / `switch` / `?:` / any loop — the function
  is 16 straight-line assignments, so there are no conditional rejection
  branches to mirror.
* **0** range checks, **0** null checks, **0** min/max constants.
* **0** enum types anywhere in the public header, therefore no
  out-of-range-enum-value class to test (see row 5 below for how this is
  discharged).

Consequence: the C library **has an empty rejection surface**. It never rejects
any input. The table below therefore has no `RETURN_ERROR`-style rows; instead
it records every generic C-API boundary condition that the task mandates
covering even when absent from the source, with the observed C outcome that the
Rust must reproduce.

## Error-surface table

`n/a (no error channel)` below means: the function is `void` and performs no
validation, so the only observable "result" is the memory effect (or the fault
the OS raises). "Same fatal signal" rows are verified by running the call in a
forked child process for each library and comparing the raw `wait` status.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `md5_digest` | `m == NULL` (out valid) | No validation exists; the `mov (%rax),%eax` at `+0x1109` faults. Process dies with **SIGSEGV (raw wait status 11, core-dump flag)**; `out` untouched. | `err_null_m_faults_identically` |
| 2 | `md5_digest` | `out == NULL` (m valid) | No validation exists; the first byte store `mov %dl,(%rax)` at `+0x1111` faults. Process dies with **SIGSEGV**. | `err_null_out_faults_identically` |
| 3 | `md5_digest` | `m == NULL && out == NULL` | Faults on the `m` load first (it precedes the first store). **SIGSEGV**. | `err_both_null_fault_identically` |
| 4 | `md5_digest` | `out` buffer shorter than 16 bytes (undersized length, e.g. 15 / 8 / 1 / 0 bytes usable) | No length parameter exists, so no check is possible: the C unconditionally writes exactly 16 bytes and overruns. Observable result = **16 bytes written, past-the-end bytes clobbered in the same order/values**. Verified against a 16-byte window inside a larger guarded arena (so the overrun is observable but not a crash). | `err_undersized_out_writes_16_anyway` |
| 5 | `md5_digest` | out-of-range enum value across the FFI boundary | **Not applicable — the public header declares no enum type.** The only parameter types are `const tflac_md5 *` and `tflac_u8 *`. The nearest analogue is an arbitrary/garbage bit pattern in the `tflac_u32` fields, which has no invalid encodings: every one of the 2^32 values is valid. Discharged by feeding fully random 32-bit words, including all-zero / all-ones / single-bit / 0x80000000-class values, and comparing byte-for-byte. | `err_no_enum_surface_all_bit_patterns_valid` |
| 6 | `md5_digest` | `out == (tflac_u8*)m` and every overlap offset in `-16..=16` (aliasing input and output — well-defined in C, since writing through a character-type lvalue may alias any object) | The C **re-loads `m->a`/`b`/`c`/`d` from memory after every single byte store** (confirmed in the disassembly: `mov -0x8(%rbp),%rax; mov (%rax),%eax` repeats before each of the 16 stores). So overlapping buffers produce a byte-cascade, not a snapshot copy. Rust must reproduce the exact cascade. | `err_overlapping_buffers_cascade` |
| 7 | `md5_digest` | misaligned `m` (struct pointer at odd/2-mod-4 addresses) | No alignment check; x86-64 `mov` loads succeed. Same 16 output bytes as the aligned case. | `err_misaligned_m_and_out` |
| 8 | `md5_digest` | misaligned `out` | No alignment check; byte stores are always aligned. Same 16 output bytes. | `err_misaligned_m_and_out` |
| 9 | `md5_digest` | "one step past a documented valid range" for the `tflac_u32` fields | The documented range of `uint32_t` is `0..=0xFFFFFFFF` and the type cannot hold anything else; `0xFFFFFFFF + 1` wraps to `0` at the caller. Both endpoints and the wrap are fed explicitly. | `err_no_enum_surface_all_bit_patterns_valid` |

### Row status

* [x] 1 — `err_null_m_faults_identically`
* [x] 2 — `err_null_out_faults_identically`
* [x] 3 — `err_both_null_fault_identically`
* [x] 4 — `err_undersized_out_writes_16_anyway`
* [x] 5 — `err_no_enum_surface_all_bit_patterns_valid`
* [x] 6 — `err_overlapping_buffers_cascade`
* [x] 7 — `err_misaligned_m_and_out`
* [x] 8 — `err_misaligned_m_and_out`
* [x] 9 — `err_no_enum_surface_all_bit_patterns_valid`
