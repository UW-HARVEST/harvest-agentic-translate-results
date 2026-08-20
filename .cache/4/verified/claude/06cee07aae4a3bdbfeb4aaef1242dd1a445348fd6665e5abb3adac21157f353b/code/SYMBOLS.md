# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared libraries.

```
C   : c_src/build/libtranslated_rust.so   (cmake, gcc 11.5.0, x86-64, default flags)
RUST: target/release/libsynth_pair_lib.so (rustc 1.94.0, crate-type = ["cdylib"])
```

## Public headers

`c_src/include/lib.h` is the only public header. It declares exactly one
function and one typedef, and contains **no namespace-renaming macros**, so
every source-level name is also the final linker name:

```c
typedef int16_t mp3d_sample_t;
void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);
```

## Translation-unit inventory

`c_src/CMakeLists.txt` compiles a single translation unit, `src/lib.c` (34
lines). It defines two functions:

| C symbol          | linkage          | exported? | Rust counterpart                                          |
|-------------------|------------------|-----------|-----------------------------------------------------------|
| `mp3d_scale_pcm`  | `static` (internal) | no     | `fn mp3d_scale_pcm` (private, `src/lib.rs`)               |
| `synth_pair`      | external         | **yes**   | `#[unsafe(no_mangle)] pub unsafe extern "C" fn synth_pair`|

No C source file, function, or type in `c_src/` is left untranslated: the whole
library is 1 header + 1 `.c` file, and both of its functions are present in
`src/lib.rs`.

## `nm -D --defined-only` (dynamic, defined)

| # | symbol       | C `.so` | Rust `.so` | status |
|---|--------------|---------|------------|--------|
| 1 | `synth_pair` | `T`     | `T`        | OK     |

```
$ nm -D --defined-only libtranslated_rust.so | awk '{print $2, $3}'
T synth_pair
$ nm -D --defined-only libsynth_pair_lib.so  | awk '{print $2, $3}'
T synth_pair
```

**Symbol diff (C minus Rust): EMPTY.** No symbol had to be added and no C
module had to be back-filled — the surface really is a single function.
`mp3d_scale_pcm` is `static` in C, so it is deliberately *not* exported by the
Rust library either (exporting it would be a spurious addition, not parity).

## Undefined (imported) symbols

The C `.so` imports only the four weak toolchain symbols every gcc-built shared
object has (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports those same weak symbols plus the Rust standard library's
libc/libgcc dependencies (`malloc`, `memcpy`, `abort`, `_Unwind_*`, …). All of
them resolve out of `libc`/`libgcc_s`, which is why `dlopen` of the Rust library
succeeds with no `LD_PRELOAD` or extra link flags — verified by the tests, which
`dlopen` it and `dlsym` `synth_pair` on every run.

**0 missing and 0 unresolvable non-libc undefined symbols in the Rust library.**

## Verification

`tests/phase_d_symbols.rs` re-derives both symbol lists with `nm -D` at test
time and fails if the C-minus-Rust difference is ever non-empty, so this table
cannot silently rot.
