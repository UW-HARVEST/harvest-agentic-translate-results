# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options or conditional source selection. The complete valid feature set is:

| # | Cargo invocation feature selection | matching C configuration | status |
|---|------------------------------------|--------------------------|--------|
| B1 | `--no-default-features` (empty feature set) | default CMake configuration | [x] |

## Runtime Configurations

The public API has one entry point, one element type (`float`), and no option,
mode, flag, format, or byte-order parameter. Rows below are the pruned
cross-product of the source's actual distinctions:

- `size / 2` and the inclusive `-hsize..=hsize` loop distinguish skipped,
  negative-one, empty, one, odd-many, and even-many shapes.
- `v > 0.0f` distinguishes clamped tails from positive samples.
- `sum > 0.0f` distinguishes normalization from raw output.
- A finite broad radius and an infinite radius both avoid clamping, but the
  latter makes every sample equal because `rs == 0`.
- Zero, NaN, and sufficiently tiny nonzero radii all produce no positive sum,
  through `rs` being infinite or NaN.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| C1 | `gaussian_kernel` | `size <= -2`; any radius; loop skipped and destination untouched | [x] |
| C2 | `gaussian_kernel` | `size == -1`; zero/NaN/tiny radius; one raw zero written, no normalization loop | [x] |
| C3 | `gaussian_kernel` | `size == -1`; finite usable or infinite radius; one raw positive peak written, no normalization loop | [x] |
| C4 | `gaussian_kernel` | `size == 0`; zero/NaN/tiny radius; one raw zero written | [x] |
| C5 | `gaussian_kernel` | `size == 0`; finite usable or infinite radius; one raw positive peak written | [x] |
| C6 | `gaussian_kernel` | `size == 1`; zero/NaN/tiny radius; one zero written, normalization skipped | [x] |
| C7 | `gaussian_kernel` | `size == 1`; finite usable or infinite radius; one sample normalized to one | [x] |
| C8 | `gaussian_kernel` | odd `size >= 3`; zero/NaN/tiny radius; all samples zero, normalization skipped | [x] |
| C9 | `gaussian_kernel` | odd `size >= 3`; finite narrow radius; positive center and clamped tails, all `size` samples normalized | [x] |
| C10 | `gaussian_kernel` | odd `size >= 3`; finite broad radius; all samples positive and non-flat, all normalized | [x] |
| C11 | `gaussian_kernel` | odd `size >= 3`; infinite radius; flat positive samples normalized | [x] |
| C12 | `gaussian_kernel` | positive even `size`; zero/NaN/tiny radius; `size + 1` zero samples written, normalization skipped | [x] |
| C13 | `gaussian_kernel` | positive even `size`; finite narrow radius; `size + 1` samples written but only the first `size` normalized | [x] |
| C14 | `gaussian_kernel` | positive even `size`; finite broad radius; `size + 1` positive non-flat samples written but only the first `size` normalized | [x] |
| C15 | `gaussian_kernel` | positive even `size`; infinite radius; `size + 1` flat raw samples written but only the first `size` normalized | [x] |
| C16 | `gaussian_kernel` | oversized practical even size (`65536`); narrow finite radius; allocation and loop boundary case | [x] |

Each valid-path row must compare complete destination buffers byte-for-byte,
including guard elements that reveal over-writes or untouched output.
