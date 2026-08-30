# Configuration Surface

Mechanically derived from the sole public header entry point and every C
control-flow branch in `smallestValue`: `if (head)`, `while (head->next)`, and
`if (head->value < smallest)`. The API has no runtime options, modes, flags,
feature gates, enums, lengths, element-type choices, formats, or byte-order
settings. A valid input is a non-null, readable, null-terminated linked list of
C `int` values.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `smallestValue` | singleton list; traversal loop is not entered | [x] |
| 2 | `smallestValue` | multi-node list; every successor is greater than the running minimum, so the strict-less branch is never taken | [x] |
| 3 | `smallestValue` | multi-node list; equal values exercise strict-less equality without updating the minimum | [x] |
| 4 | `smallestValue` | multi-node list; one or more successors lower the running minimum, with randomized branch order and minimum position | [x] |
| 5 | `smallestValue` | singleton and multi-node lists containing the full C `int` boundaries `INT_MIN` and `INT_MAX` | [x] |
