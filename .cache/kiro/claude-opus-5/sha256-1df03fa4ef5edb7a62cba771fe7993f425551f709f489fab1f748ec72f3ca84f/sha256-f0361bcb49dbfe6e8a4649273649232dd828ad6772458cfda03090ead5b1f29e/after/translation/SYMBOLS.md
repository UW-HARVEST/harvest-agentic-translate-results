# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-CJhSn4.so

cd translation && cargo build --release
# -> translation/target/release/libtritanopia_lib.so
```

The C project (`c_src/CMakeLists.txt`) builds **only** a `SHARED` library from
`src/lib.c`. There is **no binary / driver executable**, so the "compare C and
Rust stdout" clause of the completion gate does not apply.

## Defined dynamic symbols

`nm -D --defined-only <so> | awk '{print $3}' | sort`

| # | C symbol (`nm -D`) | present in Rust `.so` | Rust definition site |
|---|--------------------|-----------------------|----------------------|
| 1 | `tritanopia` | YES (`T tritanopia`) | `src/lib.rs`, `#[unsafe(no_mangle)] pub extern "C" fn tritanopia` |

Symbol diff (`comm -3` of the two sorted lists): **EMPTY**.

* Symbols in C `.so` but not Rust `.so`: **0**
* Symbols in Rust `.so` but not C `.so`: **0**

No symbol required a new `#[no_mangle]` wrapper, and no C module was left
untranslated: `c_src/src/lib.c` is the only C translation unit and all seven of
its functions are present in `src/lib.rs`.

### Internal (non-exported) C functions — translated, intentionally not exported

`c_src/src/lib.c` declares these `static`, so they are absent from the C `.so`
dynamic symbol table. The Rust keeps them private for exact parity; exporting
them would *add* symbols the C does not have.

| C `static` function | Rust counterpart |
|---------------------|------------------|
| `cbRemoveGammaRGB`  | `fn cbRemoveGammaRGB` (private) |
| `cbNorm`            | `fn cbNorm` (private) |
| `cbDenorm`          | `fn cbDenorm` (private) |
| `cbApplyGammaRGB`   | `fn cbApplyGammaRGB` (private) |
| `Tritanopia`        | `fn Tritanopia` (private) |

## Undefined (imported) symbols

The C `.so` imports exactly one non-weak libc symbol: `pow@GLIBC_2.29`.
The Rust `.so` also imports `pow@GLIBC_2.29` (declared in an
`unsafe extern "C"` block, so both builds resolve to the *same* libm `pow`).

Every other undefined symbol in the Rust `.so` is libc / libgcc-unwind runtime
support pulled in by the Rust standard library (`malloc`, `memcpy`, `abort`,
`_Unwind_*`, `dl_iterate_phdr`, ...). **0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** — therefore there is
exactly one build configuration (`--no-default-features` is equivalent to the
default). The whole gate is nevertheless re-run under both invocations by
`check_all_features.sh`.
