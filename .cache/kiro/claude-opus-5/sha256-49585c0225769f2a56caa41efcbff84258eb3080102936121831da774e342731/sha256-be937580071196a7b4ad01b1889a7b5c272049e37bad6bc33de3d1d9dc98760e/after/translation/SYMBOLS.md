# SYMBOLS.md — dynamic-symbol parity, C `.so` vs Rust `.so`

Derived mechanically from `nm -D` on the two shared libraries.

## How the libraries are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/mdcore.c src/mdmain.c)`,
so CMake alone never emits a `.so`. `c_src/` is not modified; instead `build_c.sh`
compiles the same translation units two ways for every configuration:

```
gcc -shared -fPIC -O2 -DOP=<op> -DREPEAT=<n> -o cbuild/libcdriver_<op>_<n>.so c_src/src/mdcore.c
gcc        -O2       -DOP=<op> -DREPEAT=<n> -o cbuild/driver_<op>_<n>          c_src/src/mdcore.c c_src/src/mdmain.c
```

`mdmain.c` is excluded from the `.so` because it defines `main`; `mdcore.c` alone is
exactly the set of translation units that define the header's public surface
(`mdmacros.h` declares `op_add/op_sub/op_mul`, `G_OP`, `G_OP_NAME`,
`helper_call`, `helper_ptr`, `use_generated` as "provided by md_core.c").

The Rust side (`build_rust.sh`):

```
cargo build --release --no-default-features --features <op>,repeat_<n>
  -> target/release/libdriver.so  (crate-type = ["cdylib"])  -> rbuild/librdriver_<op>_<n>.so
  -> target/release/driver        ([[bin]])                  -> rbuild/rdriver_<op>_<n>
```

## Defined (exported) dynamic symbols

`nm -D --defined-only` on `cbuild/libcdriver_add_5.so`, and the same on
`rbuild/librdriver_add_5.so`. The C symbol set is byte-identical across all
24 `OP` × `REPEAT` configurations (verified by looping `nm -D` over `cbuild/`).

| # | C symbol | C `nm` type | size | Rust exports it? | Rust definition |
|---|----------|-------------|------|------------------|-----------------|
| 1 | `op_add`        | `T` (text)   | fn | yes | `mdcore::op_add` (`#[no_mangle] extern "C"`) |
| 2 | `op_sub`        | `T` (text)   | fn | yes | `mdcore::op_sub` (`#[no_mangle] extern "C"`) |
| 3 | `op_mul`        | `T` (text)   | fn | yes | `mdcore::op_mul` (`#[no_mangle] extern "C"`) |
| 4 | `helper_call`   | `T` (text)   | fn | yes | `mdcore::helper_call` (`#[no_mangle] extern "C"`) |
| 5 | `helper_ptr`    | `T` (text)   | fn | yes | `mdcore::helper_ptr` (`#[no_mangle] extern "C"`) |
| 6 | `use_generated` | `T` (text)   | fn | yes | `mdcore::use_generated` (`#[no_mangle] extern "C"`) |
| 7 | `G_OP`          | `D` (`.data`, OBJECT) | 8 | yes | `mdcore::G_OP` — `#[no_mangle] pub static mut G_OP: OpFn` |
| 8 | `G_OP_NAME`     | `D` (`.data`, OBJECT) | 8 | yes | `mdcore::G_OP_NAME` — `#[no_mangle] pub static mut G_OP_NAME: *const c_char` |

Symbol diff (`comm -23` on the sorted name lists): **empty** — 0 missing.

### Deliberately NOT exported (correctly absent from both)

| C entity | why it is not a dynamic symbol |
|----------|-------------------------------|
| `accum_<OP>` (from `DEFINE_ACCUM(OP)`) | `DEFINE_ACCUM` declares it `static int` → internal linkage, `nm -D` shows nothing. Rust keeps it as the private `mdmacros::accum`. |
| `main` | lives in `mdmain.c`, which is not part of the `.so`. |
| `STR/CAT/STEP_*/INIT_*/REP0..REP7/DISPATCH_REP/FOR_EACH/DO_LOOP/RUN_LOOP/CHOOSE_REP/OP_FN/ACCUM_FN` | preprocessor macros; they have no linkage at all. |

No symbol in this file is a stub: every Rust export runs a real translation of
the corresponding C body.

## `.data` vs RELRO — the one parity defect found and fixed

`nm` reports `G_OP`/`G_OP_NAME` as `D` in the C `.so` (section `.data`, writable
for the lifetime of the process) because `mdmacros.h` declares them
non-`const`:

```c
extern int (*G_OP)(int, int);
extern const char *G_OP_NAME;   /* the POINTER is mutable; only the chars are const */
```

The original Rust translation used immutable `static`s, which rustc places in
`.data.rel.ro`. That section is mapped read-only once the dynamic loader
finishes relocation processing (`PT_GNU_RELRO`), so a consumer that assigns to
`G_OP` — legal, and the whole reason the C declaration is non-`const` — crashed
against the Rust `.so` while succeeding against the C `.so`:

```
$ ./scratch/probe ./cbuild/libcdriver_add_5.so    # C
name=add gop(7,3)=10
after write gop(7,3)=4
rc=0
$ ./scratch/probe ./rbuild/librdriver_add_5.so    # Rust, before the fix
name=add gop(7,3)=10
Segmentation fault (core dumped)   rc=139
```

Fixed by declaring both as `static mut`, which lands them in `.data`
(confirmed with `readelf -SW` + `readelf -sW`). Covered by
`CONFIGS.md` rows G7-* and the `writable_globals` tests.

## Undefined dynamic symbols in the Rust `.so`

`nm -D --undefined-only rbuild/librdriver_add_5.so` lists only libc / libgcc_s
imports (`memcpy`, `malloc`, `write`, `writev`, `__errno_location`,
`pthread_key_create`, `_Unwind_*`, …) plus the usual weak
`_ITM_*`/`__gmon_start__`/`__cxa_finalize` stubs. `ldd` resolves every one of
them against `libc.so.6`, `libgcc_s.so.1` and `ld-linux-x86-64.so.2`, and
`dlopen(..., RTLD_NOW)` succeeds, which proves there is no unresolved
non-libc symbol.

**0 missing exported symbols, 0 unresolved non-libc undefined symbols.**

## Verified result

Measured after the fix, over **all 24** `OP` × `REPEAT` configurations:

```
$ for op in add sub mul; do for r in 0..7; do
    comm -23 <(nm -D --defined-only cbuild/libcdriver_${op}_${r}.so | awk '{print $3}' | sort) \
             <(nm -D --defined-only rbuild/librdriver_${op}_${r}.so | awk '{print $3}' | sort)
  done; done
configs with any symbol difference: 0 / 24
```

Both directions are empty: no C symbol is missing from the Rust `.so`, and the
Rust `.so` exports nothing the C `.so` does not. Enforced continuously by
`tests/phase_d_symbols.rs`:

| test | checks |
|------|--------|
| `d1_exported_symbol_diff_is_empty` | the C `.so` exports exactly the 8 header symbols, and none is missing from the Rust `.so` |
| `d2_rust_exports_nothing_extra` | no extra exports on the Rust side |
| `d3_static_and_main_are_not_exported` | `accum_add`/`accum_sub`/`accum_mul`/`main`/`atoi` absent from both |
| `d4_no_unresolved_non_libc_symbols` | `ldd` shows no `not found` and only `libc.so`/`libgcc_s.so`/`ld-linux`/`linux-vdso`; `dlopen(RTLD_NOW)` succeeds on both |
| `d5_export_set_is_configuration_independent` | the C export set does not change with `OP`/`REPEAT` |

Completion status: **0 missing exported symbols, 0 extra exported symbols,
0 unresolved non-libc undefined symbols**, in every configuration.
