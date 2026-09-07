# Error Surface

Mechanical source audit:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|if[[:space:]]*\(|switch[[:space:]]*\(|NULL|limit' ../c_src/include ../c_src/src
```

The public function has no null checks, enum checks, range checks, assertions,
or error return codes. Its sole explicit rejection branch is in the private
bit reader and is observable through `dequantize_granule`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E01 | `dequantize_granule` via `get_bits` | After adding the requested bit count, `bs->pos > bs->limit` | That read yields `0`; `bs->pos` remains advanced; decoding continues; the public return remains `group_size * 4` |

## Generic FFI boundary coverage

These cases are required by the verification protocol even though the C source
does not reject them explicitly.

| # | boundary | expected comparison |
|---|----------|---------------------|
| G01 | null `grbuf` with an active output write | C and Rust terminate/reject identically at the FFI boundary |
| G02 | null `bs` with an active bit read | C and Rust terminate/reject identically at the FFI boundary |
| G03 | null `sci` | C and Rust terminate/reject identically at the FFI boundary |
| G04 | zero `group_size` | Same return, state changes, and no output writes |
| G05 | zero `total_bands` | Same return, state changes, and no band reads/writes |
| G06 | zero bitstream limit with a nonzero read | Same zero-read substitution, position advance, output, and return |
| G07 | very large in-bounds bitstream limit | Same normal decode behavior |
| G08 | maximum source-defined band count that keeps `2 * total_bands` within `bitalloc[64]` (`total_bands = 32`) | Same output, state, and return |
| G09 | one-past in-array band count (`total_bands = 33`), which makes C continue from `bitalloc` into the immediately adjacent struct storage | Same observed output, state, and return |
| G10 | oversized but allocated `group_size` (`128`) | Same output, state, and return |

## Completion

- [x] E01
- [x] G01
- [x] G02
- [x] G03
- [x] G04
- [x] G05
- [x] G06
- [x] G07
- [x] G08
- [x] G09
- [x] G10
