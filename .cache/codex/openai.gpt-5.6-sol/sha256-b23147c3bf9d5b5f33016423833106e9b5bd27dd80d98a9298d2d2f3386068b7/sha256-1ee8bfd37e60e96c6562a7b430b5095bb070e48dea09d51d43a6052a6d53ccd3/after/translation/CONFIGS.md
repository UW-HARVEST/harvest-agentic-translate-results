# Configuration Surface

The public API has one entry point, no runtime options, no compile-time feature
flags, and one input element type (`float`). The rows below are the
cross-product pruned to behavior distinguished by the loop condition
(`i < count`), the data comparison (`src[0] < src[1]`), and repeated pointer
advancement.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tfm` | Negative `count`; null pointers accepted because the loop does not execute | [x] |
| 2 | `tfm` | Zero `count`; null pointers accepted because the loop does not execute | [x] |
| 3 | `tfm` | One triple with `src[0] < src[1]` (first branch), randomized finite and non-finite values | [x] |
| 4 | `tfm` | One triple with `src[0] >= src[1]` (else branch), including equal values and signed zero | [x] |
| 5 | `tfm` | One triple with an unordered comparison caused by NaN (else branch), including varied NaN payloads | [x] |
| 6 | `tfm` | Many triples with mixed first/else/unordered branches and contiguous source/destination pointer advancement | [x] |
| 7 | `tfm` | Many triples with valid overlapping source and destination regions at several relative offsets | [x] |
