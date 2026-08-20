# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## Build commands

```sh
# C: executable (as shipped by c_src/CMakeLists.txt)
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> c_src/build/driver

# C: shared library built from the same, unmodified c_src/src/lib.c
gcc -shared -fPIC -O2 -I c_src/include -o build_c/libdag_c.so c_src/src/lib.c
#   -> build_c/libdag_c.so

# Rust: cdylib + binary
cargo build --offline
#   -> target/debug/libdag_rs.so , target/debug/driver
```

`c_src/CMakeLists.txt` only declares `add_executable(driver src/main.c src/lib.c)`,
so the C side has no library target of its own; `build_c/libdag_c.so` is built
from the *unmodified* `c_src/src/lib.c` purely so that the two implementations
can be compared through `dlopen()`.

## Feature / configuration matrix

`Cargo.toml` has **no `[features]` section**, so the complete set of valid
build-time configurations is:

| # | cargo invocation | meaning |
|---|------------------|---------|
| 1 | `cargo check/test --offline` | default configuration (no features) |
| 2 | `cargo check/test --offline --no-default-features` | identical: there is no `default` feature to switch off |

There are no `#ifdef`/`#if` compile-time switches in `c_src` either (`grep -n
'#if' c_src/src/*.c c_src/include/*.h` only matches the `#ifndef DAG_LIB_H`
include guard), so C and Rust each have exactly one build configuration.
Both invocations are exercised by `./run_all.sh`.

## `nm -D --defined-only` on the C `.so` vs. the Rust `.so`

| # | C symbol | in Rust `.so`? | Rust definition |
|---|----------|----------------|-----------------|
| 1 | `add_edge` | yes | `src/ffi.rs::add_edge` |
| 2 | `add_node` | yes | `src/ffi.rs::add_node` |
| 3 | `create_graph` | yes | `src/ffi.rs::create_graph` |
| 4 | `delete_node` | yes | `src/ffi.rs::delete_node` |
| 5 | `find_shortest_path` | yes | `src/ffi.rs::find_shortest_path` |
| 6 | `free_graph` | yes | `src/ffi.rs::free_graph` |
| 7 | `get_node_by_name` | yes | `src/ffi.rs::get_node_by_name` |
| 8 | `print_graph` | yes | `src/ffi.rs::print_graph` |
| 9 | `print_node` | yes | `src/ffi.rs::print_node` |
| 10 | `shallow_copy` | yes | `src/ffi.rs::shallow_copy` |

`increment_refs_recursive()` is `static` in `lib.c` and therefore not part of
either `.so`'s dynamic symbol table; it is translated as a private Rust `fn`
and is exercised through `shallow_copy()`.

Diff (must be empty):

```sh
diff <(nm -D --defined-only build_c/libdag_c.so   | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/debug/libdag_rs.so | awk '{print $3}' | sort)
# (no output — 10 symbols on each side, no extras)
```

`nm -D --undefined-only target/debug/libdag_rs.so` lists only libc / libgcc
imports (`malloc`, `free`, `fwrite`, `stdout`, `stderr`, `memcpy`, `abort`,
`_Unwind_*`, …) — 0 missing non-libc symbols.

## Why `src/ffi.rs` exists in addition to `src/dag_lib.rs`

The translated program (`src/main.rs` + `src/dag_lib.rs` + `src/cio.rs`) models
`node_t *` as an index into an arena, because `main.c` keeps using nodes after
`delete_node()` has `free()`d them and the arena is what makes that
(deliberately reproduced) use-after-free deterministic. That representation
cannot be handed to a foreign caller, which expects real `node_t *` values with
the layout published in `dag_lib.h` and calls `free()` on the array returned by
`find_shortest_path()`.

`src/ffi.rs` is therefore a second, statement-for-statement translation of
`c_src/src/lib.c` that uses the C layout (`node_t` = 240 bytes, `graph_t` = 808
bytes, verified against `offsetof`/`sizeof` in C) and glibc's `malloc`/`free`
and `stdout`/`stderr` streams, and it carries the `#[no_mangle]` exports. Both
Rust surfaces are verified against the C:

* `tests/ffi_diff.rs` — `libloading` on `build_c/libdag_c.so` **and**
  `target/debug/libdag_rs.so`, comparing return values, `node_t`/`graph_t`
  memory contents, `stdout` and `stderr` byte for byte.
* `tests/program_diff.rs` — `c_src/build/driver` vs. `target/debug/driver`,
  comparing `stdout`, `stderr` and the exit status byte for byte. This is what
  covers `src/main.rs`, `src/cio.rs` and `src/dag_lib.rs`.
