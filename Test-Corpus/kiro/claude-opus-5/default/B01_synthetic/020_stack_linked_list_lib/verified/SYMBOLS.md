# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D c_src/build/libSimpleList.so

# Rust
cd translation && cargo build --release
nm -D translation/target/release/libSimpleList.so
```

## Defined (exported) symbols

Every symbol the C `.so` defines, and whether the Rust `.so` defines it too.

| # | symbol | C `.so` | Rust `.so` | source of truth | status |
|---|--------|---------|------------|-----------------|--------|
| 1 | `smallestValue` | `T` (defined, global) | `T` (defined, global) | `c_src/include/simplestruct.h:31`, `c_src/src/simplestruct.c:26` | MATCH |

The public header contains no function-renaming or symbol-generating
preprocessor macros (`grep -nE '#define' c_src/include/simplestruct.h` yields only
the include guard `SIMPLESTRUCT_H_`), so there are no macro-generated symbols to
account for. `struct ListNode` is a type, not a linker symbol.

**Missing from Rust `.so`: 0.** No `#[no_mangle]` wrappers to add and no
untranslated C module: `c_src/src/` contains exactly one translation unit,
`simplestruct.c`, and its single function is translated in
`translation/src/lib.rs`.

## Weak / linker-provided symbols

Present in both, supplied by the toolchain rather than by library source; not
part of the API surface:

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`.

## Undefined symbols

- C `.so`: none beyond the weak toolchain symbols above.
- Rust `.so`: undefined entries exist, but **all** are libc (`malloc`, `free`,
  `memcpy`, `open64`, `read`, `write`, `pthread_key_create`, …) or the C++/Rust
  unwinder (`_Unwind_*@GCC_*`) pulled in by the Rust standard library. There are
  **0 undefined non-libc/non-runtime symbols**, i.e. no unresolved references to
  library code that was never translated.

## Verdict

Symbol parity is exact: the set of defined, non-toolchain symbols is
`{smallestValue}` for both libraries. Symbol diff is **empty**.

## Phase D completion evidence

Command:

```sh
clean() { nm -D --defined-only "$1" | awk '$2!="w" && $1!="w"' \
          | awk '{print $NF}' \
          | grep -vE '^(_ITM_|__cxa_|__gmon_start__)' | sort -u; }
diff <(clean c_src/build/libSimpleList.so) \
     <(clean translation/target/release/libSimpleList.so)
```

Result: **empty diff**. Both sides yield exactly `smallestValue`.

Rust undefined symbols outside libc / the unwinder: **none** (filtering
`@GLIBC`, `@GCC`, `_Unwind_*`, `statx`, `gettid` leaves an empty list).

Asserted programmatically as well, by the `symbol_parity` test in
`tests/differential.rs`, which shells out to `nm -D` on both libraries and fails
if the export sets differ.
