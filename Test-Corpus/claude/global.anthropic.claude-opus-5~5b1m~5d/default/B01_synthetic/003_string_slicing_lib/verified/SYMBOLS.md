# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libString_Slice.so          (cmake, -DCMAKE_POSITION_INDEPENDENT_CODE=ON)
Rust: translation/target/release/libString_Slice.so   (cargo build --release, crate-type = cdylib)
```

## C source inventory (completeness check)

The whole library is two files; every function defined in them is accounted for:

| C file | functions defined | translated in Rust? |
|--------|-------------------|---------------------|
| `c_src/src/slicing.c` | `slice` | yes — `translation/src/lib.rs::slice` |
| `c_src/include/slicing.h` | (declaration only: `slice`) | n/a |

No C module is missing from the translation; there is nothing to stub and
nothing left to translate.

## Defined dynamic symbols (`nm -D --defined-only`)

| symbol | C `.so` | Rust `.so` | status |
|--------|---------|------------|--------|
| `slice` | `T` | `T` | present in both — OK |

Symbol diff (`comm -3` of the two sorted defined-symbol lists): **empty**.

## Undefined dynamic symbols (`nm -D -u`)

C `.so`:

| symbol | kind |
|--------|------|
| `_ITM_deregisterTMCloneTable` | weak, toolchain |
| `_ITM_registerTMCloneTable` | weak, toolchain |
| `__cxa_finalize@GLIBC_2.2.5` | weak, libc |
| `__gmon_start__` | weak, toolchain |
| `printf@GLIBC_2.2.5` | libc |
| `puts@GLIBC_2.2.5` | libc (GCC rewrites `printf("literal\n")` → `puts("literal")`) |
| `strlen@GLIBC_2.2.5` | libc |

Rust `.so` adds only libc / libgcc-unwinder / Rust-runtime imports
(`_Unwind_*`, `malloc`, `free`, `memcpy`, `abort`, `write`, `dl_iterate_phdr`,
`pthread_key_*`, `stat64`, …). It imports the same three functional libc
symbols the C object needs (`printf`, `strlen`, and `puts` via the Rust
runtime), plus standard-library support symbols.

**0 missing symbols, 0 undefined non-libc/non-runtime symbols in the Rust
`.so`.**

## Reproduce

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
diff <(nm -D --defined-only ../c_src/build/libString_Slice.so | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/release/libString_Slice.so | awk '{print $NF}' | sort)
```

`tests/symbol_parity.rs` performs this diff automatically as part of the test
suite (Phase D).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
build configurations are the default (empty) feature set and
`--no-default-features`, which are identical. Both are exercised by
`./run_all.sh`.
