# Configuration surface

The sole public entry point is:

```c
void tfm(float *dest, const float *src, int count);
```

The C source has no runtime options, modes, flags, enums, compile-time feature
branches, or convenience wrappers. Its observable axes are:

- loop cardinality: zero, one, or many records;
- top-level comparison: `src[0] < src[1]` true, or false because greater,
  equal, or unordered;
- clamp comparison: `0 > sqd` true, false, or unordered;
- floating-point class and bit behavior: ordinary finite values, signed zero,
  subnormal, overflow/infinity, and NaN payloads;
- pointer layout: disjoint or overlapping source/destination regions.

Each record consumes three `float` values and produces two.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|-----|
| C1 | `tfm` | `count == 0`; valid disjoint buffers remain byte-identical | [x] |
| C2 | `tfm` | `count == 1`; finite values; `src[0] < src[1]`; `sqd >= 0` | [x] |
| C3 | `tfm` | `count == 1`; finite values; `src[0] > src[1]`; `sqd >= 0` | [x] |
| C4 | `tfm` | `count == 1`; finite equal first/second values; false comparison arm | [x] |
| C5 | `tfm` | `count == 1`; finite values causing rounded `sqd < 0`; clamp-to-zero arm | [x] |
| C6 | `tfm` | `count == 1`; signed zeros and subnormal values in all positions | [x] |
| C7 | `tfm` | `count == 1`; infinities or finite overflow producing infinity/NaN intermediates | [x] |
| C8 | `tfm` | `count == 1`; NaN in `src[0]`, so top-level comparison is unordered/false | [x] |
| C9 | `tfm` | `count == 1`; NaN in `src[1]`, so top-level comparison is unordered/false | [x] |
| C10 | `tfm` | `count == 1`; NaN in `src[2]`, so `sqd` clamp comparison is unordered/false | [x] |
| C11 | `tfm` | `count > 1`; disjoint buffers; mixed comparison/clamp/value classes across records | [x] |
| C12 | `tfm` | `count == 1`; `dest == src` exact in-place overlap | [x] |
| C13 | `tfm` | `count > 1`; `dest == src`, with output storage overlapping each current/previous source record | [x] |
| C14 | `tfm` | `count > 1`; partially overlapping buffers (`dest` before or inside `src`) | [x] |

There is one Cargo feature combination: the default feature set. `Cargo.toml`
declares no named features.
