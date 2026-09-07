# SYMBOLS.md — public ABI surface parity

Derived mechanically from `nm -D` on both shared libraries.

- C   `.so`: `c_src/build/libharvest-work-rEe7nL.so`
- Rust`.so`: `translation/target/release/libgaussian_kernel_lib.so`

## C translation unit inventory (completeness check)

`c_src/CMakeLists.txt` lists exactly one source file:

```
add_library(${project_name} SHARED src/lib.c)
```

`c_src/include/lib.h` declares exactly one prototype:

```c
void gaussian_kernel(float *dest, int size, float radius);
```

So there is **no untranslated C module**: `c_src/src/lib.c` (28 lines, one
function) is fully represented by `translation/src/lib.rs`. No macro-generated
symbols exist (the only macro-ish construct in the C is the inline
`((v) > (0)) ? (v) : (0)` clamp, which generates no symbol).

## Defined (exported) dynamic symbols

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|-----------|---------------|--------|
| 1 | `gaussian_kernel` | `T` (0x1109) | `T` (0x11750) | MATCH |

`nm -D --defined-only` output, verbatim:

```
C:    0000000000001109 T gaussian_kernel
Rust: 0000000000011750 T gaussian_kernel
```

**Symbol diff (C-defined not exported by Rust): EMPTY.**

## Undefined (imported) symbols

The C library imports exactly one non-weak, non-libc-startup symbol:

| symbol | C | Rust | note |
|--------|---|------|------|
| `expf@GLIBC_2.27` | `U` | `U` | Rust deliberately imports the *same platform libm* `expf` rather than using `f32::exp`, so results are bit-identical. |

Rust additionally imports the standard Rust runtime set (`_Unwind_*`,
`malloc`/`free`/`realloc`/`calloc`/`posix_memalign`, `memcpy`/`memmove`/
`memset`/`bcmp`, `abort`, `dl_iterate_phdr`, `open64`/`read`/`close`/`lseek64`/
`stat64`/`fstat64`/`statx`/`readlink`/`realpath`/`getcwd`/`write`/`writev`,
`getenv`, `__errno_location`, `syscall`, `pthread_key_*`,
`pthread_setspecific`, `__tls_get_addr`, `__cxa_thread_atexit_impl`, `gettid`).
These are all libc / libgcc unwinder symbols pulled in by `std` and the panic
machinery — **0 missing or undefined non-libc symbols**.

## Gate

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Every C-exported symbol is exported by Rust with the exact same name.

Verified by `scripts/verify.sh` step 4, independently reproducible with:

```
nm -D --defined-only c_src/build/libharvest-work-rEe7nL.so             | awk '{print $NF}' | sort -u > /tmp/c.txt
nm -D --defined-only translation/target/release/libgaussian_kernel_lib.so | awk '{print $NF}' | sort -u > /tmp/r.txt
comm -23 /tmp/c.txt /tmp/r.txt     # -> empty
ldd -r translation/target/release/libgaussian_kernel_lib.so | grep -i undefined   # -> none
```

Confirmed empty under all four build configurations
(release/debug × default/`--no-default-features`).

No `unimplemented!`, `todo!`, or stub exists in `translation/src/`
(`grep -rniE 'unimplemented|todo!|unreachable!|panic!\(|stub' translation/src/`
→ no matches), so the single exported symbol is a real translation, not a
placeholder.

## Link-level parity

| | C `.so` | Rust `.so` |
|---|---|---|
| `DT_NEEDED` | `libm.so.6`, `libc.so.6` | `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `ld-linux-x86-64.so.2` |
| `ldd -r` unresolved | none | none |

Both link `libm.so.6`, so the Rust `.so` resolves its `expf` import standalone,
exactly as the C one does.


## Build-flag robustness (beyond the required gate)

The ground truth is `c_src/CMakeLists.txt` with its default flags (no `-O`).
To prove the Rust is not accidentally matching one particular gcc
constant-folding choice for `s2 = 1.0f / expf(1.6f*1.6f*2.25f)`, the same C
source is also compiled (read-only, outside `c_src/`) at each optimization
level and the whole suite re-run against it — `scripts/verify.sh` step 6:

| C build | result |
|---|---|
| CMake default (ground truth) | 44/44 pass |
| `gcc -O0` | 44/44 pass |
| `gcc -O1` | 44/44 pass |
| `gcc -O2` | 44/44 pass |
| `gcc -O3` | 44/44 pass |
| `gcc -Os` | 44/44 pass |
| `gcc -O3 -ffast-math` | 16/44 — **expected divergence, informational only** |

`-ffast-math` is not the ground-truth build. It licenses reassociation and a
reciprocal approximation for `1.0f / sum`, producing 1-ULP differences such as
`C = 0x3eaaaaee` vs `Rust = 0x3eaaaaed` (size 3, radius 537.68). All standard-
conforming C builds agree with the Rust bit-for-bit.
