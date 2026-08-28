# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-3Qo5bJ.so   (project name = parent dir name)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libflac_validate_lib.so   (crate-type = ["cdylib"])
```

## C `.so` dynamic symbols (`nm -D --defined-only`)

```
000000000000111a T flac_validate
00000000000010f9 T tflac_size_memory
```

## Rust `.so` dynamic symbols (`nm -D --defined-only`)

```
0000000000011c70 T flac_validate
0000000000011d40 T tflac_size_memory
```

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | kind | notes |
|---|--------|-----------|---------------|------|-------|
| 1 | `tflac_size_memory` | yes (`T`) | yes (`T`) | `tflac_u32 (tflac_u32)` | defined in `src/lib.c`, **not** declared in `include/lib.h`; still an exported symbol, so it is part of the ABI surface and is tested. |
| 2 | `flac_validate`      | yes (`T`) | yes (`T`) | `int (tflac *)` | declared in `include/lib.h`. |

**Symbol diff (C minus Rust): EMPTY.** No missing symbols, no stubs, no
`unimplemented!()`. Both `.so`s export exactly the same two `T` symbols.

There are no macro-generated symbols in the C source (no function-generating
macros exist in `src/lib.c` / `include/lib.h`), and no additional translation
units — `CMakeLists.txt` compiles exactly one file, `src/lib.c`, which is fully
translated in `translation/src/lib.rs`.

## Undefined (imported) non-libc symbols in the Rust `.so`

```sh
nm -D -u translation/target/release/libflac_validate_lib.so
```

Only the standard C runtime / unwinder imports that rustc always emits
(`__cxa_*`, `memcpy`, `_Unwind_*`, `__tls_get_addr`, …). **0 missing/undefined
non-libc symbols.**

## Non-function ABI surface (types / constants)

These are header-level, not linker-level, but must match for the tests to be
meaningful. Verified with a C `offsetof` program (`size=28 align=4`):

| field | C offset | Rust `#[repr(C)]` offset |
|---|---|---|
| `blocksize` (`tflac_u32`) | 0 | 0 |
| `samplerate` (`tflac_u32`) | 4 | 4 |
| `channels` (`tflac_u32`) | 8 | 8 |
| `bitdepth` (`tflac_u32`) | 12 | 12 |
| `channel_mode` (`tflac_u8`) | 16 | 16 |
| `max_rice_value` (`tflac_u8`) | 17 | 17 |
| `min_partition_order` (`tflac_u8`) | 18 | 18 |
| `max_partition_order` (`tflac_u8`) | 19 | 19 |
| `partition_order` (`tflac_u8`) | 20 | 20 |
| *(padding)* | 21..23 | 21..23 |
| `cur_blocksize` (`tflac_u32`) | 24 | 24 |
| **sizeof** | **28** | **28** |

`enum TFLAC_CHANNEL_MODE` is file-local to `src/lib.c` (no exported symbol);
its values are mirrored as `pub const`s in the Rust crate.

## Phase D result

```sh
diff <(nm -D --defined-only --format=posix c_src/build/*.so            | awk '{print $1}' | sort) \
     <(nm -D --defined-only --format=posix translation/target/release/libflac_validate_lib.so \
        | awk '$2=="T"||$2=="D"||$2=="B"{print $1}' | grep -v '^_' | sort)
# -> no output: the symbol diff is EMPTY
```

Automated as `tests/phase_d_symbols.rs`:
* `phase_d_symbol_parity` — `nm -D` set difference (C \ Rust) must be empty.
* `phase_d_no_undefined_non_libc_symbols` — every undefined import of the Rust
  `.so` is a versioned glibc/GCC runtime symbol.
* `phase_d_both_symbols_callable_through_dlsym` — both symbols are resolved with
  `dlsym` and actually invoked in both libraries.

`run_all.sh` runs the whole suite over the matrix
`{default, --no-default-features} x {debug, release} x {CMake C build, -O2 C build}`
plus the symbol diff, and (with `EXHAUSTIVE=1`) the exhaustive sweeps.

## Divergence found and fixed during verification

| # | symptom | root cause | fix (Rust side only) |
|---|---------|-----------|----------------------|
| 1 | `flac_validate(NULL)`: C child died with `SIGSEGV` (11), Rust child died with `SIGABRT` (6) — `panicked at src/lib.rs: null pointer dereference occurred` | the translation began with `let t = &mut *t;`, and creating a reference from a raw pointer makes rustc emit a debug-assertion null/alignment check, which aborts instead of faulting | field access now goes through the raw pointer only (`ptr::read`/`ptr::write` on `addr_of!((*t).field)`), never through a `&mut` reference, so an invalid `t` faults exactly like the C — identical `SIGSEGV` in both, in debug *and* release |
