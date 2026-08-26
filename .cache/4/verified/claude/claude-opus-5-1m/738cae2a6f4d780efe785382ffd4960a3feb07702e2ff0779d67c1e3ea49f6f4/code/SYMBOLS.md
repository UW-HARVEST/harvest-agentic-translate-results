# SYMBOLS.md — exported surface of the C shared object vs. the Rust `cdylib`

## How the two shared objects are built

* **C** — `./build_c_lib.sh` compiles *exactly* the source list of the CMake
  target (`add_executable(driver src/main.c src/scene.c src/shape.c)`) into
  `c_build/libcdriver.so`:

  ```
  gcc -shared -fPIC -O0 -g -I c_src/include c_src/src/{main,scene,shape}.c \
      -o c_build/libcdriver.so
  ```

  Nothing inside `c_src/` is modified; only the extra `c_src/build/` directory
  produced by the documented `cmake` invocation and the out-of-tree
  `c_build/` directory are created.

* **Rust** — `cargo build` produces `target/debug/libdriver.so`
  (`crate-type = ["cdylib", "rlib"]`, `src/lib.rs` → `src/capi/{shape,scene,app}.rs`).

  The tests do **not** trust that file: `cargo test` never rebuilds a `cdylib`
  that no test target links against, so a stale (or missing) `libdriver.so`
  would silently be tested.  `tests/common/mod.rs::rust_lib_path()` therefore runs
  a nested `cargo build --lib` with `CARGO_TARGET_DIR=target/testlib` (a separate
  target directory, so it cannot deadlock against the outer `cargo test`) and
  loads `target/testlib/debug/libdriver.so`.  Cargo decides itself whether
  anything must be recompiled - no home grown staleness heuristic is involved.

## Build-time configuration matrix

| source | configuration knobs | valid combinations |
|--------|--------------------|--------------------|
| `Cargo.toml` | no `[features]` section at all — no optional dependencies either | exactly one: the (empty) default feature set |
| `c_src/CMakeLists.txt` | no `option()`, no `add_definitions`, no `#ifdef`-driven code anywhere in `c_src` (`grep -c '#if' c_src/src/*.c c_src/include/*.h` → only the two header guards) | exactly one |

Therefore the complete feature matrix is:

| # | cargo invocation | note |
|---|------------------|------|
| 1 | `cargo <cmd>` (default features) | the default configuration |
| 2 | `cargo <cmd> --no-default-features` | identical to #1 — there is no `default` feature |
| 3 | `cargo <cmd> --all-features` | identical to #1 — there are no features |

`./check_all_features.sh` runs `cargo check`, `cargo build` and `cargo test`
for every one of those invocations.

## `nm -D --defined-only` comparison

28 symbols are exported by the C shared object; all 28 are exported by the Rust
shared object with the identical name, and the Rust shared object exports no
additional symbols.

| # | symbol | C translation unit | Rust definition |
|---|--------|--------------------|-----------------|
| 1 | `shape_manager_init` | `shape.c` | `src/capi/shape.rs` |
| 2 | `shape_manager_cleanup` | `shape.c` | `src/capi/shape.rs` |
| 3 | `shape_get` | `shape.c` | `src/capi/shape.rs` |
| 4 | `shape_print` | `shape.c` | `src/capi/shape.rs` |
| 5 | `shape_equals` | `shape.c` | `src/capi/shape.rs` |
| 6 | `shape_type_name` | `shape.c` | `src/capi/shape.rs` |
| 7 | `scene_create` | `scene.c` | `src/capi/scene.rs` |
| 8 | `scene_destroy` | `scene.c` | `src/capi/scene.rs` |
| 9 | `scene_add_shape` | `scene.c` | `src/capi/scene.rs` |
| 10 | `scene_remove_shape` | `scene.c` | `src/capi/scene.rs` |
| 11 | `scene_print` | `scene.c` | `src/capi/scene.rs` |
| 12 | `scene_equals` | `scene.c` | `src/capi/scene.rs` |
| 13 | `scene_save` | `scene.c` | `src/capi/scene.rs` |
| 14 | `scene_load` | `scene.c` | `src/capi/scene.rs` |
| 15 | `scene_list_shapes` | `scene.c` | `src/capi/scene.rs` |
| 16 | `print_menu` | `main.c` | `src/capi/app.rs` |
| 17 | `view_all_shapes` | `main.c` | `src/capi/app.rs` |
| 18 | `create_new_scene` | `main.c` | `src/capi/app.rs` |
| 19 | `add_shape_to_scene` | `main.c` | `src/capi/app.rs` |
| 20 | `remove_shape_from_scene` | `main.c` | `src/capi/app.rs` |
| 21 | `view_scene` | `main.c` | `src/capi/app.rs` |
| 22 | `list_all_scenes` | `main.c` | `src/capi/app.rs` |
| 23 | `save_scene_to_file` | `main.c` | `src/capi/app.rs` |
| 24 | `load_scene_from_file` | `main.c` | `src/capi/app.rs` |
| 25 | `compare_shapes` | `main.c` | `src/capi/app.rs` |
| 26 | `compare_scenes` | `main.c` | `src/capi/app.rs` |
| 27 | `delete_scene` | `main.c` | `src/capi/app.rs` |
| 28 | `main` | `main.c` | `src/capi/app.rs` (`#[export_name = "main"]`) |

### Why a whole module had to be translated

Before this verification pass the crate had **no** C ABI surface at all: it was
a `[[bin]]`-only crate (`crate-type` was not even set), so `nm -D` on any Rust
artifact showed 0 of the 28 C symbols.  The safe/idiomatic translation of the
three C files existed (`src/cio.rs`, `src/scene.rs`, `src/shape.rs`,
`src/main.rs`) but in a shape that cannot be exported (`Vec<u8>` scene names,
`&'static str` art rows, shape *indices* instead of `shape_t *`, a synthesised
`%p` value, …).

The missing C ABI implementation was therefore written, mirroring the C source
statement by statement (`src/capi/`):

* the same `#[repr(C)]` structs — verified against the C compiler:
  `sizeof(shape_t) = 2444` (`type` @ 0, `name` @ 4, `art` @ 36, `width` @ 2436,
  `height` @ 2440), `sizeof(scene_t) = 472` (`name` @ 0, `shapes` @ 64,
  `shape_count` @ 464);
* the same singleton `malloc`/`free` pattern (`shape_t` instances are *not*
  zero-initialised in C, so the untouched `art` rows stay uninitialised in Rust
  too);
* the same libc stdio calls (`printf`, `fprintf`, `fopen`, `fgets`, `fscanf`,
  `scanf`, `sscanf`, `getchar`, `strcspn`, `strncpy`) so that the byte stream on
  `stdout`/`stderr`, the `%p` formatting, the stream buffering and the `scanf`
  push-back behaviour are the same by construction.

No symbol is stubbed: every exported function contains the full translation of
its C body.

### Undefined symbols

`nm -D --undefined-only target/debug/libdriver.so` lists only glibc / libgcc
imports (`printf`, `malloc`, `__isoc99_scanf`, `stdin`, `stderr`,
`_Unwind_*`, …) — 0 missing non-libc symbols.  `ldd` resolves everything with
`libc.so.6`, `libgcc_s.so.1` and the vdso.

### Verification command

```
$ ./build_c_lib.sh && cargo build
$ diff <(nm -D --defined-only c_build/libcdriver.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort)
   (no output — the sets are identical)
```

`tests/symbols.rs` re-checks this automatically (both directions) as part of
`cargo test`.

## Verification status

```
$ ./build_c_lib.sh && cargo build && cargo test
   ...
test binary_diff ... ok        # 179 executable level scenarios
test configs_app ... ok        # CONFIGS.md rows 31-52, 218 cases
test configs_lib ... ok        # CONFIGS.md rows  1-30,  30 cases (32-64 random inputs each)
test errors_app  ... ok        # ERRORS.md  rows 39-78, 126 cases
test errors_lib  ... ok        # ERRORS.md  rows  1-38,  24 cases
test struct_layout ... ok
test symbol_parity ... ok

$ ./check_all_features.sh
ALL FEATURE COMBINATIONS OK
```

* `nm -D --defined-only`: 28 symbols on both sides, symmetric difference empty.
* `nm -D --undefined-only`: only glibc / libgcc imports on both sides
  (`tests/symbols.rs` fails on any other name).
* Every symbol is additionally resolved through `dlsym` by the tests, i.e. the
  `#[no_mangle]` / `#[export_name]` wrappers are exercised - the Rust functions
  are never called directly.
