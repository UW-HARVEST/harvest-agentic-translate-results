# SYMBOLS.md — Phase A: exported-surface map

Derived mechanically from `nm -D` on both shared objects. Nothing here is
inferred from what looks "important"; every dynamic symbol of the C `.so` is
listed.

## Build commands used

```sh
# C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-Ptband.so   (project name == parent dir name)

# Rust translation
cd translation && cargo build --release
# -> translation/target/release/libto_barycentric_lib.so
```

`c_src/CMakeLists.txt` sets **no** `CMAKE_BUILD_TYPE` and **no** optimisation
flags, so the reference object is compiled at `-O0`. This matters: it fixes the
register allocation, and therefore the SSE *destination* operand of every scalar
float op — which in turn fixes which NaN payload survives (see `CONFIGS.md`
rows C-*).

## C source inventory (completeness check)

`c_src/CMakeLists.txt` names exactly one translation unit:

| C file | lines | translated in Rust? |
|--------|-------|---------------------|
| `c_src/src/lib.c`      | 29 | yes — `translation/src/lib.rs` |
| `c_src/include/lib.h`  | 5  | yes — `lm_vec2` `#[repr(C)]` struct + fn signature |

No other `.c` / `.h` file exists under `c_src/`, so no module was skipped:

```sh
$ find c_src -name '*.c' -o -name '*.h'
c_src/include/lib.h
c_src/src/lib.c
```

## Defined (exported) dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `to_barycentric` | `T` @ `0x11a3` | `T` @ `0x11c40` | `#[unsafe(no_mangle)] pub extern "C" fn` |

**Symbol diff (C defined − Rust defined): EMPTY.**

```sh
$ diff <(nm -D --defined-only c_src/build/libharvest-work-Ptband.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libto_barycentric_lib.so | awk '{print $NF}' | sort)
# (no output)
```

### Deliberately *not* exported

These three functions in `c_src/src/lib.c` are declared `static`, i.e. they have
internal linkage and are absent from the C `.so`'s dynamic symbol table. The
Rust translation reproduces them as private `fn`s (so the arithmetic *and its
evaluation order* are identical) and likewise does not export them. Exporting
them would be a **divergence**, not a fix.

| C declaration | linkage | Rust counterpart |
|---------------|---------|------------------|
| `static lm_vec2 lm_v2(float x, float y)`          | internal | `fn lm_v2` (private) |
| `static lm_vec2 lm_sub2(lm_vec2 a, lm_vec2 b)`    | internal | `fn lm_sub2` (private) |
| `static float   lm_dot2(lm_vec2 a, lm_vec2 b)`    | internal | `fn lm_dot2` (private) |

Confirmed absent from both `.so` dynamic tables:

```sh
$ nm -D c_src/build/libharvest-work-Ptband.so | grep -cE 'lm_v2|lm_sub2|lm_dot2'
0
$ nm -D translation/target/release/libto_barycentric_lib.so | grep -cE 'lm_v2|lm_sub2|lm_dot2'
0
```

## Undefined / imported symbols

C `.so` (all weak, all toolchain glue — no real imports):

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

Rust `.so` undefined symbols are the Rust `std` + panic-unwind runtime's libc
and libgcc dependencies only (`_Unwind_*@GCC_*`, `malloc`, `memcpy`, `mmap64`,
`dl_iterate_phdr`, …). **0 undefined non-libc / non-toolchain symbols**, i.e.
nothing from the translated library itself is left dangling:

```sh
$ nm -D --undefined-only translation/target/release/libto_barycentric_lib.so \
    | awk '{print $NF}' | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_|_Unwind_|__errno_location|__tls_get_addr|gettid|statx)' \
    | grep -vxE 'abort|bcmp|calloc|close|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev'
# (no output)
```

## ABI notes verified by the differential tests

* `lm_vec2` is an 8-byte, 2×`float` aggregate → one SSE eightbyte under the
  x86-64 SysV ABI, so all four parameters arrive in `xmm0..xmm3` and the result
  is returned packed in `xmm0`. `#[repr(C)]` + `extern "C"` reproduces this.
  The differential tests call *both* `.so`s through the identical
  `unsafe extern "C" fn(Vec2, Vec2, Vec2, Vec2) -> Vec2` pointer type loaded
  with `libloading`, so a struct-passing mismatch would show up immediately.
* Results are compared as raw `u32` bit patterns (`f32::to_bits`), never with
  `==`, so `-0.0` vs `+0.0` and differing NaN payloads are both caught.

## Cargo feature matrix

`translation/Cargo.toml` declares **no** `[features]` table, so the only
configurations are the (empty) default set. Verified:

```sh
$ grep -c '^\[features\]' translation/Cargo.toml
0
```

`--no-default-features` and `--all-features` are therefore identical to the
default build; the test driver (`check_all_features.sh`) still runs all three
explicitly.
