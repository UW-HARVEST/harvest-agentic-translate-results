# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C: `c_src/build/libharvest-work-NC97R8.so` (cmake, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust: `translation/target/{release,debug}/libflac_validate_lib.so` (`crate-type = ["cdylib"]`)

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```
add_library(${project_name} SHARED src/lib.c)
```

`c_src/src/lib.c` contains exactly two non-`static` definitions
(`tflac_size_memory`, `flac_validate`) and one file-local `enum
TFLAC_CHANNEL_MODE` (enums emit no symbols). `c_src/include/lib.h` declares
`flac_validate` and the `struct tflac` type only. There is **no untranslated C
module** — the whole library is one file and both of its exported functions are
translated in `translation/src/lib.rs`. No stubs, no `unimplemented!()`.

## Defined (exported) symbol parity

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `flac_validate` | `T` | `T` | present in both |
| 2 | `tflac_size_memory` | `T` | `T` | present in both |

`tflac_size_memory` is **not** declared in `include/lib.h` but is non-`static`
in `src/lib.c`, so it is part of the dynamic export surface and is exported by
the Rust side too (`#[unsafe(no_mangle)] pub extern "C"`).

### Symbol diff

```
$ comm -3 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
          <(nm -D --defined-only rust.so| awk '{print $NF}' | sort)
(empty)
```

**Missing from Rust: 0. Extra in Rust: 0.**

## Weak / linker-generated symbols

Both objects carry the same set of compiler/linker weak undefined markers
(`_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__gmon_start__`,
`__cxa_finalize`). These are toolchain artifacts, not API.

## Undefined symbols in the Rust `.so`

All undefined symbols are libc / libgcc-unwind imports pulled in by the Rust
runtime (`malloc`, `memcpy`, `memset`, `abort`, `pthread_key_*`,
`_Unwind_*`, …). **0 undefined non-libc symbols.**

## ABI: `struct tflac` layout

Confirmed against the C compiler (`offsetof`/`sizeof` probe) and locked into
`src/lib.rs` with `const _: () = { assert!(...) }`:

| field | C offset | Rust offset |
|-------|---------:|------------:|
| `blocksize` | 0 | 0 |
| `samplerate` | 4 | 4 |
| `channels` | 8 | 8 |
| `bitdepth` | 12 | 12 |
| `channel_mode` | 16 | 16 |
| `max_rice_value` | 17 | 17 |
| `min_partition_order` | 18 | 18 |
| `max_partition_order` | 19 | 19 |
| `partition_order` | 20 | 20 |
| *(padding)* | 21..23 | 21..23 |
| `cur_blocksize` | 24 | 24 |
| **sizeof / alignof** | **28 / 4** | **28 / 4** |

## Feature configurations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default (empty) feature set;
`--no-default-features` is equivalent to the default build. Phase D therefore
verifies: default features, `--no-default-features`, and both the `debug`
(overflow checks + `panic=unwind`) and `release` (`panic=abort`) `.so` builds.
