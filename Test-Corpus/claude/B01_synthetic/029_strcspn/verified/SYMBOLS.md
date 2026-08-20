# SYMBOLS.md — symbol parity between the C `.so` and the Rust `.so`

## How the two libraries are built

`c_src/CMakeLists.txt` only declares `add_executable(driver src/main.c)`, i.e. the
upstream project builds an **executable**, not a library. To be able to compare
the two implementations *through the FFI boundary* the same translation unit is
additionally compiled as a shared object (no change to anything in `c_src/`):

```sh
# C (reference)
gcc -shared -fPIC -o target/cbuild/libc_driver.so c_src/src/main.c
# C (executable, exactly as CMakeLists.txt does it)
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# Rust
cargo build --offline           # -> target/debug/libdriver.so  (crate-type = ["cdylib"])
                                # -> target/debug/driver        (bin, mirrors C main)
```

`src/core.rs` holds the translated logic and is shared by the bin (`src/main.rs`)
and the `cdylib` (`src/lib.rs`), so the library and the executable can never
drift apart.

## Exported (dynamic, defined) symbols

`nm -D --defined-only` on both objects:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | `T driver` | `T driver` | `void driver(const char *s1, const char *s2)` |
| 2 | `main`   | `T main`   | `T main`   | `int main(void)`; reads stdin, prints, returns 0 |

Total exported symbols: **C = 2, Rust = 2**.

### Symbol diff

```
$ comm -3 <(nm -D --defined-only target/cbuild/libc_driver.so | awk '{print $NF}' | sort) \
          <(nm -D --defined-only target/debug/libdriver.so    | awk '{print $NF}' | sort)
(empty)
```

**0 missing symbols.** Nothing was stubbed: both exports are real translations of
the C code (`driver` -> `core::driver`, `main` -> `core::run`).

## Undefined (imported) symbols

The C `.so` imports only libc symbols; the Rust `cdylib` links Rust `std`
statically and imports only libc/loader symbols. Neither library has a
non-libc undefined symbol.

C `.so` undefined:

```
w _ITM_deregisterTMCloneTable      w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5       w __gmon_start__
U fgets@GLIBC_2.2.5                U printf@GLIBC_2.2.5
U stdin@GLIBC_2.2.5                U strcspn@GLIBC_2.2.5
U strlen@GLIBC_2.2.5
```

Rust `.so` undefined: only `libc`/`libgcc_s`/`ld-linux` symbols
(`__libc_start_main` is *not* needed; `memcpy`, `pthread_*`, `dl_iterate_phdr`,
`__cxa_thread_atexit_impl`, … — all resolved by the system libraries the
`cdylib` declares in `DT_NEEDED`).

Verified by:

```sh
ldd -r target/debug/libdriver.so    # no "undefined symbol" lines
```

## C static/internal symbols

`c_src/src/main.c` declares no `static` functions or globals, so there is no
internal symbol that could have been missed.

## Verification status (re-checked by `./verify.sh`)

| gate | result |
|------|--------|
| `nm -D` symbol diff C → Rust | **empty** (0 missing), both in `debug` and `release` |
| `ldd -r` undefined non-libc symbols in the Rust `.so` | **none** |
| Phase B — every `CONFIGS.md` row (35) | **passing** (randomized, fixed seeds) |
| Phase C — every `ERRORS.md` row (27) | **passing** |
| Feature combinations | 1 (the empty set — no `[features]` exist); verified in `debug` + `release` |

Automated by `./verify.sh`, which also cross-checks that every table row names a
test that exists and actually ran, and that no test is missing from the tables.
