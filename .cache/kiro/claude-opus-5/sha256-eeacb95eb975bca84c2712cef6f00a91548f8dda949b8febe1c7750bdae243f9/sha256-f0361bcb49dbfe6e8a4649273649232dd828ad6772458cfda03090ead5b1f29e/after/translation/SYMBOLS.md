# SYMBOLS.md — public ABI surface parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

```
C   : c_src/build/libharvest-work-zpZw8l.so
Rust: translation/target/release/libflac_validate_lib.so
```

## C exports (`nm -D --defined-only`, non-libc)

| # | symbol | type | C signature | exported by Rust `.so`? |
|---|--------|------|-------------|--------------------------|
| 1 | `flac_validate`     | `T` | `int flac_validate(tflac *t)`                   | YES |
| 2 | `tflac_size_memory` | `T` | `tflac_u32 tflac_size_memory(tflac_u32 blocksize)` | YES |

`c_src/src/lib.c` is the only translation unit in `CMakeLists.txt`; both
non-`static` functions in it are the complete public surface. No other C
source file exists, so no module was skipped during translation.

## Symbol diff

```
$ comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # in C, missing from Rust
<empty>
```

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`.**

The Rust `.so` additionally exports the usual toolchain/runtime symbols
(`rust_eh_personality`, `_init`, `_fini`, unwinder shims, …), which is expected
and not an ABI mismatch.

## Type / ABI parity

`struct tflac` layout confirmed by compiling a C `offsetof` probe against
`c_src/include/lib.h` and comparing against the `const _: ()` layout asserts in
`translation/src/lib.rs`:

| field | C offset | Rust offset |
|-------|----------|-------------|
| `blocksize`           | 0  | 0  |
| `samplerate`          | 4  | 4  |
| `channels`            | 8  | 8  |
| `bitdepth`            | 12 | 12 |
| `channel_mode`        | 16 | 16 |
| `max_rice_value`      | 17 | 17 |
| `min_partition_order` | 18 | 18 |
| `max_partition_order` | 19 | 19 |
| `partition_order`     | 20 | 20 |
| `cur_blocksize`       | 24 | 24 |

`sizeof == 28`, `alignof == 4` in both. Bytes 21..=23 are tail padding and are
written by neither implementation.
