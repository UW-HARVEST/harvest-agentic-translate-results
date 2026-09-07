# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/liblong.so

cd translation && cargo build --release
# -> translation/target/release/liblong.so
```

## C `.so` defined dynamic symbols (`nm -D --defined-only`)

| symbol | type | size | present in Rust `.so` | notes |
|--------|------|------|-----------------------|-------|
| `array` | `B` (bss object) | `0x100000` | YES (`B`, `0x100000`) | `int array[256*1024]` global |
| `long_exec` | `T` (text) | — | YES (`T`) | `void long_exec(unsigned int seed)` |
| `perform_expensive_operations` | `T` (text) | — | YES (`T`) | `void perform_expensive_operations(void)` |

**Missing from Rust `.so`: 0.**
`array` matches in both symbol *type* (`B`) and *size* (`0x100000` = 1 MiB), so an
external caller can `dlsym("array")` and index all 262144 `int` slots in either
library identically.

The C source file `c_src/src/long.c` is the only translation unit
(`c_src/CMakeLists.txt`: `add_library(long SHARED src/long.c)`), and every
function it defines is non-`static`, so the three symbols above are the complete
public surface. No C module was left untranslated.

## Undefined (imported) symbols

C imports exactly `printf`, `rand`, `srand` from glibc (plus the standard weak
`_ITM_*`/`__gmon_start__`/`__cxa_finalize` stubs).

The Rust `.so` imports the same three (`printf@GLIBC_2.2.5`, `rand@GLIBC_2.2.5`,
`srand@GLIBC_2.2.5`) — i.e. it uses the *host libc* PRNG and the *host libc*
`printf`, exactly as the C does, which is what makes the random fill and the
printed bytes identical. The remainder of the Rust import list
(`malloc`/`free`/`memcpy`/`_Unwind_*`/…) is Rust runtime and `std` support, all
libc/libgcc-provided, and none of it is a missing translated symbol.

**Non-libc undefined symbols in the Rust `.so`: 0.**

## Verification command

```
diff <(nm -D --defined-only c_src/build/liblong.so           | awk '{print $NF}' | sort) \
     <(nm -D --defined-only translation/target/release/liblong.so | awk '{print $NF}' | sort)
```

Result: empty diff for **every** cargo feature combination
(`--no-default-features`, default, `--features debug-stats`,
`--no-default-features --features debug-stats`); each build exports exactly the
same 3 symbols. Asserted at test time by `tests/symbols.rs`, which also checks
that the Rust `.so` never *imports* a symbol the C library defines (which would
mean the Rust was delegating to the C instead of implementing it) and that both
`.so`s resolve all three symbols through `libloading`.
