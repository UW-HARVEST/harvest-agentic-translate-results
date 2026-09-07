# SYMBOLS.md — Phase A symbol map

Derived mechanically from `nm -D` on both shared libraries.

## C library
`c_src/build/libharvest-work-INRZYd.so` (built from the single TU `c_src/src/lib.c`)

```
$ nm -D --defined-only c_src/build/libharvest-work-INRZYd.so | grep ' T '
0000000000001160 T synth_pair
```

## Rust library
`translation/target/release/libsynth_pair_lib.so` (`crate-type = ["cdylib"]`)

```
$ nm -D --defined-only translation/target/release/libsynth_pair_lib.so | awk '$2=="T"'
00000000000116e0 T synth_pair
```

## Parity table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `synth_pair` | `T` (global text) | `T` (global text, `#[unsafe(no_mangle)] pub unsafe extern "C"`) | OK — exact name match |

### Symbols intentionally NOT exported

| C symbol | linkage in C | required in Rust? |
|----------|--------------|-------------------|
| `mp3d_scale_pcm` | `static` (internal, not in `nm -D`) | No — private helper `fn mp3d_scale_pcm` in Rust; must not be exported |

`mp3d_sample_t` is a `typedef` (`int16_t`), not a linker symbol.

## Result

* Missing-from-Rust symbols: **0**
* Extra Rust exports that would collide with the C surface: **0**
* Undefined non-libc symbols in the Rust `.so`: **0**
  (`nm -D -u` on the Rust `.so` lists only libc/`libgcc` runtime imports.)

**Phase A / D symbol gate: PASS (diff is empty).**

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` section, so the only
configuration is the default (empty) feature set. `--no-default-features` and
the default build are therefore the same code path; both are verified.

---

## Verified output of `./verify_all.sh`

```
-- C exported symbols:
     synth_pair
-- exported by C but MISSING from Rust:
     (none)
-- undefined (imported) non-libc symbols in the Rust .so:
     (none)
```

The Rust `.so`'s only dynamic imports are libc/libgcc runtime entries
(`_Unwind_*`, `__cxa_*`, `pthread_key_*`, `malloc`, `memcpy`, …), all resolved by
the dynamic linker — confirmed in practice by the fact that every test
successfully `dlopen`s the library and calls through it.
