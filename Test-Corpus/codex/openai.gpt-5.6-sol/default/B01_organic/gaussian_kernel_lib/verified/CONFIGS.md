# Configuration surface

The public header exposes one entry point and no runtime options, modes, flags,
element-type choices, formats, byte-order choices, or feature gates.

The C implementation mechanically distinguishes these axes:

- `size / 2` controls whether the generation loop runs, whether its one-center
  case is normalized, and whether a positive size writes exactly `size` floats
  (odd) or `size + 1` floats (even).
- Floating-point arithmetic for `radius` determines whether generated values
  are positive, clipped to zero by `(v > 0) ? v : 0`, or become NaN and are
  therefore clipped to zero.
- `sum > 0` controls normalization of indices `0..size`; for positive even
  sizes, the extra generated coefficient is deliberately not normalized.

Rows below are the pruned cross-product of those source-visible behaviors.
“Randomized” means many fixed-seed values within the stated class.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `gaussian_kernel` | `size <= -2`; any radius; generation and normalization loops both skipped | [x] |
| C2 | `gaussian_kernel` | `size == -1`; finite nonzero radius with finite `1.6/radius`, or infinite radius; one raw positive center write, no normalization | [x] |
| C3 | `gaussian_kernel` | `size == -1`; signed zero, NaN, or finite radius so tiny that `1.6/radius` is infinite; one zero write, no normalization | [x] |
| C4 | `gaussian_kernel` | `size == 0`; finite nonzero radius with finite `1.6/radius`, or infinite radius; one raw positive center write, no normalization | [x] |
| C5 | `gaussian_kernel` | `size == 0`; signed zero, NaN, or finite radius so tiny that `1.6/radius` is infinite; one zero write, no normalization | [x] |
| C6 | `gaussian_kernel` | `size == 1`; finite nonzero radius with finite `1.6/radius`, or infinite radius; one positive center write normalized to one | [x] |
| C7 | `gaussian_kernel` | `size == 1`; signed zero, NaN, or finite radius so tiny that `1.6/radius` is infinite; one zero write and no normalization | [x] |
| C8 | `gaussian_kernel` | positive odd `size >= 3`; randomized finite signed radii broad enough that every generated coefficient is positive | [x] |
| C9 | `gaussian_kernel` | positive odd `size >= 3`; randomized finite signed radii producing both positive and clipped-zero coefficients | [x] |
| C10 | `gaussian_kernel` | positive odd `size >= 3`; randomized finite signed radii producing only a positive center coefficient | [x] |
| C11 | `gaussian_kernel` | positive odd `size >= 3`; finite subnormal radii for which `1.6/radius` is infinite; every coefficient is zero | [x] |
| C12 | `gaussian_kernel` | positive odd `size >= 3`; signed zero radius; every coefficient is zero | [x] |
| C13 | `gaussian_kernel` | positive odd `size >= 3`; randomized NaN payload/sign radius; every coefficient is zero | [x] |
| C14 | `gaussian_kernel` | positive odd `size >= 3`; positive or negative infinite radius; uniform coefficients normalized over `size` elements | [x] |
| C15 | `gaussian_kernel` | positive even `size >= 2`; randomized finite signed radii broad enough that every coefficient is positive; `size + 1` generated and only first `size` normalized | [x] |
| C16 | `gaussian_kernel` | positive even `size >= 2`; randomized finite signed radii producing positive and clipped-zero coefficients; extra coefficient remains unnormalized | [x] |
| C17 | `gaussian_kernel` | positive even `size >= 2`; randomized finite signed radii producing only a positive center coefficient; extra coefficient remains zero | [x] |
| C18 | `gaussian_kernel` | positive even `size >= 2`; finite subnormal radii for which `1.6/radius` is infinite; all `size + 1` coefficients are zero | [x] |
| C19 | `gaussian_kernel` | positive even `size >= 2`; signed zero radius; all `size + 1` coefficients are zero | [x] |
| C20 | `gaussian_kernel` | positive even `size >= 2`; randomized NaN payload/sign radius; all `size + 1` coefficients are zero | [x] |
| C21 | `gaussian_kernel` | positive even `size >= 2`; positive or negative infinite radius; first `size` coefficients normalized and extra coefficient left raw | [x] |

There is one Cargo configuration: default/no features. `Cargo.toml` declares
no `[features]` table, so `--no-default-features` is behaviorally the same
configuration and is still run at the completion gate.
