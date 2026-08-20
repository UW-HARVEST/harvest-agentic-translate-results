# SYMBOLS.md — public symbol parity (Phase A / Phase D)

## Build commands used

C shared object:

```sh
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so    (project name = parent dir name)
```

Rust shared object:

```sh
cd translated_rust && cargo build            # -> target/debug/libgaussian_kernel_lib.so
```

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** → exactly **one** valid feature
  combination: the default/empty one. `--no-default-features` and the plain
  default build are therefore identical, and both were exercised.
* `c_src/CMakeLists.txt` has **no options, no `#ifdef`-driven flags, no
  `add_definitions`, no conditional sources**. It compiles a single translation
  unit (`src/lib.c`) into one `SHARED` library and links `m`. There is exactly
  **one** C build configuration.
* `c_src/src/lib.c` contains **no preprocessor conditionals at all** (verified:
  `grep -nE '#(if|ifdef|ifndef|else|elif|endif|define)' c_src/src/lib.c` → no
  matches other than `#include`), so there is no compile-time variability to
  enumerate.

Enumerated feature combinations (complete):

| # | cargo invocation | notes |
|---|------------------|-------|
| 1 | `cargo test --no-default-features` | the only combination; identical to the default build |

## `nm -D --defined-only` on the C `.so`

```
0000000000001109 T gaussian_kernel
```

That is the complete exported surface of the C library (one symbol). It matches
the single declaration in the public header `c_src/include/lib.h`:

```c
void gaussian_kernel(float *dest, int size, float radius);
```

## Symbol parity table

| # | C symbol (`nm -D`) | exported by Rust `.so`? | Rust item |
|---|--------------------|-------------------------|-----------|
| 1 | `gaussian_kernel` (T) | **yes** — `T gaussian_kernel` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn gaussian_kernel` in `src/lib.rs` |

Weak/loader-provided symbols in the C `.so` that are not part of the API and are
supplied by the C runtime / linker rather than the library
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`) are excluded, as is the libc import `expf@GLIBC_2.27`.

**Missing symbols: 0.** No symbol required a new `#[no_mangle]` wrapper and no C
source file was left untranslated: the whole library is the single function
above, and `c_src/src/lib.c` (28 lines) is fully represented in `src/lib.rs`.

## Undefined-symbol audit of the Rust `.so`

`nm -D --undefined-only target/debug/libgaussian_kernel_lib.so` lists **only**
libc/glibc and `libgcc` unwinder imports — `expf@GLIBC_2.27`,
`malloc`/`free`/`calloc`/`realloc`/`posix_memalign`, `memcpy`/`memmove`/`memset`/`bcmp`/`strlen`,
`abort`, `__errno_location`, `getenv`, `getcwd`, `realpath`, `readlink`,
`open64`/`close`/`read`/`write`/`writev`/`lseek64`/`stat64`/`fstat64`/`statx`,
`mmap64`/`munmap`, `dl_iterate_phdr`, `syscall`, `gettid`,
`pthread_key_*`/`pthread_setspecific`/`__tls_get_addr`/`__cxa_thread_atexit_impl`,
and the `_Unwind_*` family (Rust std panic machinery). All of these are resolved
by the platform at load time.

**0 missing / unresolvable non-libc symbols.**

Crucially, the Rust `.so` imports the *same* `expf@GLIBC_2.27` that the C `.so`
imports, because `src/lib.rs` declares `extern "C" { fn expf(x: f32) -> f32; }`
instead of using `f32::exp()`. This is what makes the results bit-identical:
`objdump -d` on the C `.so` shows **both** `expf` uses compiled to runtime
`call expf@plt` (no MPFR constant folding of `expf(sigma*sigma*tetha)`), so both
libraries evaluate the exponential with the identical glibc implementation.

## Verification commands

```sh
diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/debug/libgaussian_kernel_lib.so | awk '{print $3}' \
       | grep -x gaussian_kernel | sort)
# -> empty (every C symbol is exported by the Rust .so under the exact same name)
```

The automated form of this check is the test
`phase_d_symbol_parity::c_symbols_are_all_exported_by_rust` in
`tests/differential.rs`, which shells out to `nm` and fails if the diff is
non-empty.

## How to re-run the whole verification

```sh
./run_verification.sh    # builds C .so, then check+build+test for every
                         # feature combination in both debug and release,
                         # then diffs `nm -D` symbol sets
```

### Two harness pitfalls this suite guards against

1. **`cargo test` does not rebuild a `cdylib`.** Integration tests never link
   the `cdylib`, so cargo happily leaves a stale
   `target/<profile>/libgaussian_kernel_lib.so` in place and the differential
   tests would then validate an obsolete binary. `run_verification.sh` always
   runs `cargo build` first, and `tests/differential.rs::assert_not_stale`
   *fails the run* if either `.so` is older than its sources.
2. **Test power was validated by mutation.** Six deliberate mutations were
   injected into `src/lib.rs` (normalisation loop `r <= size`; clamp letting NaN
   through; taps loop `r < hsize`; `sigma = 1.5999999`; floor instead of
   truncating division for `hsize`; `s2` rounded through `f64`). The suite caught
   5 of them (19–36 failing tests each). The sixth (`s2` via `f64`) is
   semantically equivalent for this one constant — double rounding yields the
   identical `f32` — so no test can distinguish it.

### C optimization-level robustness

The suite was additionally re-run against C libraries built with
`-DCMAKE_BUILD_TYPE=Release`, `RelWithDebInfo` and `MinSizeRel` (out-of-tree, in
`$TMPDIR`, leaving `c_src/` untouched) via the `HARVEST_C_SO` override. At `-O2`
GCC constant-folds `expf(sigma*sigma*tetha)` with MPFR, leaving only one runtime
`expf` call — yet all 40 tests still pass bit-for-bit, so the Rust `s2` matches
both the folded and the runtime-computed constant.
