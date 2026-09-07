# CONFIGS.md — configuration / valid-input surface table

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from the
branches the C actually takes.

## Axes the C code branches on

| axis | source of the branch | values the C distinguishes |
|---|---|---|
| A. entry point | `include/lib.h` + external linkage in `lib.c` | `extractFilename` (low level), `FIO_createFilename_fromOutDir` (composed) |
| B. `separator` value (only a parameter of `extractFilename`; hard-wired to `'/'` inside `FIO_...`) | `strrchr(path, separator)`, `lib.c:9` | `'/'`, `'\\'`, `'\0'`, ordinary ASCII, high-bit (negative `char`), full `-128..=127` sweep |
| C. separator occurrence in `path` | `if (search == NULL)`, `lib.c:11` | absent / exactly once / many / first byte / last byte / only byte / adjacent runs |
| D. `path` length | implicit in `strrchr`/`strlen` | 0, 1, small, long (> 256 bytes) |
| E. `path` byte content | none — raw bytes, no validation | ASCII, embedded high-bit bytes, arbitrary random non-NUL bytes |
| F. `outDirName` last byte | `if (outDirName[strlen(outDirName)-1] == separator)`, `lib.c:45` | `== '/'` (branch 1: no separator inserted) / `!= '/'` (branch 2: separator inserted) |
| G. `outDirName` length | `strlen(outDirName)`, and the `-1` subscript | 0 (out-of-bounds read, see E8), 1, small, long |
| H. `suffixLen` | allocation size only, `lib.c:38` | 0, 1, small, large-but-allocatable, near-`SIZE_MAX` (wrapping — see E10) |
| I. build-time platform switch | `#if defined(_MSC_VER) \|\| defined(__MINGW32__) \|\| defined(__MSVCRT__)`, `lib.c:27,34` | Windows branch (`'\\'` + second `extractFilename(.., '/')` pass) / **non-Windows branch (`'/'`, single pass)** — the CMake+GCC/Linux reference build takes the non-Windows branch, so that is the only configuration the `.so` under test realises. The Rust mirrors this and the Windows branch is unreachable in both. |
| J. Cargo feature combinations | `translation/Cargo.toml` | the crate declares **no** `[features]` table, so the only combinations are: default, `--no-default-features`. Both are verified. |

## Rows (pruned cross-product of the combinations the C distinguishes)

Every row is exercised with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG in the test harness) against both `.so`s, comparing the full
returned bytes.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `extractFilename` | `separator='/'`, random ASCII paths **containing** `'/'` at random positions, len 1..64 | [x] |
| C2 | `extractFilename` | `separator='/'`, random ASCII paths **without** any `'/'` (NULL branch) — assert same *pointer offset* returned | [x] |
| C3 | `extractFilename` | `separator='/'`, path length 0 (empty string) | [x] |
| C4 | `extractFilename` | `separator='/'`, path length 1: `"/"` and one random non-`'/'` byte | [x] |
| C5 | `extractFilename` | `separator='/'`, `'/'` is the **first** byte only (`"/abc"`) | [x] |
| C6 | `extractFilename` | `separator='/'`, `'/'` is the **last** byte (`"abc/"`) → empty filename | [x] |
| C7 | `extractFilename` | `separator='/'`, **many** `'/'` incl. adjacent runs (`"a//b///c"`) → last occurrence wins | [x] |
| C8 | `extractFilename` | `separator='/'`, long paths (256..1024 random bytes) with random `'/'` density | [x] |
| C9 | `extractFilename` | `separator='\\'` (the Windows separator, reachable here only as an explicit argument) over random paths containing/not containing `'\\'` | [x] |
| C10 | `extractFilename` | `separator='\0'` — matches the terminator; returns `path+strlen+1` | [x] |
| C11 | `extractFilename` | `separator` swept over **every** value `-128..=127`, against a fixed path containing all 255 non-NUL byte values | [x] |
| C12 | `extractFilename` | `separator` high-bit / negative (`0x80..0xFF` as signed `char`) with random paths built from bytes `0x80..0xFF` | [x] |
| C13 | `extractFilename` | `separator` passed as an out-of-`char`-range `int` across FFI (`256`, `-129`, `0x1FF`, `i32::MIN`, `i32::MAX`) | [x] |
| C14 | `extractFilename` | random paths of arbitrary non-NUL bytes (`0x01..0xFF`) × random `separator` from those bytes | [x] |
| C15 | `FIO_createFilename_fromOutDir` | branch F1: `outDirName` **ends with** `'/'`; random paths **with** `'/'`; `suffixLen=0` | [x] |
| C16 | `FIO_createFilename_fromOutDir` | branch F1: `outDirName` ends with `'/'`; random paths **without** `'/'`; `suffixLen=0` | [x] |
| C17 | `FIO_createFilename_fromOutDir` | branch F2: `outDirName` does **not** end with `'/'`; random paths **with** `'/'`; `suffixLen=0` | [x] |
| C18 | `FIO_createFilename_fromOutDir` | branch F2: `outDirName` does not end with `'/'`; random paths **without** `'/'`; `suffixLen=0` | [x] |
| C19 | `FIO_createFilename_fromOutDir` | branch F1/F2 × `suffixLen = 1` (random) | [x] |
| C20 | `FIO_createFilename_fromOutDir` | branch F1/F2 × random `suffixLen` in `2..4096` — verifies the zero-filled tail from `calloc` | [x] |
| C21 | `FIO_createFilename_fromOutDir` | branch F1/F2 × `suffixLen = 1<<20` (large but allocatable) | [x] |
| C22 | `FIO_createFilename_fromOutDir` | `outDirName == "/"` (length 1, F1 with minimal prefix) × random paths | [x] |
| C23 | `FIO_createFilename_fromOutDir` | `outDirName` length 1, non-`'/'` (F2 with minimal prefix) × random paths | [x] |
| C24 | `FIO_createFilename_fromOutDir` | `outDirName` long (256..1024 random bytes) × both F branches × random `suffixLen` | [x] |
| C25 | `FIO_createFilename_fromOutDir` | `path == ""` (empty) × both F branches × random `suffixLen` | [x] |
| C26 | `FIO_createFilename_fromOutDir` | `path` ends with `'/'` (empty filename) × both F branches | [x] |
| C27 | `FIO_createFilename_fromOutDir` | `path == "/"` × both F branches | [x] |
| C28 | `FIO_createFilename_fromOutDir` | `path` with many `'/'` and adjacent runs × both F branches | [x] |
| C29 | `FIO_createFilename_fromOutDir` | `outDirName` with embedded high-bit bytes, last byte `'/'` and last byte non-`'/'` | [x] |
| C30 | `FIO_createFilename_fromOutDir` | `path` with embedded high-bit bytes (incl. `0xFF`, i.e. `-1` as signed `char`) × both F branches | [x] |
| C31 | `FIO_createFilename_fromOutDir` | `outDirName == ""` (length 0 → the `[-1]` out-of-bounds read) with a controlled sentinel byte before the buffer, sentinel `= '/'` and sentinel `!= '/'` | [x] |
| C32 | `FIO_createFilename_fromOutDir` | full random fuzz: random `path`, random `outDirName`, random `suffixLen in 0..64`, 20000 iterations, seeded | [x] |
| C33 | composed pipeline | `extractFilename(path,'/')` result fed as the `path` of `FIO_createFilename_fromOutDir`, and the `FIO_...` result fed back into `extractFilename` — cross-library round-trip (C→Rust and Rust→C mixed) over random inputs | [x] |
| C34 | build config | every row above re-run under `--no-default-features` (the crate declares no features, so this is the complete feature space) | [x] |
| C35 | build config | every row above re-run against the **debug** Rust `.so` as well as the **release** `.so` (catches `overflow-checks` divergence in the allocation-size arithmetic) | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/lib.c)` —
there is **no** `add_executable`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]` and no `src/main.rs`. There is
no driver binary, so the "compare binary stdout byte-for-byte" gate is
**not applicable**; it is satisfied vacuously.
