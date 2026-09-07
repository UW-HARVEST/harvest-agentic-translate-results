# CONFIGS.md — Phase B configuration-surface table

## Axes actually branched on by the C code

Derived from `c_src/src/lib.c` + `c_src/include/lib.h`:

**Public entry points** (the complete set — note the low-level one,
`extractFilename`, is *not* in the header but *is* exported and *is* the
building block of the wrapper):

* L0 `extractFilename(const char* path, char separator)` — lowest level
* L1 `FIO_createFilename_fromOutDir(const char* path, const char* outDirName, size_t suffixLen)` — wrapper, calls L0

**Runtime options / flags**: none. The only "mode" is compile-time:
`#if defined(_MSC_VER) || defined(__MINGW32__) || defined(__MSVCRT__)`, which
selects `separator = '\\'` and adds a *second* `extractFilename(..., '/')` pass.
On this (Linux/ELF) target both C and Rust take the non-Windows arm
(`separator = '/'`, single pass); the Rust mirrors this with
`#[cfg(windows)]` / `cfg!(windows)`. Rows C1–C8 drive L0 with an *arbitrary*
separator, which covers the `'\\'` arm's behaviour of the shared helper.

**Branches the code takes:**

| location | branch |
|----------|--------|
| `lib.c:11` | `strrchr` result `NULL` vs non-`NULL` |
| `lib.c:27/34` | Windows vs POSIX separator (compile-time) |
| `lib.c:38`  | `calloc` size = `strlen(outDirName)+1+strlen(filenameStart)+suffixLen+1` (value-dependent, `size_t` arithmetic) |
| `lib.c:39`  | `result == NULL` vs not |
| `lib.c:45`  | `outDirName[strlen(outDirName)-1] == separator` vs not |

**Input shapes special-cased:** separator absent / present once / present many /
at the very start / at the very end / `path` all separators / `path` empty /
`separator == '\0'` / separator is a high-bit (negative `char`) byte /
`outDirName` empty / `outDirName` ending in separator / `outDirName` not ending
in separator / `outDirName` == `"/"` (single separator) / `suffixLen` 0 / small /
large-but-allocatable.

**Observable outputs compared byte-for-byte:**

* L0: the *offset* of the returned pointer relative to `path` (the pointer
  identity is the whole result).
* L1: the **entire `calloc`ed buffer**, all
  `strlen(outDirName)+1+strlen(filenameStart)+suffixLen+1` bytes — not just the
  NUL-terminated prefix. `calloc` zeroes the block, so the trailing padding is
  part of the contract and would expose any `malloc`-instead-of-`calloc`
  or off-by-one length bug.

## Rows

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_5678_9ABC`, in-crate xorshift64* PRNG) driving both `.so`s, except
where the row pins a single degenerate shape.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C1 | L0 `extractFilename` | random ASCII `path` (len 0..64), `separator='/'` **present at least once** at a random interior index | [x] |
| C2 | L0 | random `path` **guaranteed not to contain** `separator` → `strrchr`→`NULL` fallback arm | [x] |
| C3 | L0 | random `path` with `separator` as the **last** byte → returns pointer to the NUL (empty filename) | [x] |
| C4 | L0 | random `path` with `separator` as the **first** byte only | [x] |
| C5 | L0 | random `path` with **many** (2..12) occurrences of `separator` → must pick the *last* | [x] |
| C6 | L0 | `separator == '\0'` over random paths → always matches the terminator, returns `path+len+1` | [x] |
| C7 | L0 | random **high-bit** separator `0x80..0xFF` (negative `signed char`) over random byte strings containing that byte | [x] |
| C8 | L0 | `separator == '\\'` (the Windows arm's separator) over random paths mixing `'/'` and `'\\'` | [x] |
| C9 | L0 | random full-byte-range `path` (`0x01..0xFF`, any bytes) × random full-range `separator` — pure property sweep | [x] |
| C10 | L1 `FIO_createFilename_fromOutDir` | `outDirName` **not** ending in `/`, `path` with separators, `suffixLen = 0` | [x] |
| C11 | L1 | `outDirName` **ending** in `/` (the `lib.c:45` true arm), `path` with separators, `suffixLen = 0` | [x] |
| C12 | L1 | `outDirName` **not** ending in `/`, `path` **without** any separator, `suffixLen = 0` | [x] |
| C13 | L1 | `outDirName` **ending** in `/`, `path` **without** any separator, `suffixLen = 0` | [x] |
| C14 | L1 | `outDirName == "/"` (single separator, so `outDirLen == 1` and the true arm), random `path` | [x] |
| C15 | L1 | `outDirName` not ending in `/`, random `path`, `suffixLen` random **small** 1..16 → tests the trailing zero padding | [x] |
| C16 | L1 | `outDirName` ending in `/`, random `path`, `suffixLen` random **small** 1..16 | [x] |
| C17 | L1 | random `outDirName`/`path`, `suffixLen` **large but allocatable** (1 MiB .. 8 MiB) → whole multi-MiB zero-padded buffer compared | [x] |
| C18 | L1 | `path` with a **trailing** separator → `filenameLen == 0`, both arms of `lib.c:45` | [x] |
| C19 | L1 | `path` that is **all separators** (`"/"`, `"//"`, `"///"`) × both `outDirName` arms | [x] |
| C20 | L1 | `path` **empty string** × both `outDirName` arms × `suffixLen` 0 and 5 | [x] |
| C21 | L1 | `outDirName` **empty string** with a *controlled* preceding byte (buffer `"X\0"`, pointer + 1) so the `outDirName[-1]` read is deterministic — separately with preceding byte `'/'` (true arm) and `'X'` (false arm) | [x] |
| C22 | L1 | deep multi-component `outDirName` (`"a/b/c/d/e"`) and deep `path`, random depths 1..8, both arms | [x] |
| C23 | L1 | `outDirName` containing **non-ASCII / high-bit** bytes and `path` containing high-bit bytes (UTF-8 and invalid-UTF-8 byte strings) — the C is byte-oriented, the Rust must not assume UTF-8 | [x] |
| C24 | L1 | `outDirName` whose last byte is `'\\'` (not the POSIX separator → false arm) — guards against a Rust build that picked the Windows separator | [x] |
| C25 | L0 + L1 **composed** | for the same random `path`, assert `L1(path, dir, n)` ends with exactly the bytes `L0(path, '/')` returns, and that C and Rust agree on both simultaneously (pipeline consistency, invisible to per-function tests) | [x] |
| C26 | L1 | `suffixLen` boundary shapes: `0`, `1`, and the exact value making `size` a power of two (`2^12`, `2^16`) | [x] |
| C27 | L1 | repeated invocation (200×) with the same arguments → both must return fresh, independently-`free()`-able, identically-contented buffers (no shared static state) | [x] |

## Suite sensitivity (mutation check)

To prove these rows actually constrain the Rust, seven mutations were injected
into `src/lib.rs`, each rebuilt and re-run; **all seven were caught**:

| mutation | detected by |
|----------|-------------|
| M1 invert `outDirName[len-1] == separator` | 18 failing assertions (C10–C24, E7, E13, E14) |
| M2 `calloc` size one byte short | 23 failing assertions (trailing-zero-padding checks) |
| M3 `extractFilename` returns `NULL` instead of `path` | E1/E2 + harness `SIGSEGV` (test binary aborts) |
| M4 use `'\\'` as the POSIX separator | 18 failing assertions (C24 in particular) |
| M5 `exit(31)` instead of `exit(30)` | 3 failing assertions (E5, E5b, E11a) |
| M6 `search+0` instead of `search+1` | 23 failing assertions (C1–C9, E3) |
| M7 `malloc` instead of `calloc` | 18 failing assertions (zero-padding of the `suffixLen` reservation) |

A stale-artifact guard in `tests/harness/mod.rs` refuses to run if either `.so`
is older than its sources — `cargo test` alone does **not** rebuild a
`crate-type = ["cdylib"]` artifact, which would otherwise produce false PASSes.

## No binary target

Neither `c_src/CMakeLists.txt` (only `add_library(driver SHARED …)`, no
`add_executable`, no `main()`) nor `translation/Cargo.toml` (no `[[bin]]`, no
`src/main.rs`) builds an executable, so the "compare C and Rust stdout"
gate is not applicable. The one place the library writes to a stream —
`fprintf(stderr, …)` on the `calloc`-failure path — *is* compared
byte-for-byte, in `e5b_alloc_failure_stderr_is_byte_identical`.
