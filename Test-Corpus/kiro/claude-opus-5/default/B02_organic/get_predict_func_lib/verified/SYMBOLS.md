# SYMBOLS.md — Exported-symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
nm -D --defined-only c_src/build/libharvest-work-pwrO7n.so
nm -D --defined-only translation/target/release/libget_predict_func_lib.so
```

## C `.so` dynamic (exported) symbols

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `get_predict_func` | `T` (global text) | YES |

That is the complete list. `c_src/include/lib.h` declares exactly one
function (`int get_predict_func(int pfcn);`) and every other function in
`c_src/src/lib.c` is declared `static`, so it has internal linkage and is
**not** part of the dynamic symbol table:

`BTAC1C2_PredictSample`, `BTAC1C2_PredictSample_Pfn0` … `_Pfn11`,
`BTAC1C2_GetPredictFunc` — all appear in `nm` output as lowercase `t`
(local text), never as `T`, and never in `nm -D`.

These MUST NOT be exported from the Rust `.so` either: exporting them would
be a *divergence* from the C ABI surface, not an improvement. They are
translated in `src/lib.rs` with private (non-`no_mangle`) linkage, which is
the faithful equivalent of C `static`.

## Rust `.so` dynamic (exported) symbols

| # | symbol | type | present in C `.so`? |
|---|--------|------|---------------------|
| 1 | `get_predict_func` | `T` (global text) | YES |

## Symbol diff

```
$ comm -3 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
          <(nm -D --defined-only RUST.so | awk '{print $NF}' | sort)
(empty)
```

- Missing from Rust: **none**
- Extra in Rust: **none**
- Undefined non-libc symbols in Rust `.so`: **none**. `nm -D -u` lists only
  glibc imports (`malloc`, `memcpy`, `abort`, `open64`, `pthread_*`, …) and
  `libgcc` unwinder symbols (`_Unwind_*`), all of which resolve from the
  system C runtime. Verified with:
  `nm -D -u RUST.so | grep -v -E '@GLIBC|@GCC|_ITM_|__gmon_start__'` → empty.

## Completeness check against the C source

Every function in `c_src/src/lib.c` has a counterpart in
`translation/src/lib.rs`:

| C function (all `static` except the last) | Rust counterpart | linkage |
|---|---|---|
| `BTAC1C2_PredictSample` | `BTAC1C2_PredictSample` | private |
| `BTAC1C2_PredictSample_Pfn0` | same name | private |
| `BTAC1C2_PredictSample_Pfn1` | same name | private |
| `BTAC1C2_PredictSample_Pfn2` | same name | private |
| `BTAC1C2_PredictSample_Pfn3` | same name | private |
| `BTAC1C2_PredictSample_Pfn4` | same name | private |
| `BTAC1C2_PredictSample_Pfn5` | same name | private |
| `BTAC1C2_PredictSample_Pfn6` | same name | private |
| `BTAC1C2_PredictSample_Pfn7` | same name | private |
| `BTAC1C2_PredictSample_Pfn8` | same name | private |
| `BTAC1C2_PredictSample_Pfn9` | same name | private |
| `BTAC1C2_PredictSample_Pfn10` | same name | private |
| `BTAC1C2_PredictSample_Pfn11` | same name | private |
| `BTAC1C2_GetPredictFunc` | `BTAC1C2_GetPredictFunc` | private |
| `get_predict_func` | `get_predict_func` | `#[unsafe(no_mangle)] extern "C"` |

No C module/file was skipped: `src/lib.c` is the only source file listed in
`c_src/CMakeLists.txt`, and `include/lib.h` is the only header.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the
only build configuration is the default one. Verified with:

```sh
grep -n '^\[features\]' translation/Cargo.toml   # no match
cargo metadata --no-deps --format-version 1 | \
  python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["features"])'
# -> {}
```

`cargo test --no-default-features` is therefore equivalent to `cargo test`,
and both are run by `run_all.sh`.
