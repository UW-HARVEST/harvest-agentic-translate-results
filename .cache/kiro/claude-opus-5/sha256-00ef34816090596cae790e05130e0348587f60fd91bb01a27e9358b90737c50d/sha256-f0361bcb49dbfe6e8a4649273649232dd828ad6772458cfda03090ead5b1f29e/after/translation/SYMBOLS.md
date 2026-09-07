# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> c_src/build/libSimpleList.so

cd translation && cargo build --release
#   -> translation/target/release/libSimpleList.so
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libSimpleList.so`

| # | symbol | type | source of truth | present in Rust `.so`? |
|---|--------|------|-----------------|------------------------|
| 1 | `smallestValue` | `T` (global text) | `c_src/src/simplestruct.c:26`, declared `c_src/include/simplestruct.h:31` | YES — `T smallestValue` |

There are no other C translation units (`CMakeLists.txt` lists exactly
`src/simplestruct.c`), no macro-generated / renamed exports, no versioned
aliases, and no exported data objects. The public header declares exactly one
function and no global variables, so the complete public ABI surface is the
single row above.

## Rust `.so` exported (defined) dynamic symbols

`nm -D --defined-only translation/target/release/libSimpleList.so`

| # | symbol | type | Rust definition |
|---|--------|------|-----------------|
| 1 | `smallestValue` | `T` (global text) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn smallestValue` in `src/lib.rs` |

`crate-type = ["cdylib"]` means Rust internals are not exported, so the Rust
export set is exactly the one C symbol — no extra public symbols leak.

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libSimpleList.so    | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only translation/.../libSimpleList.so | awk '{print $NF}' | sort -u)
```

Result: **empty**. Missing-symbol count = **0**.

No symbol required translation work: the single C function is fully translated
(not stubbed, no `unimplemented!()`), and no C module was skipped.

## Undefined (imported) symbols

* C `.so` undefined: only the four standard glibc/toolchain weak/undef entries
  (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
  `__cxa_finalize`, `__gmon_start__`).
* Rust `.so` undefined: the same four, plus only libc / libgcc-unwind imports
  pulled in by the Rust standard library (`malloc`, `free`, `memcpy`, `mmap64`,
  `pthread_key_create`, `_Unwind_*`, …).

**0 missing/undefined non-libc symbols in the Rust `.so`.** ✅

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration (the default, which is also `--no-default-features`).
Symbol parity and all tests were re-checked under both invocations; see
`CONFIGS.md` / `ERRORS.md` for results.
