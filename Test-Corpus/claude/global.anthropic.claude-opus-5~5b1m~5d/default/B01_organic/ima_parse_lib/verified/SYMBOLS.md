# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-WmMOBZ.so`
- Rust `.so`: `translation/target/release/libima_parse_lib.so`

## C exported (dynamic, defined) symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-WmMOBZ.so
000000000000122d T ima_parse
```

Everything else in `c_src/src/lib.c` has internal linkage (`static`) and is
therefore not part of the ABI:

| C entity | linkage | in Rust? |
|---|---|---|
| `ima_bswap16` | `static` | yes, private `fn ima_bswap16` |
| `ima_bswap32` | `static` | yes, private `fn ima_bswap32` |
| `ima_bswap64` | `static` | yes, private `fn ima_bswap64` |
| `ima_btoh16`  | `static` | yes, private `fn ima_btoh16` |
| `ima_btoh32`  | `static` | yes, private `fn ima_btoh32` |
| `ima_btoh64`  | `static` | yes, private `fn ima_btoh64` |
| `ima_parse`   | **external** | yes, `#[no_mangle] extern "C"` |

No other translation units exist (`CMakeLists.txt` lists only `src/lib.c`), so
no C module was skipped by the translation.

## Parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `ima_parse` | `T` | `T` | ✅ present in both, exact name |

## Undefined (imported) symbols

```
$ nm -D --undefined-only c_src/build/libharvest-work-WmMOBZ.so   # (excluding libc/ld glue)
(none beyond _ITM_deregisterTMCloneTable / __gmon_start__ / _ITM_registerTMCloneTable /
 __cxa_finalize — all standard GCC/glibc startup glue)
```

The Rust `.so` imports only libc symbols (`memcpy`-class) plus the standard
Rust runtime glue; there are **0 missing/undefined non-libc symbols**.

## Result

- Symbols exported by C but missing from Rust: **0**
- Symbols exported by Rust but not by C: **0** (only `ima_parse` is `#[no_mangle]`)

✅ **Symbol diff is empty.**

## Phase D verification (automated)

`./run_all_features.sh` re-checks the symbol diff for all 4 C build variants
(default / Release / RelWithDebInfo / Debug) against both Rust profiles
(release / dev) and additionally runs `ldd -r`:

```
OK  C/Release          ->  target/release/libima_parse_lib.so : 0 missing symbols  (1 exported)
    ldd -r: 0 unresolved symbols in target/release/libima_parse_lib.so
OK  C/Release          ->  target/debug/libima_parse_lib.so   : 0 missing symbols  (1 exported)
    ldd -r: 0 unresolved symbols in target/debug/libima_parse_lib.so
OK  C/Debug            ->  ... (same)
OK  C/RelWithDebInfo   ->  ... (same)
OK  C/default          ->  ... (same)
```

✅ 0 missing symbols, 0 unresolved non-libc symbols, in every configuration.
