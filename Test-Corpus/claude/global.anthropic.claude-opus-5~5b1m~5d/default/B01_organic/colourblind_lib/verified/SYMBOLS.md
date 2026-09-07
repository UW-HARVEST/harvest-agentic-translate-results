# SYMBOLS.md — Phase A: exported-symbol surface

## Build commands

```
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-skTiBN.so   (name derives from the parent dir)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libcolourblind_lib.so
```

## `nm -D` on the C `.so`

```
             w _ITM_deregisterTMCloneTable
             w _ITM_registerTMCloneTable
             w __cxa_finalize@GLIBC_2.2.5
             w __gmon_start__
000000000000 T colourblind
```

Only ONE defined, non-weak, non-libc symbol: `colourblind`.
The three transform helpers (`Protanopia`, `Deuteranopia`, `Tritanopia`) are
`static` in `c_src/src/lib.c`, so they have **no** dynamic symbol and must NOT
be exported by Rust either.

## Parity table

| # | C symbol | kind | present in Rust `.so`? | notes |
|---|----------|------|------------------------|-------|
| 1 | `colourblind` | `T` (defined, global) | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn colourblind` | exact name match |
| 2 | `_ITM_deregisterTMCloneTable` | `w` (weak undefined) | yes (weak undef, same as C) | toolchain-generated, not API |
| 3 | `_ITM_registerTMCloneTable` | `w` (weak undefined) | yes | toolchain-generated |
| 4 | `__cxa_finalize@GLIBC_2.2.5` | `w` | yes | libc |
| 5 | `__gmon_start__` | `w` | yes | toolchain-generated |

## C static (non-exported) functions — must stay unexported in Rust

| C static fn | Rust counterpart | exported? |
|---|---|---|
| `Protanopia`   | `protanopia`   | no (correct) |
| `Deuteranopia` | `deuteranopia` | no (correct) |
| `Tritanopia`   | `tritanopia`   | no (correct) |

## Result

`nm -D --defined-only` diff between the two `.so` files is **EMPTY**.
Rust's remaining `U` entries are all libc / `_Unwind_*` (Rust std + panic
machinery) — no missing project symbols. **0 missing symbols.**

## Cargo features

`translation/Cargo.toml` declares **no `[features]` table** and no
`default` feature, so there is exactly ONE feature combination
(the empty/default one). `--no-default-features` is equivalent.

No `[[bin]]` target and no `main.rs`: the project builds **no binary
executable**, so the "compare C and Rust binary stdout" gate is N/A.
