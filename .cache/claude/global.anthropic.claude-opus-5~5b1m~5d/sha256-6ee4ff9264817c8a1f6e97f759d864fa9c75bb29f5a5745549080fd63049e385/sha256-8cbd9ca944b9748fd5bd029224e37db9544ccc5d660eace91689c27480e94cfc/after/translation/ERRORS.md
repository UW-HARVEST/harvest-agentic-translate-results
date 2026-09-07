# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / error / abort path in
`c_src/src/lib.c`. The file contains **no** `return -1`, no error enum, no
`assert`, and no `RETURN_ERROR` macro. The complete set of non-normal exits is:

* `c_src/src/lib.c:11` — `if (search == NULL) return path;`  (sentinel-driven
  fallback, the only "not found" branch)
* `c_src/src/lib.c:39-42` — `if (!result) { fprintf(stderr, ...); exit(30); }`
  (the only hard failure; documented as *"This function never returns an error
  (it may abort() in case of pb)"*)

Everything else is an *unchecked* precondition: the C code dereferences
`path`, `outDirName` and `outDirName[strlen(outDirName)-1]` without any null or
range check, so those are the generic FFI boundary cases required by the task.
They are listed too, because "the C crashes identically" is the observable
contract across the FFI boundary.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `extractFilename` | `separator` does not occur anywhere in `path` (`strrchr` → `NULL`) | returns `path` itself (offset 0), **not** `NULL` | `e1_separator_absent_returns_path` | [x] |
| E2 | `extractFilename` | `path` is the empty string `""` and `separator != '\0'` (`strrchr` → `NULL`) | returns `path` (offset 0) | `e2_empty_path_returns_path` | [x] |
| E3 | `extractFilename` | `separator == '\0'`: `strrchr` treats the terminator as part of the string, so it *always* matches | returns `path + strlen(path) + 1`, i.e. **one past the NUL** (out-of-bounds pointer, C quirk) | `e3_nul_separator_returns_one_past_end` | [x] |
| E4 | `extractFilename` | `path == NULL` | dereference of `NULL` → `SIGSEGV` (no null check in C) | `e4_null_path_crashes` (forked) | [x] |
| E5 | `FIO_createFilename_fromOutDir` | `calloc(1, N)` returns `NULL` because `N` is unsatisfiable — `suffixLen` huge (e.g. `usize::MAX/2`) | `fprintf(stderr, "zstd: FIO_createFilename_fromOutDir: %s", strerror(errno))` then `exit(30)` — process exit status **30** | `e5_alloc_failure_exits_30` (forked) | [x] |
| E6 | `FIO_createFilename_fromOutDir` | `suffixLen == SIZE_MAX`: `strlen(outDirName)+1+strlen(filenameStart)+suffixLen+1` wraps around `size_t` to a small value | `calloc` **succeeds** with the wrapped size; the function then writes `outDirLen+1+filenameLen` bytes into that too-small buffer (heap overflow, C quirk). Both must compute the same wrapped size. | `e6_suffixlen_size_overflow_wraps` (forked) | [x] |
| E7 | `FIO_createFilename_fromOutDir` | `outDirName` is the empty string `""` → `outDirName[strlen(outDirName)-1]` == `outDirName[-1]`, a read **before** the buffer | no check; the branch taken depends on the byte preceding the buffer. Both impls must read the same byte and take the same branch. | `e7_empty_outdir_reads_byte_before_buffer` (both branches) | [x] |
| E8 | `FIO_createFilename_fromOutDir` | `path == NULL` | `SIGSEGV` inside `extractFilename`/`strrchr` | `e8_null_path_crashes` (forked) | [x] |
| E9 | `FIO_createFilename_fromOutDir` | `outDirName == NULL` | `SIGSEGV` in `strlen(outDirName)` | `e9_null_outdir_crashes` (forked) | [x] |
| E10 | `FIO_createFilename_fromOutDir` | `path == NULL` **and** `outDirName == NULL` | `SIGSEGV` | `e10_both_null_crash` (forked) | [x] |
| E11a | `FIO_createFilename_fromOutDir` | `suffixLen` values whose *wrapped* size is still astronomically large: `usize::MAX-100`, `usize::MAX/2+1`, `1<<62`, `(1<<63)+12345`, `usize::MAX-base` (size == `SIZE_MAX`) | `calloc` fails → `exit(30)` | `e11_near_max_suffixlen_exits_30` (a) (forked) | [x] |
| E11b | `FIO_createFilename_fromOutDir` | `suffixLen` one step *past* the wrap point, so `size` wraps to 0/1/4/12: `usize::MAX-8`, `usize::MAX-base+1`, `usize::MAX-base+2`, `usize::MAX-1`, `usize::MAX` | `calloc` **succeeds** with the tiny wrapped size and the function returns non-NULL (it then writes past the allocation — a genuine C bug that must be reproduced, **not** turned into `exit(30)`) | `e11_near_max_suffixlen_exits_30` (b) (forked) | [x] |
| E5b | `FIO_createFilename_fromOutDir` | same trigger as E5, but the **stderr text** is compared | the exact bytes `"zstd: FIO_createFilename_fromOutDir: Cannot allocate memory"` (no newline), byte-for-byte identical between C and Rust | `e5b_alloc_failure_stderr_is_byte_identical` (piped, forked) | [x] |
| E12 | `FIO_createFilename_fromOutDir` | `separator` argument type confusion / out-of-range "enum": there is no enum in this API, but `char separator` of `extractFilename` accepts **any** `int` at the ABI level. Values `0x80..0xFF` (negative `signed char` on x86-64) and values `> 0xFF` passed as `c_int` must be handled identically. | high-bit bytes match the corresponding byte in `path`; the value is truncated to `char` by the ABI/`strrchr` | `e12_out_of_range_separator_values` | [x] |
| E13 | `FIO_createFilename_fromOutDir` | `path` consisting solely of separators, e.g. `"///"` → `filenameStart` is the empty string, `filenameLen == 0` | succeeds; result is `outDirName` + separator + `""` + NUL padding (no rejection) | `e13_path_all_separators` | [x] |
| E14 | `FIO_createFilename_fromOutDir` | `path` is the empty string `""` → `strrchr` → `NULL` → `filenameStart == path == ""` | succeeds; result is `outDirName` (+ separator) with zero-length filename | `e14_empty_path` | [x] |

## Notes on non-rows

* There is **no** minimum/maximum constant, no range check, and no
  `#define`d limit anywhere in `c_src/src/lib.c` or `c_src/include/lib.h`.
* `extractFilename` never returns `NULL`; the `NULL` from `strrchr` is
  converted into the `path` fallback (row E1). Any Rust version returning
  `NULL` there would be a divergence — covered by E1/E2.
* `exit(30)` (row E5/E11) is a process-level effect, so those tests `fork()`
  and compare `WEXITSTATUS`, for the C `.so` and the Rust `.so` separately.

## Divergence found and fixed during Phase C

* **E4 / E8 / E9 / E10 (null pointers), debug profile only.** The Rust used
  hand-written `strlen` / `strrchr` / `memcpy` helpers. Under
  `-C debug-assertions` (the `dev` profile) Rust's raw-pointer UB checks fire on
  the null dereference and `abort()` — `SIGABRT` (6) — whereas the C faults with
  `SIGSEGV` (11). Fixed in `src/lib.rs` by importing `strlen`, `strrchr` and
  `memcpy` from libc, i.e. the very same symbols the C calls. Both profiles now
  produce `SIGSEGV` identically.

## Reproduction

```
cd translation && ./run_verification.sh
```

builds the C `.so`, then runs Phases B/C/D for both the `dev` and `release`
profiles and for every feature combination (`<default>` and
`--no-default-features`; the crate declares no features, so those are the only
two).
