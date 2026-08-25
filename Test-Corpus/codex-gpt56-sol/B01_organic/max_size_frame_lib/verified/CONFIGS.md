# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no build
options or conditional sources. There is one valid configuration:

| # | Cargo invocation | CMake configuration |
|---|------------------|---------------------|
| 1 | `--no-default-features --features ''` | Default source set |

## Runtime and Input Configurations

The only public entry point is the low-level function `max_size_frame`. It has
no mutable state or runtime options. The rows below are the cross-product of
the branches in `c_src/src/lib.c` and ordinary versus wrapping `uint32_t`
arithmetic. Boundary sets include zero, one, `UINT32_MAX`, and values adjacent
to branch constants.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `max_size_frame` | `channels != 2`; ordinary arithmetic; randomized `blocksize`, `channels` (including 0, 1, and values > 2), and `bitdepth` | [x] |
| 2 | `max_size_frame` | `channels != 2`; multiplication/addition wraps `uint32_t`; randomized high values and exact boundaries | [x] |
| 3 | `max_size_frame` | `channels == 2`, `bitdepth == 32`; ordinary arithmetic | [x] |
| 4 | `max_size_frame` | `channels == 2`, `bitdepth == 32`; multiplication/addition wraps `uint32_t`; randomized high `blocksize` values | [x] |
| 5 | `max_size_frame` | `channels == 2`, `bitdepth != 32`; ordinary arithmetic, including `bitdepth` 0, 1, 31, and 33 | [x] |
| 6 | `max_size_frame` | `channels == 2`, `bitdepth != 32`; multiplication/addition wraps `uint32_t`, including `bitdepth == UINT32_MAX` where `bitdepth + 1` wraps | [x] |
