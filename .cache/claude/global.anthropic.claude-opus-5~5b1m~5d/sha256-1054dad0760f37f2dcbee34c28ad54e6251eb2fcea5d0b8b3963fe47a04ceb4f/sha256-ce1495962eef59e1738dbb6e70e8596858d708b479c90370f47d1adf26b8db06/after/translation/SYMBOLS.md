# SYMBOLS.md — Phase A symbol surface map

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C `.so` exported (defined, dynamic) symbols

`nm -D --defined-only c_src/build/libdriver.so`

```
0000000000001173 T driver
```

Total: **1** exported symbol.

## Rust `.so` exported (defined, dynamic) symbols

`nm -D --defined-only translation/target/release/libdriver.so`

```
0000000000011720 T driver
```

Total: **1** exported symbol.

## Parity table

| # | C symbol | type | exported by Rust `.so`? | notes |
|---|----------|------|-------------------------|-------|
| 1 | `driver` | `T` (global text) | YES — `T driver` | `#[unsafe(no_mangle)] pub extern "C" fn driver(floors: c_int)` |

### Missing symbols

**None.** The symbol diff (C-exported minus Rust-exported) is EMPTY.

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(no output)
```

### Non-exported C entities (correctly private in Rust)

These are `static`/file-local in `c_src/src/driver.c`, so they are NOT part of the
ABI and must NOT be exported by the Rust `.so`:

| C entity | kind | Rust counterpart | visibility |
|----------|------|------------------|------------|
| `print_hex(unsigned char *p, int len)` | `static` function | `fn print_hex(p: *const c_uchar, len: c_int)` | private (not exported) — correct |
| `house_t { int floors; int bedrooms; double bathrooms; }` | file-local typedef | `#[repr(C)] struct HouseT` | private type — correct |

Confirmed: neither `.so` exports `print_hex` or any `house_t` related symbol.

## Undefined (imported) symbols

The C `.so` imports:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
U printf@GLIBC_2.2.5
U putchar@GLIBC_2.2.5
```

Note: GCC lowered the `printf("\n")` call into `putchar('\n')`, hence the
`putchar` import. LLVM performs the same lowering for the Rust translation, so the
Rust `.so` also imports both `printf` and `putchar`.

The Rust `.so` imports the same `printf`/`putchar` plus the usual Rust `std`
runtime imports (libc: `malloc`, `free`, `memcpy`, `write`, `pthread_*`, …, and
`_Unwind_*` from libgcc). **0 missing/undefined non-libc symbols** — every
undefined symbol in the Rust `.so` is satisfied by glibc / libgcc, which are
present at load time (verified: `libloading` loads the Rust `.so` successfully
and resolves `driver`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the only
feature configuration that exists is the default (empty) one. `--no-default-features`
and the default build are the same code. Verified by
`tests/feature_matrix.rs` / `check_features.sh`.
