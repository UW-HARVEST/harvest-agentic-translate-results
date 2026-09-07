# Configuration Surface

The public header exposes only `void driver(int floors)`. The implementation
has no runtime options, modes, flags, conditionals, switches, or feature
branches. It always serializes the native C representation of a zero-initialized
`house_t` with the caller's `floors`, `bedrooms = 3`, and `bathrooms = 2.0`,
then prints every byte as two lowercase hexadecimal digits followed by a
newline.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver` | No options; by-value native `int` across its full domain, including zero, positive, negative, `INT_MIN`, and `INT_MAX`; fixed native `house_t` layout and byte order | [x] |

There are no Cargo features and CMake defines no conditional build
configurations, so this is the complete feature/configuration cross-product.
