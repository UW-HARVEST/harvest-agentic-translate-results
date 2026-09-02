# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-Iix8pd.so

# Rust
cd translation && cargo build --release
# -> translation/target/release/libcolourblind_lib.so
```

## C `.so` — `nm -D`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000013d2 T colourblind
```

`nm -D --defined-only` on the C `.so` yields exactly one entry:

```
00000000000013d2 T colourblind
```

## Rust `.so` — `nm -D --defined-only`

```
00000000000116e0 T colourblind
```

## Parity table

Every non-libc symbol defined by the C `.so`, and its status in the Rust `.so`.

| # | C symbol | C bind/type | in Rust `.so`? | notes |
|---|----------|-------------|----------------|-------|
| 1 | `colourblind` | `T` (global text) | YES — `T colourblind` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn colourblind` in `src/lib.rs`. Signature `(c_int, *mut f32, *mut f32, *mut f32) -> ()` matches `void colourblind(cb_impairment, float*, float*, float*)`; `cb_impairment` has no explicit enumerator values and no `__attribute__((packed))`, so it is passed as a plain `int` in `edi` (confirmed in the disassembly: `mov %edi,-0x4(%rbp)`). |

**Missing symbols: 0. Symbol diff is EMPTY.**

## Symbols deliberately NOT exported

These are `static` in `c_src/src/lib.c`, therefore absent from the C `.so`'s
dynamic symbol table. They must NOT be exported from Rust either, or the Rust
`.so` would have a *larger* surface than the C one. They are translated as
private `unsafe fn`s.

| C symbol | linkage in C | Rust counterpart | exported? |
|----------|--------------|------------------|-----------|
| `Protanopia`   | `static` | `protanopia`   | no (correct) |
| `Deuteranopia` | `static` | `deuteranopia` | no (correct) |
| `Tritanopia`   | `static` | `tritanopia`   | no (correct) |

Confirmed absent from the C `.so`: `nm -D libharvest-work-Iix8pd.so | grep -c
-E 'Protanopia|Deuteranopia|Tritanopia'` → 0. Same for the Rust `.so`.

Because these three are unreachable across the FFI boundary, all differential
coverage of their arithmetic in Phases B and C is driven **through**
`colourblind`, selecting each one via the `Impairment` argument.

## Undefined (imported) symbols

C `.so` imports only the libc/CRT weak symbols shown above. The Rust `.so`
imports only libc symbols (`memcpy`, `__cxa_thread_atexit_impl`, and similar
`std` runtime hooks). No non-libc undefined symbols in either object.

## Completeness of the translation

`c_src` contains exactly one translation unit (`src/lib.c`, 35 lines) and one
public header (`include/lib.h`, 7 lines); `CMakeLists.txt` lists `src/lib.c` as
the sole source. Every function in that translation unit (`Protanopia`,
`Deuteranopia`, `Tritanopia`, `colourblind`) has a corresponding Rust
implementation. No C module was skipped, so no additional translation work was
required for symbol parity. Nothing is stubbed or `unimplemented!()`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is equivalent to the
default here). Verified with `cargo metadata`: the feature map for the package
is empty. Phase D's "every feature combination" therefore reduces to the single
default configuration, and the automation script re-runs the suite under both
`--all-features` and `--no-default-features` to prove they are identical.
