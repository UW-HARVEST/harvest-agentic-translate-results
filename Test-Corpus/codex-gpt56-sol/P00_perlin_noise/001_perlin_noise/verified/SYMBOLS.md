# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver.so`

Extraction command:

```text
nm -D --defined-only c_src/build/libdriver.so
```

The CMake-produced PIC object was linked as a shared object without changing
`c_src/`. Weak ELF runtime symbols and undefined libc symbols are excluded;
the table contains every defined public symbol reported by `nm -D`.

| # | C symbol | Type | Rust parity | Translation status |
|---|----------|------|-------------|--------------------|
| 1 | `stb_perlin_noise3_internal` | `T` | [x] | Exported and differentially tested |
| 2 | `stb_perlin_noise3` | `T` | [x] | Exported and differentially tested |
| 3 | `stb_perlin_noise3_seed` | `T` | [x] | Exported and differentially tested |
| 4 | `stb_perlin_ridge_noise3` | `T` | [x] | Exported and differentially tested |
| 5 | `stb_perlin_fbm_noise3` | `T` | [x] | Exported and differentially tested |
| 6 | `stb_perlin_turbulence_noise3` | `T` | [x] | Exported and differentially tested |
| 7 | `stb_perlin_noise3_wrap_nonpow2` | `T` | [x] | Exported and differentially tested |
| 8 | `inner` | `T` | [x] | Exported and differentially tested |
| 9 | `main` | `T` | [x] | Exported and byte-tested through isolated stdin/stdout |

Completion criterion: all nine rows checked and the C-to-Rust defined-symbol
set difference is empty.
