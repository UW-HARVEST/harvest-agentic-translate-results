# SYMBOLS.md — exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libharvest-work-Qr4nQs.so
Rust: translation/target/release/libsynth_pair_lib.so
```

## C `.so` — `nm -D --defined-only`

| symbol | type | present in Rust `.so`? | note |
|--------|------|------------------------|------|
| `synth_pair` | `T` (global text) | **yes** (`T synth_pair`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn synth_pair` |

## C `.so` — weak/undefined entries (toolchain-generated, NOT part of the API)

These are emitted by GCC/glibc CRT glue, not by `src/lib.c`. They are *not*
required of the Rust `.so` (Rust's own CRT glue provides the equivalents).

| symbol | class |
|--------|-------|
| `_ITM_deregisterTMCloneTable` | `w` weak undefined (transactional-memory glue) |
| `_ITM_registerTMCloneTable`   | `w` weak undefined (transactional-memory glue) |
| `__cxa_finalize@GLIBC_2.2.5`  | `w` weak undefined (libc) |
| `__gmon_start__`              | `w` weak undefined (profiling) |

## Non-exported C symbols (verified: must NOT be exported)

| symbol | C storage | reason |
|--------|-----------|--------|
| `mp3d_scale_pcm` | `static int16_t` | file-local (`t`, not in `.dynsym`); Rust keeps it a private `fn` |

## Header surface (`c_src/include/lib.h`)

| entity | kind | Rust counterpart |
|--------|------|------------------|
| `mp3d_sample_t` | `typedef int16_t` | `pub type mp3d_sample_t = i16` |
| `synth_pair`    | `void (mp3d_sample_t*, int, const float*)` | `unsafe extern "C" fn(*mut i16, c_int, *const f32)` |

No macros, no `#ifdef`-gated / namespace-renamed aliases, no additional C source
files exist in `c_src/src/` (`src/lib.c` is the only translation unit listed in
`CMakeLists.txt`), so no C module was left untranslated.

## Diff result

```
$ comm -23 <(nm -D --defined-only c_so   | awk '{print $NF}' | sort) \
           <(nm -D --defined-only rust_so| awk '{print $NF}' | sort)
<empty>
```

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`.**
(Checked automatically by `tests/symbols.rs::symbol_parity_c_subset_of_rust`
and `::no_unexpected_undefined_symbols`.)

## Verified output

```
$ nm -D --defined-only <C .so>
0000000000001160 T synth_pair

$ nm -D --defined-only <Rust .so>
0000000000011c80 T synth_pair

$ nm -D --undefined-only <Rust .so> | grep -v '@GLIBC\|_ITM_\|__gmon_start__\|__cxa_'
<empty>
```

Checked automatically, for **every feature combination and both profiles**, by
`./run_all_feature_combos.sh` (which runs the `comm -23` diff itself) and by
`tests/symbols.rs`:

* `symbol_parity_c_subset_of_rust`
* `symbol_diff_is_empty`
* `no_unexpected_undefined_symbols`
* `static_c_helper_is_not_exported`
* `both_libraries_resolve_the_symbol_through_dlsym`
