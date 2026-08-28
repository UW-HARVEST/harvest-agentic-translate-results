# Configuration Surface

The public API contains one entry point and no runtime options, mutable state,
feature flags, element types, formats, or byte-order modes. Its implementation
branches on two input predicates:

- `channels == 2` versus `channels != 2`
- `bitdepth == 32` versus `bitdepth != 32`

`blocksize` has no branch, so every row exercises randomized values plus `0`,
`1`, `UINT32_MAX - 1`, and `UINT32_MAX`. Non-equal classes include their scalar
boundaries and values adjacent to the special value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `max_size_frame` | `channels == 2`, `bitdepth == 32`; randomized and boundary `blocksize` | [x] |
| 2 | `max_size_frame` | `channels == 2`, `bitdepth != 32`; randomized and boundary `blocksize` and bit depth | [x] |
| 3 | `max_size_frame` | `channels != 2`, `bitdepth == 32`; randomized and boundary `blocksize` and channel count | [x] |
| 4 | `max_size_frame` | `channels != 2`, `bitdepth != 32`; randomized and boundary values for all scalar classes | [x] |
