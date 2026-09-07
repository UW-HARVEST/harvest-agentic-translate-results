# Configuration Surface

The sole public entry point is `max_size_frame(blocksize, channels, bitdepth)`.
The C implementation branches only on `channels == 2` and `bitdepth == 32`.
There are no runtime options, modes, flags, state objects, element types, byte
orders, formats, compile-time features, or additional entry points.

Each row includes fixed boundary cases plus many fixed-seed randomized inputs.
The randomized domains include zero, one, large values, and values that cause
intermediate `uint32_t` arithmetic to wrap.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `max_size_frame` | `channels == 2`, `bitdepth == 32`; `blocksize` spans the full `uint32_t` domain | [x] |
| 2 | `max_size_frame` | `channels == 2`, `bitdepth != 32`; `blocksize` and non-32 `bitdepth` span the full `uint32_t` domain | [x] |
| 3 | `max_size_frame` | `channels != 2`, `bitdepth == 32`; `blocksize` and non-2 `channels` span the full `uint32_t` domain | [x] |
| 4 | `max_size_frame` | `channels != 2`, `bitdepth != 32`; all three values span their applicable `uint32_t` domains | [x] |

## Feature combinations

`Cargo.toml` defines no features, so the only feature configuration is the
default/no-feature build.

- [x] Default feature invocation passes.
- [x] `--no-default-features` invocation passes.
