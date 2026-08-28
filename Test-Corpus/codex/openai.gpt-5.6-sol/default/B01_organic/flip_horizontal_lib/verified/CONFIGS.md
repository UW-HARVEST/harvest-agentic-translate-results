# Configuration Surface

The public header exposes one entry point and no options, flags, modes, enums,
formats, element-type choices, byte-order choices, or compile-time features.
`cp_pixel_t` is always four bytes in `r`, `g`, `b`, `a` order.

The C loop structure distinguishes these valid image-shape axes:

- width: zero (inner loop never executes), one, or many;
- height: zero, one, even and at least two, or odd and at least three.

The table is their complete meaningful cross-product. Randomized dimensions
and pixel bytes within each row are required before checking it.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `flip_horizontal` | no options; width zero, height zero | [x] |
| 2 | `flip_horizontal` | no options; width zero, height one | [x] |
| 3 | `flip_horizontal` | no options; width zero, positive even height | [x] |
| 4 | `flip_horizontal` | no options; width zero, odd height at least three | [x] |
| 5 | `flip_horizontal` | no options; width one, height zero | [x] |
| 6 | `flip_horizontal` | no options; width one, height one | [x] |
| 7 | `flip_horizontal` | no options; width one, positive even height | [x] |
| 8 | `flip_horizontal` | no options; width one, odd height at least three | [x] |
| 9 | `flip_horizontal` | no options; width many, height zero | [x] |
| 10 | `flip_horizontal` | no options; width many, height one | [x] |
| 11 | `flip_horizontal` | no options; width many, positive even height | [x] |
| 12 | `flip_horizontal` | no options; width many, odd height at least three | [x] |
