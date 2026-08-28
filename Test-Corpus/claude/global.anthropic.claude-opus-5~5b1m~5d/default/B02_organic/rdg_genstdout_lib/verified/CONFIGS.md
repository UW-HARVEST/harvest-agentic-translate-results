# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`. This is the
mirror of `ERRORS.md`: it enumerates the **valid** input space, as the
cross-product of the axes the C code actually branches on.

## Axes the C code branches on

Enumerated from every `if` / `#if` / library call in the source:

| axis | values the C distinguishes | evidence |
|---|---|---|
| **A. entry point** | `extractFilename` (low-level, *not* in `lib.h` but exported), `FIO_createFilename_fromOutDir` (the `lib.h` wrapper that calls it) | `lib.c:8`, `lib.c:21` |
| **B. separator argument** (`extractFilename` only — it is a *parameter*, so the low-level entry point can be driven with any byte, while the wrapper hard-codes one) | `'/'`; `'\\'`; `0`; any other ASCII byte; high/negative bytes `0x80..0xFF` | `lib.c:10` param `char separator` |
| **C. `strrchr` outcome** | separator **absent** → `return path`; separator **present** → `return search+1` | `lib.c:11` vs `lib.c:12` |
| **D. platform separator** (compile-time) | `'\\'` + extra `'/'` pass on `_MSC_VER`/`__MINGW32__`/`__MSVCRT__`; `'/'` and a single pass otherwise | `lib.c:27-31`, `lib.c:34-36` |
| **E. `outDirName` trailing byte** | ends **with** separator → dir+name concatenated directly; does **not** end with separator → separator byte inserted | `lib.c:45` vs `lib.c:47` |
| **F. `path` shape** | no separator; one separator; many separators; separator first; separator last; separator only; empty; long | `lib.c:10` via `strrchr` |
| **G. `outDirName` shape** | empty; single byte; single separator `"/"`; many trailing separators; long; contains inner separators | `lib.c:38,44,45` via `strlen` |
| **H. `suffixLen`** | `0`; small; large-but-allocatable; wrapping (see `ERRORS.md` rows 7–8) | `lib.c:38` |
| **I. byte content** | ASCII; embedded high bytes `0x80..0xFF` (UTF-8 / Latin-1 path names); every byte value except `0` | `strlen`/`memcpy` are byte-oriented |

**Axis D is fixed to the non-Windows branch** on this Linux host: both the C
`.so` and the Rust `.so` are compiled for the same target, so the C takes the
`'/'` path and the Rust's `#[cfg(not(windows))]` / `cfg!(windows) == false` path.
The tests assert the two agree, which is what "byte-identical on this target"
means. The Windows branch is unreachable in both and is not separately testable
without a Windows toolchain (documented, not silently skipped).

**Feature combinations:** `translation/Cargo.toml` has **no `[features]`
section**, so there is exactly one build configuration; every row below is run
under it (and under `--no-default-features`, which is identical). See
`check_feature_combos.sh`.

## Rows (pruned cross-product of the axes the code actually distinguishes)

Every row is exercised with **many randomized inputs** from a fixed-seed PRNG
(seed `0x5EED_1234_ABCD_0001`), not a single hand-picked value, and the C and
Rust results are compared byte-for-byte through their `.so` exports.

### `extractFilename` — the low-level entry point, driven directly

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|----|
| 1 | `extractFilename` | sep `'/'`, path **without** any separator (axis C=absent), randomized ASCII, len 0..64 | [x] |
| 2 | `extractFilename` | sep `'/'`, path with **exactly one** separator, randomized position/len | [x] |
| 3 | `extractFilename` | sep `'/'`, path with **many** separators, randomized count/positions | [x] |
| 4 | `extractFilename` | sep `'/'`, separator is the **first** byte (`"/abc"`) | [x] |
| 5 | `extractFilename` | sep `'/'`, separator is the **last** byte (`"abc/"`) → returns empty tail | [x] |
| 6 | `extractFilename` | sep `'/'`, path is **only** separators (`"/"`, `"//"`, `"///"`, …) | [x] |
| 7 | `extractFilename` | sep `'\\'` (the Windows separator, exercised as a *value* on Linux), randomized paths containing `\` and `/` | [x] |
| 8 | `extractFilename` | sep = a **random arbitrary byte** `1..=255` each iteration, over randomized full-byte-range paths (axis B×I) | [x] |
| 9 | `extractFilename` | sep = **high/negative** byte `0x80..=0xFF`, paths containing high bytes (signed-`char` boundary, axis B×I) | [x] |
| 10 | `extractFilename` | sep `'/'`, **long** paths (256..4096 bytes) with randomized separator density | [x] |
| 11 | `extractFilename` | sep `'/'`, path bytes drawn from `{'/', 'a'}` only — maximal separator density / adversarial for a backward scan | [x] |

### `FIO_createFilename_fromOutDir` — the `lib.h` entry point, full pipeline

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|----|
| 12 | `FIO_createFilename_fromOutDir` | `outDirName` **not** ending in `/` (axis E=insert), `path` without separator, `suffixLen=0` | [x] |
| 13 | `FIO_createFilename_fromOutDir` | `outDirName` **not** ending in `/`, `path` **with** separators, randomized `suffixLen` 0..64 | [x] |
| 14 | `FIO_createFilename_fromOutDir` | `outDirName` **ending in `/`** (axis E=concat), `path` without separator, `suffixLen=0` | [x] |
| 15 | `FIO_createFilename_fromOutDir` | `outDirName` **ending in `/`**, `path` with separators, randomized `suffixLen` | [x] |
| 16 | `FIO_createFilename_fromOutDir` | `outDirName` ending in **multiple** `/` (`"a//"`, `"a///"`) | [x] |
| 17 | `FIO_createFilename_fromOutDir` | `outDirName` == `"/"` exactly (single byte that *is* the separator) | [x] |
| 18 | `FIO_createFilename_fromOutDir` | `outDirName` == single non-separator byte (shortest non-empty, `[-1]` read not triggered) | [x] |
| 19 | `FIO_createFilename_fromOutDir` | `path` == `""` (empty filename) × both axis-E branches | [x] |
| 20 | `FIO_createFilename_fromOutDir` | `path` ending in `/` → `filenameStart` is empty × both axis-E branches | [x] |
| 21 | `FIO_createFilename_fromOutDir` | `outDirName` containing **inner** separators (`"a/b/c"`, `"a/b/c/"`) × both axis-E branches | [x] |
| 22 | `FIO_createFilename_fromOutDir` | randomized `suffixLen` up to 4096 — asserts the trailing zero-fill from `calloc` matches byte-for-byte, not just the prefix | [x] |
| 23 | `FIO_createFilename_fromOutDir` | **long** `outDirName` and `path` (256..2048 bytes each), randomized | [x] |
| 24 | `FIO_createFilename_fromOutDir` | full-byte-range content (`0x01..=0xFF`, high bytes) in both `outDirName` and `path` | [x] |
| 25 | `FIO_createFilename_fromOutDir` | fully randomized fuzz over **all** axes at once (dir shape × path shape × suffixLen × byte range), 20 000 iterations | [x] |

### Composed / cross-entry-point rows

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|----|
| 26 | `extractFilename` → `FIO_createFilename_fromOutDir` | the composed pipeline: assert the tail that `extractFilename(path,'/')` returns is exactly the tail `FIO_createFilename_fromOutDir` appends, in C and in Rust alike (catches a divergence that is invisible to per-function tests) | [x] |
| 27 | both, cross-linked | call the **C** `extractFilename` on a buffer, then hand the *returned interior pointer* to the **Rust** `FIO_createFilename_fromOutDir` and vice-versa — verifies the two `.so`s agree on interior-pointer semantics | [x] |
| 28 | `FIO_createFilename_fromOutDir` | returned buffer is released with libc `free()` after each call in both libraries — verifies both allocate from the *same* allocator (`calloc`), which is part of the contract | [x] |
