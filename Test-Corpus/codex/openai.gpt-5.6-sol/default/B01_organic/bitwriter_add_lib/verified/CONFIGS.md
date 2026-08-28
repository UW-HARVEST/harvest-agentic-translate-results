# Configuration Surface

The public API has one entry point, no runtime options, no modes, no flags, no
element types, no formats, no byte-order setting, and no compile-time Cargo
features. `pos`, `len`, and `buffer` are present in the public state but are
neither read nor written by the C function.

For valid bit-writer states (`bw.bits <= 63`) and positive widths
(`bits <= 64`), the source distinguishes the rows below. The exact-boundary
and over-boundary loop cases are separate because they leave different
remaining widths for the final write. Initial `tot` values that do and do not
wrap are randomized within every row; the C code does not branch on `tot`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `bitwriter_add` | zero width: `bits == 0`, `bw.bits` in `0..=63`; loop not entered | [x] |
| 2 | `bitwriter_add` | positive width with `bw.bits + bits < 64`; loop not entered | [x] |
| 3 | `bitwriter_add` | `bw.bits` in `0..=62`, `bw.bits + bits == 64`; first loop transfer is positive, then the 100-iteration cap terminates zero-progress iterations | [x] |
| 4 | `bitwriter_add` | `bw.bits` in `0..=62`, `bw.bits + bits > 64`, `bits <= 64`; first loop transfer is positive, then the cap terminates zero-progress iterations | [x] |
| 5 | `bitwriter_add` | `bw.bits == 63`, positive `bits <= 64`; every loop transfer is zero and the 100-iteration cap terminates the loop | [x] |

No Cargo features are declared.

- [x] Default configuration
- [x] `--no-default-features`
