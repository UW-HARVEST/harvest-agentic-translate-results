# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

```
C:    c_src/build/libharvest-work-wnyfiX.so
Rust: translation/target/release/libread_side_info_lib.so
```

## C `.so` exported symbols (`nm -D --defined-only`)

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `read_side_info` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn read_side_info` |

The C `.so` exports exactly **one** symbol. `get_bits` is `static` in
`c_src/src/lib.c`, therefore it has internal linkage and is deliberately NOT
exported (`nm -D` does not list it). The Rust side mirrors this: `get_bits` is a
private `unsafe fn` with no `#[no_mangle]`.

The three scalefactor-band tables (`g_scf_long`, `g_scf_short`, `g_scf_mixed`)
are function-local `static const` in C and are likewise not exported; in Rust
they are a private `static G_SCF_TABLES` blob.

## Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
           <(nm -D --defined-only rust.so| awk '{print $NF}' | sort)
<empty>
```

* Symbols in C but missing from Rust: **0**
* Undefined non-libc symbols in the Rust `.so`: **0** (only `libc`/`libgcc`
  runtime imports, which the C `.so` also has)

No module of the C source was left untranslated: `c_src` consists of exactly
`include/lib.h` (types + one prototype) and `src/lib.c` (`get_bits` +
`read_side_info`), and both functions are present in `translation/src/lib.rs`.

## Types crossing the ABI (verified layout)

`bs_t` — size 16, align 8: `buf` @0, `pos` @8, `limit` @12.

`L3_gr_info_t` — size 32, align 8, no internal padding:

| offset | field | C type |
|--------|-------|--------|
| 0  | `sfbtab`            | `const uint8_t *` |
| 8  | `part_23_length`    | `uint16_t` |
| 10 | `big_values`        | `uint16_t` |
| 12 | `scalefac_compress` | `uint16_t` |
| 14 | `global_gain`       | `uint8_t` |
| 15 | `block_type`        | `uint8_t` |
| 16 | `mixed_block_flag`  | `uint8_t` |
| 17 | `n_long_sfb`        | `uint8_t` |
| 18 | `n_short_sfb`       | `uint8_t` |
| 19 | `table_select[3]`   | `uint8_t[3]` |
| 22 | `region_count[3]`   | `uint8_t[3]` |
| 25 | `subblock_gain[3]`  | `uint8_t[3]` |
| 28 | `preflag`           | `uint8_t` |
| 29 | `scalefac_scale`    | `uint8_t` |
| 30 | `count1_table`      | `uint8_t` |
| 31 | `scfsi`             | `uint8_t` |

Because there is no padding, the whole 32-byte struct can be compared
byte-for-byte between the two implementations (test `struct_layout_matches`
asserts the sizes/offsets both sides agree on).

## Cargo features

`translation/Cargo.toml` declares one non-default feature:

| feature | default | effect |
|---------|---------|--------|
| `c_layout_o2` | off | Reproduces the `.rodata` array order the C compiler emits at `-O2` (`cmake -DCMAKE_BUILD_TYPE=Release`) instead of the `-O0` order produced by the plain `cmake ..` build. Observable **only** through the out-of-range `sr_idx == 8` scalefactor-band row (ERRORS.md E8/N5). |

Both feature combinations export the same single symbol and are verified
against the C build whose layout they reproduce. `run_all.sh` enumerates the
power set of the declared features (`<default>`, `--no-default-features`,
`--no-default-features --features c_layout_o2`, `--all-features`) x both build
profiles (`debug`, `release`) = 8 configurations, checks `nm -D` parity for each
and runs the full differential suite for each. All 8 pass.

## Verification of this table

```
$ ./run_all.sh
...
>>> <default features> / debug    symbols: OK (1 exported by C, 0 missing from Rust)
>>> <default features> / release  symbols: OK (1 exported by C, 0 missing from Rust)
>>> --no-default-features / debug ...
>>> --no-default-features --features c_layout_o2 / debug ...
>>> --all-features / release      symbols: OK (1 exported by C, 0 missing from Rust)
 ALL PHASES PASSED
```
