# SYMBOLS.md — Phase A: dynamic-symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C file | translated to | status |
|--------|---------------|--------|
| `c_src/src/lib.c` | `translation/src/lib.rs` | fully translated |
| `c_src/include/lib.h` (types/macros only) | `translation/src/lib.rs` | fully translated |

No C module was skipped, so no symbol is missing because of an absent
implementation.

## Defined (exported) dynamic symbols

`nm -D --defined-only <so>`:

| # | symbol | C `libdriver.so` | Rust `libdriver.so` | notes |
|---|--------|------------------|---------------------|-------|
| 1 | `parse_number` | `T` (0x1139) | `T` (0x11e40) | declared in `include/lib.h`; no namespacing macros, so linker name == source name |

**Symbol diff (C-defined minus Rust-defined): EMPTY.**

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
<no output>
```

The Rust `.so` additionally exports no other non-Rust-runtime names of its own
(`crate-type = ["cdylib"]`, all internal items private / not `#[no_mangle]`).

## Undefined (imported) symbols

The C object imports only libc:

| symbol | needed by |
|--------|-----------|
| `malloc@GLIBC_2.2.5` | temporary buffer allocation |
| `free@GLIBC_2.2.5` | temporary buffer release |
| `memcpy@GLIBC_2.14` | copying the scanned digits |
| `strtod@GLIBC_2.2.5` | number conversion |
| weak: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__` | C runtime glue |

The Rust `.so` imports the same four functional libc symbols (`malloc`, `free`,
`memcpy`, `strtod`) — i.e. it really does route allocation and conversion through
libc, exactly like the C — plus the standard Rust-runtime imports
(`_Unwind_*`, `dl_iterate_phdr`, `pthread_key_*`, `mmap64`, `abort`, …) that come
from `std`/panic machinery. **0 missing / unresolvable non-libc symbols.**

```
$ ldd -r translation/target/release/libdriver.so | grep -c 'undefined symbol'
0
$ ldd -r c_src/build/libdriver.so | grep -c 'undefined symbol'
0
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. The feature-combination sweep in Phase D
therefore degenerates to:

| combo | command |
|-------|---------|
| default | `cargo test` |
| `--no-default-features` (identical to default: no features exist) | `cargo test --no-default-features` |
| release codegen | `cargo test --release` |

All three are still run, because the debug/release split changes Rust codegen
(overflow checks, float/int cast lowering, inlining) even though the feature set
does not.
