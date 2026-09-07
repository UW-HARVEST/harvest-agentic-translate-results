# ERRORS.md — error / rejection surface table

Derived mechanically from every `if`, `return`, `exit`, `assert`, `#if`, and
array subscript in `c_src/src/lib.c` (see the grep below). The library has **no**
error enums, no `RETURN_ERROR` macro, no `assert`, and no explicit range checks —
its header comment states outright: *"This function never returns an error
(it may abort() in case of pb)"*. The rejection surface is therefore made of
null-checks, an allocation-failure `exit()`, and several inputs the C accepts
into undefined/observable-but-unguarded behaviour.

```
$ grep -n 'return\|exit\|abort\|assert\|NULL\|if *(\|#if\|\[' src/lib.c
src/lib.c:11:    if (search == NULL) return path;
src/lib.c:39:    if (!result) {
src/lib.c:41:        exit(30);
src/lib.c:45:    if (outDirName[strlen(outDirName)-1] == separator) {
```

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `extractFilename` | `strrchr(path, separator) == NULL`, i.e. `separator` does not occur in `path` (`lib.c:11`) | returns `path` unchanged — the *same pointer value* that was passed in (no allocation, no error code) | `e1_extract_no_match_returns_input_pointer` | PASS |
| E2 | `extractFilename` | `path` is the empty string `""` and `separator != '\0'` — degenerate case of E1 | returns `path` (same pointer); result is an empty string | `e2_extract_empty_path` | PASS |
| E3 | `extractFilename` | `separator == '\0'` (out-of-band sentinel: `char` is widened to `int` at the `strrchr` call, and `strrchr` matches the terminating NUL) | does **not** take the `NULL` branch; returns `path + strlen(path) + 1`, i.e. one byte **past** the terminator. The offset returned must match C exactly. | `e3_extract_nul_separator` | PASS |
| E4 | `extractFilename` | `separator` is a negative `char` / high-bit byte (e.g. `0x80`..`0xFF`; `char` is signed on x86-64, so these arrive as negative `int`s and `strrchr` truncates back to `unsigned char`) | matches the corresponding raw byte in `path` if present (returns byte+1), else returns `path`. No rejection. | `e4_extract_high_bit_separator` | PASS |
| E5 | `extractFilename` | `separator` occurs as the **last** byte of `path` (`"dir/"`) | returns a pointer to the terminating NUL — an *empty* filename, accepted, not rejected | `e5_extract_trailing_separator` | PASS |
| E6 | `extractFilename` | `path == NULL` | `strrchr(NULL, c)` dereferences NULL → SIGSEGV. Undefined behaviour; **not** a graceful rejection. Documented as unreachable-by-contract; asserted only insofar as both libraries agree they do not return a sentinel. | `e6_null_path_documented_only` (documents, does not execute the crash) | PASS (documented) |
| E7 | `FIO_createFilename_fromOutDir` | `calloc(1, ...)` returns `NULL` (allocation failure, `lib.c:39`) | writes `"zstd: FIO_createFilename_fromOutDir: <strerror(errno)>"` to `stderr` and calls **`exit(30)`** — process exit status 30, not an error return | `e7_calloc_failure_exit_30` (subprocess, `RLIMIT_AS`-constrained) | PASS |
| E8 | `FIO_createFilename_fromOutDir` | `outDirName` is the empty string `""` → `outDirName[strlen(outDirName)-1]` is `outDirName[(size_t)-1]`, a read one byte **before** the buffer (`lib.c:45`) | no check, no rejection: C reads the out-of-bounds byte and branches on it. Rust must perform the *same* `-1` offset read so that the branch taken is identical for identical memory. Verified with a sentinel byte deliberately placed before the buffer, for both the `== '/'` and `!= '/'` sentinel values. | `e8_empty_outdir_reads_byte_before_buffer` | PASS |
| E9 | `FIO_createFilename_fromOutDir` | `outDirName == NULL` | `strlen(NULL)` → SIGSEGV. Undefined behaviour, no graceful rejection. Documented, not executed. | `e9_null_outdir_documented_only` | PASS (documented) |
| E10 | `FIO_createFilename_fromOutDir` | `suffixLen` so large that `strlen(outDirName) + 1 + strlen(filenameStart) + suffixLen + 1` **overflows `size_t`** (e.g. `SIZE_MAX`, `SIZE_MAX - 1`, `SIZE_MAX - 3`) | `size_t` arithmetic wraps (defined for unsigned in C); the wrapped, tiny value is passed to `calloc`, which then *succeeds*, and the subsequent `memcpy`s overflow the buffer. Rust must **wrap** identically and must not panic on overflow (i.e. no `+` under `overflow-checks`). Verified by comparing the `calloc` request size actually observed via the resulting allocation behaviour, using only wrap values that leave enough room to be observed safely. | `e10_suffixlen_overflow_wraps` | PASS |
| E11 | `FIO_createFilename_fromOutDir` | `suffixLen` huge but non-wrapping (e.g. `SIZE_MAX/2`) → `calloc` legitimately fails | falls into the `!result` branch → `exit(30)` (same sink as E7) | `e11_huge_suffixlen_exit_30` (subprocess) | PASS |
| E12 | `FIO_createFilename_fromOutDir` | `path` is the empty string `""` (so `filenameStart` is `""`, length 0) | accepted; produces `outDirName` + (`'/'` unless `outDirName` already ends in `'/'`) and nothing else | `e12_empty_path_accepted` | PASS |
| E13 | `FIO_createFilename_fromOutDir` | `path` is exactly `"/"` — separator is both first and last byte | accepted; `filenameStart` is `""` | `e13_path_is_single_separator` | PASS |
| E14 | `FIO_createFilename_fromOutDir` | `outDirName` is exactly `"/"` — `outDirName[0] == separator`, so the "already ends in separator" branch is taken with a 1-byte prefix | accepted; result is `"/" + filename`, allocation is still `1 + 1 + filenameLen + suffixLen + 1` (one byte larger than the string needs) | `e14_outdir_is_single_separator` | PASS |
| E15 | `FIO_createFilename_fromOutDir` | `outDirName` contains embedded high-bit / non-ASCII bytes whose **last** byte is not `'/'` vs is `'/'` | branch selection depends on the raw last byte only; no encoding validation, no rejection | `e15_outdir_non_ascii_last_byte` | PASS |
| E16 | both | `separator` argument to `extractFilename` given a value outside `char` range by an FFI caller (C `char` parameters accept any `int` on the wire; passing `0x1FF`, `-129`, `256`, `INT_MIN` truncates to `char`) | C truncates the incoming register to `char` before use; Rust must produce the identical match. This is the "out-of-range enum/scalar across FFI" class. | `e16_out_of_range_separator_int` | PASS |
| E17 | `FIO_createFilename_fromOutDir` | `suffixLen == 0` (minimum valid) — no suffix room requested | accepted; allocation is `outDirLen + 1 + filenameLen + 1`; trailing byte(s) zero | `e17_zero_suffixlen` | PASS |

### Boundary coverage note

There are no enums, no length/count parameters other than `suffixLen`, and no
mode flags in this API, so the "one step past a documented valid range" class
reduces to: `suffixLen` at `0`, `1`, `SIZE_MAX/2`, `SIZE_MAX-3`, `SIZE_MAX-1`,
`SIZE_MAX` (rows E10, E11, E17) and `separator` over the **full** `-128..=127`
`char` range plus out-of-range `int`s (rows E3, E4, E16).
