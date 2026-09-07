# Configuration Surface

Derived from every exported C entry point and every comparison/switch branch in
`c_src/src/lib.c`. The C library has no compile-time Cargo feature equivalent,
runtime option object, byte-order mode, format selector, or length/count input.
Its varying shapes are IEEE-754 `float` values, geometric relative positions,
and the `c2Collided` type selector.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `c2V` | arbitrary finite components, including positive and negative values | [x] |
| C02 | `c2V` | IEEE exceptional components: signed zero, infinities, and NaNs | [x] |
| C03 | `c2Mulvs` | finite vector and positive finite scalar | [x] |
| C04 | `c2Mulvs` | finite vector and negative or signed-zero scalar | [x] |
| C05 | `c2Mulvs` | overflow, infinity, or NaN operands | [x] |
| C06 | `c2Maxv` | `a.x > b.x` / `a.y > b.y` comparison true | [x] |
| C07 | `c2Maxv` | comparison false because `a < b` or equal, including signed zero | [x] |
| C08 | `c2Maxv` | unordered comparison because either operand is NaN | [x] |
| C09 | `c2Minv` | `a.x < b.x` / `a.y < b.y` comparison true | [x] |
| C10 | `c2Minv` | comparison false because `a > b` or equal, including signed zero | [x] |
| C11 | `c2Minv` | unordered comparison because either operand is NaN | [x] |
| C12 | `c2Clampv` | each component below its lower bound | [x] |
| C13 | `c2Clampv` | each component inside inclusive bounds | [x] |
| C14 | `c2Clampv` | each component above its upper bound | [x] |
| C15 | `c2Clampv` | mixed below/inside/above states across x and y | [x] |
| C16 | `c2Clampv` | reversed/equal bounds and NaN operands | [x] |
| C17 | `c2Sub` | arbitrary finite operands, including cancellation and signed zero | [x] |
| C18 | `c2Sub` | overflow, infinity, and NaN operands | [x] |
| C19 | `c2Dot` | arbitrary finite operands and cancellation | [x] |
| C20 | `c2Dot` | overflow, underflow, infinity, and NaN operands | [x] |
| C21 | `c2CircletoCircle` | center distance strictly less than squared summed radius | [x] |
| C22 | `c2CircletoCircle` | exact tangency (`d2 == r2`), which returns false | [x] |
| C23 | `c2CircletoCircle` | separated circles (`d2 > r2`) | [x] |
| C24 | `c2CircletoCircle` | zero/negative radii, overflow, or NaN values | [x] |
| C25 | `c2CircletoAABB` | center inside/on AABB so clamped point equals center | [x] |
| C26 | `c2CircletoAABB` | center outside one face/edge | [x] |
| C27 | `c2CircletoAABB` | center outside a corner in both axes | [x] |
| C28 | `c2CircletoAABB` | exact tangency, zero/negative radius, or degenerate/reversed bounds | [x] |
| C29 | `c2CircletoAABB` | infinity or NaN in circle/AABB inputs | [x] |
| C30 | `c2CircletoCapsule` | `da < 0`: closest to endpoint `a` | [x] |
| C31 | `c2CircletoCapsule` | `da >= 0` and `db < 0`: closest to segment interior | [x] |
| C32 | `c2CircletoCapsule` | `da >= 0` and `db >= 0`: closest to endpoint `b` | [x] |
| C33 | `c2CircletoCapsule` | exact tangency and zero/negative radii | [x] |
| C34 | `c2CircletoCapsule` | degenerate capsule (`a == b`) | [x] |
| C35 | `c2CircletoCapsule` | overflow, infinity, or NaN inputs | [x] |
| C36 | `c2Collided` | valid selector 0 with correctly shaped circle pointers | [x] |
| C37 | `c2Collided` | valid selector 1 with circle/AABB pointers | [x] |
| C38 | `c2Collided` | valid selector 2 with circle/capsule pointers | [x] |
| C39 | `circle_collide` | finite x/y with positive radius across all three fixed target shapes | [x] |
| C40 | `circle_collide` | zero/negative radius and IEEE exceptional x/y/r values | [x] |

## Public call hierarchy

```text
circle_collide
└── c2Collided (selectors 0, 1, 2)
    ├── c2CircletoCircle
    ├── c2CircletoAABB
    └── c2CircletoCapsule

collision functions
└── c2V / c2Mulvs / c2Maxv / c2Minv / c2Clampv / c2Sub / c2Dot
```
