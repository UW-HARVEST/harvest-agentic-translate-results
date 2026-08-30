# SYMBOLS.md — Phase A symbol surface map

Derived mechanically from `nm -D` on both shared objects. No assumptions.

## Source inventory (proof the whole C tree is covered)

Every file in `c_src/`:

| C file | contents | translated? |
|--------|----------|-------------|
| `c_src/CMakeLists.txt` | build script, `add_library(driver SHARED src/driver.c)` | n/a (build file) |
| `c_src/include/driver.h` | single declaration `void driver(int x);` | yes |
| `c_src/src/driver.c` | single definition `driver` (5 statements) | yes — `translation/src/lib.rs` |

There is exactly **one** translation unit and **one** public function in the whole
library. No module/file was skipped, so no Phase A "translate the missing C
source" work applies.

## `nm -D` — DEFINED (exported) symbols

Commands:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | `T` (0x1109) | `T` (0x116c0) | `void driver(int)`. Exported from Rust via `#[unsafe(no_mangle)] pub extern "C" fn driver`. |

**Symbol diff (C-defined minus Rust-defined): EMPTY (0 symbols).**

```
$ comm -23 c_def.txt r_def.txt | wc -l
0
```

No macro-generated symbols exist in this library (the C source contains no
function-defining macros — verified by grep: there are no `#define`s at all
other than the `DRIVER_H_` include guard).

## Weak / undefined symbols (informational — not part of the parity gate)

The C `.so` additionally lists these, none of which are library API:

- `w _ITM_deregisterTMCloneTable`, `w _ITM_registerTMCloneTable`,
  `w __cxa_finalize@GLIBC_2.2.5`, `w __gmon_start__` — toolchain-injected weak
  symbols; the Rust `.so` also has the first, third and fourth.
- `U printf@GLIBC_2.2.5` — libc import. The Rust `.so` imports the **same**
  `printf@GLIBC_2.2.5`, so formatting is performed by the identical glibc code
  in both cases.

The Rust `.so` has additional `U` imports (`_Unwind_*`, `malloc`, `memcpy`,
`mmap64`, `dl_iterate_phdr`, …) pulled in by the Rust standard library /
panic-unwind machinery. These are **imports**, not exports, and all resolve
against the system libc/libgcc present in the process. There are **0 missing or
unresolvable non-libc symbols**:

```
$ ldd -r translation/target/release/libdriver.so   # no "undefined symbol" lines
```

## Gate status

- [x] Every symbol the C `.so` defines is defined by the Rust `.so` with the
      exact same name.
- [x] Symbol diff reaches empty.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
