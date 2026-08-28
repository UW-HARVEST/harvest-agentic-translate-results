# Configuration Surface

The public API has one entry point and no runtime options, modes, flags, element
types, byte-order choices, compile-time feature branches, or convenience
wrappers. The C implementation branches on:

- `size`: empty, one element, or many elements through its loops;
- accumulated `sum`: greater than zero (finite or infinity) versus comparison
  false (zero or NaN);
- buffer relationship: exactly aliased versus distinct in the non-positive
  branch, with forward/backward partial overlap affecting the positive branch.

Rows below are the meaningful source-derived combinations. Underflow,
overflow, signed zero, NaN, and infinity are listed separately because they
drive the same C branches with observably different bytes.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|--------|
| C1 | `normalize` | `size = 0`, exactly aliased valid buffers | [x] |
| C2 | `normalize` | `size = 0`, distinct valid buffers | [x] |
| C3 | `normalize` | one finite nonzero element, exactly aliased | [x] |
| C4 | `normalize` | one finite nonzero element, distinct buffers | [x] |
| C5 | `normalize` | one positive or negative zero, exactly aliased | [x] |
| C6 | `normalize` | one positive or negative zero, distinct buffers | [x] |
| C7 | `normalize` | one nonzero subnormal whose square underflows to zero, exactly aliased | [x] |
| C8 | `normalize` | one nonzero subnormal whose square underflows to zero, distinct buffers | [x] |
| C9 | `normalize` | one NaN (random payload/sign), exactly aliased | [x] |
| C10 | `normalize` | one NaN (random payload/sign), distinct buffers | [x] |
| C11 | `normalize` | one positive or negative infinity, exactly aliased | [x] |
| C12 | `normalize` | one positive or negative infinity, distinct buffers | [x] |
| C13 | `normalize` | many finite values with finite positive sum, exactly aliased | [x] |
| C14 | `normalize` | many finite values with finite positive sum, distinct buffers | [x] |
| C15 | `normalize` | many mixed positive/negative zeros, exactly aliased | [x] |
| C16 | `normalize` | many mixed positive/negative zeros, distinct buffers | [x] |
| C17 | `normalize` | many nonzero values whose squares all underflow to zero, exactly aliased | [x] |
| C18 | `normalize` | many nonzero values whose squares all underflow to zero, distinct buffers | [x] |
| C19 | `normalize` | many finite values whose accumulated sum overflows to infinity, exactly aliased | [x] |
| C20 | `normalize` | many finite values whose accumulated sum overflows to infinity, distinct buffers | [x] |
| C21 | `normalize` | many values including NaN, exactly aliased | [x] |
| C22 | `normalize` | many values including NaN, distinct buffers | [x] |
| C23 | `normalize` | many values including infinity, exactly aliased | [x] |
| C24 | `normalize` | many values including infinity, distinct buffers | [x] |
| C25 | `normalize` | many finite values, partial overlap with `dest` below `src` | [x] |
| C26 | `normalize` | many finite values, partial overlap with `dest` above `src` | [x] |
