# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   `.so`: `c_src/build/libharvest-work-R3geiw.so`
* Rust`.so`: `translation/target/release/libupdate_md5_lib.so`

## C exported symbols (`nm -D --defined-only`, sorted)

| # | symbol | type | declared in | defined in | present in Rust `.so`? |
|---|--------|------|-------------|------------|------------------------|
| 1 | `tflac_pack_u64le`    | `T` (global text) | (not in `lib.h`; external linkage in `lib.c`) | `c_src/src/lib.c` | YES |
| 2 | `tflac_md5_addsample` | `T` (global text) | (not in `lib.h`; external linkage in `lib.c`) | `c_src/src/lib.c` | YES |
| 3 | `update_md5`          | `T` (global text) | `c_src/include/lib.h` | `c_src/src/lib.c` | YES |

There are no macro-generated symbols, no exported data objects, no exported
enums/constants, and no `static` (internal-linkage) functions in `lib.c`.
The C `.so` therefore exports exactly three non-libc symbols.

## Rust exported symbols (`nm -D --defined-only`, sorted)

```
tflac_md5_addsample
tflac_pack_u64le
update_md5
```

## Diff

```
$ comm -23 <(nm -D --defined-only C.so  | awk '{print $3}' | sort) \
           <(nm -D --defined-only R.so  | awk '{print $3}' | sort)
<empty>
```

**Missing from Rust: 0.** No stubs were added; each Rust symbol is a real
`#[no_mangle] pub unsafe extern "C"` translation of the corresponding C body.

## Undefined (imported) symbols

C `.so` undefined: none besides the ELF/libc glue (`__cxa_finalize`,
`_ITM_*`, `__gmon_start__`) — `lib.c` calls no library functions.
Rust `.so` undefined: only libc/unwind glue. No unresolved non-libc symbols.

## ABI / layout parity (checked against the C compiler, see `tests/`)

| type | C `sizeof` | C offsets | Rust `size_of` | Rust offsets |
|------|-----------|-----------|----------------|--------------|
| `tflac_md5` | 88 | `pos`@0, `total`@8, `buffer`@16 (len 72) | 88 | same |
| `tflac`     | 96 | `md5_ctx`@0, `cur_blocksize`@88, `channels`@92 | 96 | same |

Signatures:

```c
void      tflac_pack_u64le   (tflac_u8 *d, tflac_u64 n);
void      tflac_md5_addsample(tflac_md5 *m, tflac_u32 bits, tflac_u64 val);
tflac_u32 update_md5         (tflac *t, const tflac_s32 *samples);
```

## Verification result (re-checked after all fixes)

```
$ nm -D --defined-only <C.so>  | awk '{print $3}' | sort > c_syms
$ nm -D --defined-only <RS.so> | awk '{print $3}' | sort > r_syms
$ comm -23 c_syms r_syms      # exported by C, missing from Rust
                              # -> EMPTY (0 lines)
$ diff c_syms r_syms          # -> no differences
```

* **Missing/undefined non-libc symbols in Rust: 0.**
* `nm -D --undefined-only` on the Rust `.so` lists only libc/`_Unwind_*`/TLS
  glue pulled in by the Rust runtime (`malloc`, `memcpy`, `abort`,
  `_Unwind_Resume`, …). No unresolved symbol from this library.
* No stubs, no `unimplemented!()`, no `todo!()` anywhere:
  `grep -rnE 'unimplemented!|todo!|panic!\("not' src/` → 0 matches.

## Binary / driver executable

Neither build produces an executable: `c_src/CMakeLists.txt` contains only
`add_library(... SHARED src/lib.c)` (no `add_executable`), and
`translation/Cargo.toml` declares only `[lib]` (no `[[bin]]`, no `src/main.rs`).
There is therefore **no stdout to compare** — that completion-gate item is N/A.
