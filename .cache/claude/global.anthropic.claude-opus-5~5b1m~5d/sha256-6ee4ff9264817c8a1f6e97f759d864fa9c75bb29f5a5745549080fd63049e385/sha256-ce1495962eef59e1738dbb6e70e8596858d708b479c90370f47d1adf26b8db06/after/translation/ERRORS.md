# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Every `return`, every `if`, every
error/exit statement, every implicit rejection and every boundary read in the C
source is enumerated below — one row per **distinct** rejection / degenerate
condition.

The C library is unusual in that it has **no error return codes at all**: the
header comment states *"This function never returns an error (it may abort() in
case of pb)"*. Its "error surface" therefore consists of (a) the one hard-exit
path, (b) the sentinel-style `NULL`-search fallback, and (c) the degenerate /
out-of-range inputs that the code reads without validating. All of them are
still observable across the FFI boundary and all must match.

Grep evidence:

```
src/lib.c:11:    if (search == NULL) return path;      # sentinel fallback
src/lib.c:12:    return search+1;
src/lib.c:39:    if (!result) {                         # allocation failure
src/lib.c:40:        fprintf(stderr, "zstd: FIO_createFilename_fromOutDir: %s", strerror(errno));
src/lib.c:41:        exit(30);                           # the ONLY error exit
src/lib.c:45:    if (outDirName[strlen(outDirName)-1] == separator)   # unguarded [-1] read
src/lib.c:52:    return result;
```

Note there is **no** null-pointer check, **no** length/range validation, and
**no** `assert` anywhere in the C source. That absence is itself the specified
behaviour: passing `NULL` makes the C crash, so the Rust must crash the same way
rather than defensively returning something. Rows 8–10 pin that down.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|---------------------------------------------|-------------------|------|----|
| 1 | `extractFilename` | separator absent from `path` → `strrchr` returns `NULL` (`lib.c:11`) | returns `path` **unchanged** (identical pointer, offset 0) | `err_01_extract_separator_absent` | [x] |
| 2 | `extractFilename` | `path` is the empty string `""` (separator absent from a zero-length string) | returns `path` unchanged (offset 0), *not* `path+1` | `err_02_extract_empty_path` | [x] |
| 3 | `extractFilename` | `separator == 0` — the NUL byte. Per C, the terminator is part of the string, so `strrchr` *succeeds* at index `strlen(path)` | returns `path + strlen(path) + 1` — one **past** the terminator (a pointer that must not be dereferenced) | `err_03_extract_nul_separator` | [x] |
| 4 | `extractFilename` | separator occurs as the **last** byte of `path` (e.g. `"a/"`, `'/'`) | returns pointer to the terminating NUL → an empty string, offset `strlen(path)` | `err_04_extract_trailing_separator` | [x] |
| 5 | `extractFilename` | `separator` is a non-ASCII / sign-extended byte (`0x80`, `0xFF`), i.e. a negative `char` on x86-64 | byte-compared as `char`; found/not-found identically — no promotion mismatch | `err_05_extract_negative_separator` | [x] |
| 6 | `FIO_createFilename_fromOutDir` | `outDirName` is the empty string `""` → `lib.c:45` evaluates `outDirName[strlen("")-1]` = `outDirName[(size_t)-1]` = **`outDirName[-1]`**, a read one byte *before* the buffer | reads that out-of-bounds byte and branches on it; must read the **same address** in Rust so both branch identically for the same caller-supplied buffer | `err_06_empty_outdir_oob_read` | [x] |
| 7 | `FIO_createFilename_fromOutDir` | `suffixLen` so large that `strlen(outDirName)+1+strlen(filenameStart)+suffixLen+1` **wraps** `size_t` (e.g. `suffixLen == SIZE_MAX`) | `size_t` wrap-around, `calloc` of the wrapped (tiny) size **succeeds**, buffer is filled with dir+sep+name and is **not** NUL-terminated. No error is reported | `err_07_suffixlen_size_t_overflow` | [x] |
| 8 | `FIO_createFilename_fromOutDir` | `suffixLen` huge but non-wrapping (e.g. `SIZE_MAX/2`) → `calloc` returns `NULL` → `!result` (`lib.c:39`) | writes `"zstd: FIO_createFilename_fromOutDir: <strerror(errno)>"` to `stderr` and calls **`exit(30)`** — process exit status 30, no return | `err_08_alloc_failure_exit_30` (forked child, asserts exit code 30 from *both*) | [x] |
| 9 | `extractFilename` | `path == NULL` — no null check exists in C (`strrchr(NULL, c)`) | dereferences NULL → fatal signal (`SIGSEGV`) | `err_09_null_path_extract` (forked child, asserts same signal) | [x] |
| 10 | `FIO_createFilename_fromOutDir` | `path == NULL` / `outDirName == NULL` — no null checks exist in C | dereferences NULL → fatal signal (`SIGSEGV`) | `err_10_null_args_create` (forked child, asserts same signal) | [x] |
| 11 | `extractFilename` | `separator` passed as an **out-of-range value for the declared `char` parameter** across FFI (a C `char` parameter accepts any `int` in the register; e.g. `0x1FF`, `-300`). Only the low 8 bits are significant | truncated to the low byte and compared as `char`; identical result to passing that low byte | `err_11_out_of_range_separator_int` | [x] |
| 12 | `FIO_createFilename_fromOutDir` | `suffixLen == 0` (zero length — the minimum) | no error; allocates exactly `dirLen + 1 + nameLen + 1` and returns the joined path | `err_12_zero_suffixlen` | [x] |

## Boundary conditions covered in addition to the table

* **Null pointers** — rows 9, 10 (both functions, every pointer parameter),
  verified in forked children so the identical fatal signal is observed.
* **Zero lengths** — rows 2, 12 (empty `path`, empty `outDirName`, `suffixLen == 0`).
* **Oversized lengths** — rows 7, 8 (`SIZE_MAX`, `SIZE_MAX-1`, `SIZE_MAX/2`,
  `SIZE_MAX/2+1`), i.e. one step past the point where the arithmetic wraps and
  one step past the point where `calloc` starts failing.
* **Out-of-range "enum"/narrow-parameter values across FFI** — row 11. There is
  no `enum` in this API, but `extractFilename`'s `char separator` is the
  equivalent narrow-parameter case: C accepts any `int` in the argument
  register, so values with no valid `char` representation (`0x100`, `0x1FF`,
  `-300`, `INT_MIN`) are real inputs. Both sides must truncate identically.
* **Values one step past a documented valid range** — row 3 (`separator == 0`,
  the one byte value that makes `strrchr` behave "backwards" by matching the
  terminator) and row 5 (the signed/unsigned `char` boundary at `0x7F`/`0x80`).
