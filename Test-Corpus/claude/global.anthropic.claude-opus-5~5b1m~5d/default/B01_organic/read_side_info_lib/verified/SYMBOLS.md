# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C   `.so`: `c_src/build/libharvest-work-OPiDaj.so` (cmake, `CMAKE_BUILD_TYPE` empty → gcc 11.5 with **no** `-O` flag)
* Rust `.so`: `translation/target/release/libread_side_info_lib.so`

Regenerate with `./check_symbols.sh`.

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `read_side_info` | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn`; parity asserted by `sym_read_side_info_exported_by_both` |

`nm -D --defined-only` on the C `.so` reports exactly **one** symbol. The
`get_bits` helper is `static` in C, so it is a local symbol (`t`, not `T`) and
is deliberately *not* exported; the Rust translation keeps it as a private
`unsafe fn`. Symbol parity therefore requires exactly one export.

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/*.so     | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/*.so | awk '{print $3}' | sort)
<empty>
```

**0 missing symbols.** No C module was left untranslated: `c_src` contains a
single translation unit (`src/lib.c`, 163 lines) holding exactly two functions
(`get_bits`, `read_side_info`), both present in `src/lib.rs`. No stubs, no
`unimplemented!()`.

## Non-exported C local symbols (for reference, not part of the ABI)

| symbol | kind | Rust counterpart |
|--------|------|------------------|
| `get_bits` | `t` (local text) | private `unsafe fn get_bits` |
| `g_scf_long.2`  | `r` (local rodata, `0x2000`, 8×23 = 184 B) | `G_SCF_TABLES[OFF_LONG..]` |
| `g_scf_short.1` | `r` (local rodata, `0x20c0`, 8×40 = 320 B) | `G_SCF_TABLES[OFF_SHORT..]` |
| `g_scf_mixed.0` | `r` (local rodata, `0x2200`, 8×40 = 320 B) | `G_SCF_TABLES[OFF_MIXED..]` |

These three are function-local `static const` arrays. Their *relative* layout
is observable through the public API, because `sr_idx` can reach `8` while every
table only has 8 rows (`0..=7`) — see `CONFIGS.md` rows 25–27. The C object file
lays them out as

```
+0    g_scf_long   184 B
+184  (8 zero pad bytes, gcc aligns the next array to 32)
+192  g_scf_short  320 B
+512  g_scf_mixed  320 B
+832  .eh_frame_hdr   <-- no longer .rodata
```

`src/lib.rs` reproduces this exact blob (verified byte-for-byte against
`objdump -s -j .rodata`, and re-checked at test time by
`sym_layout_matches_c_rodata`), so the out-of-bounds row 8 aliasing matches:
`g_scf_long[8]` → pad + `g_scf_short[0]`, and `g_scf_short[8]` → `g_scf_mixed[0]`.

**This ordering is optimisation-dependent.** With gcc 11.5 the unoptimised build
(what the documented cmake invocation produces, since `CMAKE_BUILD_TYPE` is
empty) emits `long, pad, short, mixed`, while `-O1` and above emit
`mixed, short, long` with no padding. The translation targets the reference
build. Rows `0..=7` are byte-identical either way; only the one-past-the-end
`sr_idx == 8` case can tell the two layouts apart. See `CONFIGS.md` C15–C17.

## Undefined symbols

The C `.so` imports only weak toolchain hooks (`__cxa_finalize`,
`__gmon_start__`, `_ITM_*`). The Rust `.so` additionally imports libc and
libgcc unwinder symbols (`malloc`, `memcpy`, `_Unwind_*`, …) pulled in by the
Rust standard library. **0 missing/undefined non-libc symbols** — every
undefined Rust symbol resolves from `libc`/`libgcc_s`, which `libloading`
confirms by loading the object successfully in every test.
