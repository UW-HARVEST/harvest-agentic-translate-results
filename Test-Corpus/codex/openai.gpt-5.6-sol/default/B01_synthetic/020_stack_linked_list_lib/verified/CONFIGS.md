# Configuration Surface

The public API has one entry point and no runtime options, modes, flags,
element-type choices, format choices, byte-order choices, feature declarations,
or higher-level wrappers. The mechanically visible valid-input axes are list
length, the outcome sequence of `head->value < smallest`, duplicate values,
and the full C `int` boundary values.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `smallestValue` | singleton list; randomized ordinary `int` value (loop executes zero times) | [x] |
| 2 | `smallestValue` | singleton list; `INT_MIN` or `INT_MAX` | [x] |
| 3 | `smallestValue` | two-node list; second value is greater than or equal to the head (comparison false) | [x] |
| 4 | `smallestValue` | two-node list; second value is less than the head (comparison true) | [x] |
| 5 | `smallestValue` | many-node nondecreasing/equal list; every comparison is false | [x] |
| 6 | `smallestValue` | many-node strictly decreasing list; every comparison is true | [x] |
| 7 | `smallestValue` | many-node mixed list; true and false comparison outcomes, including duplicate minima | [x] |
| 8 | `smallestValue` | many-node list containing `INT_MIN`, `INT_MAX`, and randomized interior values | [x] |
