# Dynamic symbol surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-fDrOX9.so
nm -D --defined-only target/release/libunfilter_lib.so
```

The C shared object has nine defined public dynamic symbols. All nine are
present under the same name in the Rust shared object.

| C symbol | kind | Rust export | status |
|---|---:|---:|---:|
| `cp_dist_base` | data | `cp_dist_base` | [x] |
| `cp_dist_extra_bits` | data | `cp_dist_extra_bits` | [x] |
| `cp_error_reason` | BSS pointer | `cp_error_reason` | [x] |
| `cp_fixed_table` | data | `cp_fixed_table` | [x] |
| `cp_inflate` | function | `cp_inflate` | [x] |
| `cp_len_base` | data | `cp_len_base` | [x] |
| `cp_len_extra_bits` | data | `cp_len_extra_bits` | [x] |
| `cp_permutation_order` | data | `cp_permutation_order` | [x] |
| `unfilter` | function | `unfilter` | [x] |

Undefined C-library/runtime imports (`calloc`, `free`, `memcmp`, `memcpy`,
`memset`, `__assert_fail`, and weak ELF runtime hooks) are not library exports
and are therefore outside export parity.

Phase D was rerun after the final release build: C defined symbols = 9, Rust
defined symbols = 9, missing = 0, extra = 0. `ldd -r` reports no unresolved
symbols in the Rust shared object.
