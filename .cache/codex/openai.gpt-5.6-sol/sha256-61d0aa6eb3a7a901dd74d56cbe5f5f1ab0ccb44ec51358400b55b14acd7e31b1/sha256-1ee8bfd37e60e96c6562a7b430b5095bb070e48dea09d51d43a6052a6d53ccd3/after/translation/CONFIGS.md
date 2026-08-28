# Configuration Surface

The rows below come from every dynamic C entry point and every `if`,
`switch`, shape-kind, pointer-option, radius-option, cache-state, and
input-count branch in `c_src/src/lib.c`. Randomized tests use a fixed seed.
No Cargo features are declared, so the effective feature matrix is default
and `--no-default-features` (behaviorally identical, both still tested).

| # | entry point(s) | configuration (options set + input shape) | status |
|---:|----------------|--------------------------------------------|-----|
| C001 | `c2V`, `c2Mulvs`, `c2Sub`, `c2Dot`, `c2Add`, `c2Neg`, `c2Skew`, `c2CCW90` | random finite vectors/scalars, including signs and zero | [x] |
| C002 | `c2Maxv` | a.x > b.x; a.y > b.y | [x] |
| C003 | `c2Maxv` | a.x > b.x; a.y <= b.y | [x] |
| C004 | `c2Maxv` | a.x <= b.x; a.y > b.y | [x] |
| C005 | `c2Maxv` | a.x <= b.x; a.y <= b.y | [x] |
| C006 | `c2Minv` | a.x < b.x; a.y < b.y | [x] |
| C007 | `c2Minv` | a.x < b.x; a.y >= b.y | [x] |
| C008 | `c2Minv` | a.x >= b.x; a.y < b.y | [x] |
| C009 | `c2Minv` | a.x >= b.x; a.y >= b.y | [x] |
| C010 | `c2Clampv` | x below [lo, hi]; y below [lo, hi] | [x] |
| C011 | `c2Clampv` | x below [lo, hi]; y inside [lo, hi] | [x] |
| C012 | `c2Clampv` | x below [lo, hi]; y above [lo, hi] | [x] |
| C013 | `c2Clampv` | x inside [lo, hi]; y below [lo, hi] | [x] |
| C014 | `c2Clampv` | x inside [lo, hi]; y inside [lo, hi] | [x] |
| C015 | `c2Clampv` | x inside [lo, hi]; y above [lo, hi] | [x] |
| C016 | `c2Clampv` | x above [lo, hi]; y below [lo, hi] | [x] |
| C017 | `c2Clampv` | x above [lo, hi]; y inside [lo, hi] | [x] |
| C018 | `c2Clampv` | x above [lo, hi]; y above [lo, hi] | [x] |
| C019 | `c2RotIdentity`, `c2xIdentity` | zero-argument identity constructors | [x] |
| C020 | `c2BBVerts` | finite AABB; four output vertices | [x] |
| C021 | `c2MakeProxy` | shape type circle | [x] |
| C022 | `c2MakeProxy` | shape type AABB | [x] |
| C023 | `c2MakeProxy` | shape type capsule | [x] |
| C024 | `c2Len`, `c2Det2` | random finite vectors, including zero | [x] |
| C025 | `c2GJKSimplexMetric` | simplex count 1 | [x] |
| C026 | `c2GJKSimplexMetric` | simplex count 2 | [x] |
| C027 | `c2GJKSimplexMetric` | simplex count 3 | [x] |
| C028 | `c2GJKSimplexMetric` | simplex count default/out-of-range | [x] |
| C029 | `c2Mulrv`, `c2Mulxv`, `c2MulrvT` | random finite rotations, translations, and vectors | [x] |
| C030 | `c22` | v <= 0 (vertex a) | [x] |
| C031 | `c22` | v > 0 and u <= 0 (vertex b) | [x] |
| C032 | `c22` | u > 0 and v > 0 (edge ab) | [x] |
| C033 | `c23` | vAB <= 0 and uCA <= 0 (vertex a) | [x] |
| C034 | `c23` | uAB <= 0 and vBC <= 0 (vertex b) | [x] |
| C035 | `c23` | uBC <= 0 and vCA <= 0 (vertex c) | [x] |
| C036 | `c23` | uAB > 0 and vAB > 0 and wABC <= 0 (edge ab) | [x] |
| C037 | `c23` | uBC > 0 and vBC > 0 and uABC <= 0 (edge bc) | [x] |
| C038 | `c23` | uCA > 0 and vCA > 0 and vABC <= 0 (edge ca) | [x] |
| C039 | `c23` | remaining region (triangle abc) | [x] |
| C040 | `c2D` | count 1 | [x] |
| C041 | `c2D` | count 2 with positive determinant | [x] |
| C042 | `c2D` | count 2 with nonpositive determinant | [x] |
| C043 | `c2D` | count 3/default | [x] |
| C044 | `c2Support` | count 1 | [x] |
| C045 | `c2Support` | count many with unique strict maximum | [x] |
| C046 | `c2Support` | count many with tied maximum; first index wins | [x] |
| C047 | `c2Witness` | simplex count 1 | [x] |
| C048 | `c2Witness` | simplex count 2 | [x] |
| C049 | `c2Witness` | simplex count 3 | [x] |
| C050 | `c2Witness` | simplex count default/out-of-range | [x] |
| C051 | `c2Div` | random finite vector and nonzero divisor | [x] |
| C052 | `c2Norm` | random finite nonzero vector | [x] |
| C053 | `c2L` | simplex count 1 | [x] |
| C054 | `c2L` | simplex count 2 | [x] |
| C055 | `c2L` | simplex count default/out-of-range | [x] |
| C056 | `c2GJK` | A=circle; B=circle; ax=null, bx=null; use_radius=0; cache=null | [x] |
| C057 | `c2GJK` | A=circle; B=circle; ax=null, bx=null; use_radius=0; cache=empty | [x] |
| C058 | `c2GJK` | A=circle; B=circle; ax=null, bx=null; use_radius=0; cache=warm | [x] |
| C059 | `c2GJK` | A=circle; B=circle; ax=null, bx=null; use_radius=1; cache=null | [x] |
| C060 | `c2GJK` | A=circle; B=circle; ax=null, bx=null; use_radius=1; cache=empty | [x] |
| C061 | `c2GJK` | A=circle; B=circle; ax=null, bx=null; use_radius=1; cache=warm | [x] |
| C062 | `c2GJK` | A=circle; B=circle; ax=set, bx=null; use_radius=0; cache=null | [x] |
| C063 | `c2GJK` | A=circle; B=circle; ax=set, bx=null; use_radius=0; cache=empty | [x] |
| C064 | `c2GJK` | A=circle; B=circle; ax=set, bx=null; use_radius=0; cache=warm | [x] |
| C065 | `c2GJK` | A=circle; B=circle; ax=set, bx=null; use_radius=1; cache=null | [x] |
| C066 | `c2GJK` | A=circle; B=circle; ax=set, bx=null; use_radius=1; cache=empty | [x] |
| C067 | `c2GJK` | A=circle; B=circle; ax=set, bx=null; use_radius=1; cache=warm | [x] |
| C068 | `c2GJK` | A=circle; B=circle; ax=null, bx=set; use_radius=0; cache=null | [x] |
| C069 | `c2GJK` | A=circle; B=circle; ax=null, bx=set; use_radius=0; cache=empty | [x] |
| C070 | `c2GJK` | A=circle; B=circle; ax=null, bx=set; use_radius=0; cache=warm | [x] |
| C071 | `c2GJK` | A=circle; B=circle; ax=null, bx=set; use_radius=1; cache=null | [x] |
| C072 | `c2GJK` | A=circle; B=circle; ax=null, bx=set; use_radius=1; cache=empty | [x] |
| C073 | `c2GJK` | A=circle; B=circle; ax=null, bx=set; use_radius=1; cache=warm | [x] |
| C074 | `c2GJK` | A=circle; B=circle; ax=set, bx=set; use_radius=0; cache=null | [x] |
| C075 | `c2GJK` | A=circle; B=circle; ax=set, bx=set; use_radius=0; cache=empty | [x] |
| C076 | `c2GJK` | A=circle; B=circle; ax=set, bx=set; use_radius=0; cache=warm | [x] |
| C077 | `c2GJK` | A=circle; B=circle; ax=set, bx=set; use_radius=1; cache=null | [x] |
| C078 | `c2GJK` | A=circle; B=circle; ax=set, bx=set; use_radius=1; cache=empty | [x] |
| C079 | `c2GJK` | A=circle; B=circle; ax=set, bx=set; use_radius=1; cache=warm | [x] |
| C080 | `c2GJK` | A=circle; B=AABB; ax=null, bx=null; use_radius=0; cache=null | [x] |
| C081 | `c2GJK` | A=circle; B=AABB; ax=null, bx=null; use_radius=0; cache=empty | [x] |
| C082 | `c2GJK` | A=circle; B=AABB; ax=null, bx=null; use_radius=0; cache=warm | [x] |
| C083 | `c2GJK` | A=circle; B=AABB; ax=null, bx=null; use_radius=1; cache=null | [x] |
| C084 | `c2GJK` | A=circle; B=AABB; ax=null, bx=null; use_radius=1; cache=empty | [x] |
| C085 | `c2GJK` | A=circle; B=AABB; ax=null, bx=null; use_radius=1; cache=warm | [x] |
| C086 | `c2GJK` | A=circle; B=AABB; ax=set, bx=null; use_radius=0; cache=null | [x] |
| C087 | `c2GJK` | A=circle; B=AABB; ax=set, bx=null; use_radius=0; cache=empty | [x] |
| C088 | `c2GJK` | A=circle; B=AABB; ax=set, bx=null; use_radius=0; cache=warm | [x] |
| C089 | `c2GJK` | A=circle; B=AABB; ax=set, bx=null; use_radius=1; cache=null | [x] |
| C090 | `c2GJK` | A=circle; B=AABB; ax=set, bx=null; use_radius=1; cache=empty | [x] |
| C091 | `c2GJK` | A=circle; B=AABB; ax=set, bx=null; use_radius=1; cache=warm | [x] |
| C092 | `c2GJK` | A=circle; B=AABB; ax=null, bx=set; use_radius=0; cache=null | [x] |
| C093 | `c2GJK` | A=circle; B=AABB; ax=null, bx=set; use_radius=0; cache=empty | [x] |
| C094 | `c2GJK` | A=circle; B=AABB; ax=null, bx=set; use_radius=0; cache=warm | [x] |
| C095 | `c2GJK` | A=circle; B=AABB; ax=null, bx=set; use_radius=1; cache=null | [x] |
| C096 | `c2GJK` | A=circle; B=AABB; ax=null, bx=set; use_radius=1; cache=empty | [x] |
| C097 | `c2GJK` | A=circle; B=AABB; ax=null, bx=set; use_radius=1; cache=warm | [x] |
| C098 | `c2GJK` | A=circle; B=AABB; ax=set, bx=set; use_radius=0; cache=null | [x] |
| C099 | `c2GJK` | A=circle; B=AABB; ax=set, bx=set; use_radius=0; cache=empty | [x] |
| C100 | `c2GJK` | A=circle; B=AABB; ax=set, bx=set; use_radius=0; cache=warm | [x] |
| C101 | `c2GJK` | A=circle; B=AABB; ax=set, bx=set; use_radius=1; cache=null | [x] |
| C102 | `c2GJK` | A=circle; B=AABB; ax=set, bx=set; use_radius=1; cache=empty | [x] |
| C103 | `c2GJK` | A=circle; B=AABB; ax=set, bx=set; use_radius=1; cache=warm | [x] |
| C104 | `c2GJK` | A=circle; B=capsule; ax=null, bx=null; use_radius=0; cache=null | [x] |
| C105 | `c2GJK` | A=circle; B=capsule; ax=null, bx=null; use_radius=0; cache=empty | [x] |
| C106 | `c2GJK` | A=circle; B=capsule; ax=null, bx=null; use_radius=0; cache=warm | [x] |
| C107 | `c2GJK` | A=circle; B=capsule; ax=null, bx=null; use_radius=1; cache=null | [x] |
| C108 | `c2GJK` | A=circle; B=capsule; ax=null, bx=null; use_radius=1; cache=empty | [x] |
| C109 | `c2GJK` | A=circle; B=capsule; ax=null, bx=null; use_radius=1; cache=warm | [x] |
| C110 | `c2GJK` | A=circle; B=capsule; ax=set, bx=null; use_radius=0; cache=null | [x] |
| C111 | `c2GJK` | A=circle; B=capsule; ax=set, bx=null; use_radius=0; cache=empty | [x] |
| C112 | `c2GJK` | A=circle; B=capsule; ax=set, bx=null; use_radius=0; cache=warm | [x] |
| C113 | `c2GJK` | A=circle; B=capsule; ax=set, bx=null; use_radius=1; cache=null | [x] |
| C114 | `c2GJK` | A=circle; B=capsule; ax=set, bx=null; use_radius=1; cache=empty | [x] |
| C115 | `c2GJK` | A=circle; B=capsule; ax=set, bx=null; use_radius=1; cache=warm | [x] |
| C116 | `c2GJK` | A=circle; B=capsule; ax=null, bx=set; use_radius=0; cache=null | [x] |
| C117 | `c2GJK` | A=circle; B=capsule; ax=null, bx=set; use_radius=0; cache=empty | [x] |
| C118 | `c2GJK` | A=circle; B=capsule; ax=null, bx=set; use_radius=0; cache=warm | [x] |
| C119 | `c2GJK` | A=circle; B=capsule; ax=null, bx=set; use_radius=1; cache=null | [x] |
| C120 | `c2GJK` | A=circle; B=capsule; ax=null, bx=set; use_radius=1; cache=empty | [x] |
| C121 | `c2GJK` | A=circle; B=capsule; ax=null, bx=set; use_radius=1; cache=warm | [x] |
| C122 | `c2GJK` | A=circle; B=capsule; ax=set, bx=set; use_radius=0; cache=null | [x] |
| C123 | `c2GJK` | A=circle; B=capsule; ax=set, bx=set; use_radius=0; cache=empty | [x] |
| C124 | `c2GJK` | A=circle; B=capsule; ax=set, bx=set; use_radius=0; cache=warm | [x] |
| C125 | `c2GJK` | A=circle; B=capsule; ax=set, bx=set; use_radius=1; cache=null | [x] |
| C126 | `c2GJK` | A=circle; B=capsule; ax=set, bx=set; use_radius=1; cache=empty | [x] |
| C127 | `c2GJK` | A=circle; B=capsule; ax=set, bx=set; use_radius=1; cache=warm | [x] |
| C128 | `c2GJK` | A=AABB; B=circle; ax=null, bx=null; use_radius=0; cache=null | [x] |
| C129 | `c2GJK` | A=AABB; B=circle; ax=null, bx=null; use_radius=0; cache=empty | [x] |
| C130 | `c2GJK` | A=AABB; B=circle; ax=null, bx=null; use_radius=0; cache=warm | [x] |
| C131 | `c2GJK` | A=AABB; B=circle; ax=null, bx=null; use_radius=1; cache=null | [x] |
| C132 | `c2GJK` | A=AABB; B=circle; ax=null, bx=null; use_radius=1; cache=empty | [x] |
| C133 | `c2GJK` | A=AABB; B=circle; ax=null, bx=null; use_radius=1; cache=warm | [x] |
| C134 | `c2GJK` | A=AABB; B=circle; ax=set, bx=null; use_radius=0; cache=null | [x] |
| C135 | `c2GJK` | A=AABB; B=circle; ax=set, bx=null; use_radius=0; cache=empty | [x] |
| C136 | `c2GJK` | A=AABB; B=circle; ax=set, bx=null; use_radius=0; cache=warm | [x] |
| C137 | `c2GJK` | A=AABB; B=circle; ax=set, bx=null; use_radius=1; cache=null | [x] |
| C138 | `c2GJK` | A=AABB; B=circle; ax=set, bx=null; use_radius=1; cache=empty | [x] |
| C139 | `c2GJK` | A=AABB; B=circle; ax=set, bx=null; use_radius=1; cache=warm | [x] |
| C140 | `c2GJK` | A=AABB; B=circle; ax=null, bx=set; use_radius=0; cache=null | [x] |
| C141 | `c2GJK` | A=AABB; B=circle; ax=null, bx=set; use_radius=0; cache=empty | [x] |
| C142 | `c2GJK` | A=AABB; B=circle; ax=null, bx=set; use_radius=0; cache=warm | [x] |
| C143 | `c2GJK` | A=AABB; B=circle; ax=null, bx=set; use_radius=1; cache=null | [x] |
| C144 | `c2GJK` | A=AABB; B=circle; ax=null, bx=set; use_radius=1; cache=empty | [x] |
| C145 | `c2GJK` | A=AABB; B=circle; ax=null, bx=set; use_radius=1; cache=warm | [x] |
| C146 | `c2GJK` | A=AABB; B=circle; ax=set, bx=set; use_radius=0; cache=null | [x] |
| C147 | `c2GJK` | A=AABB; B=circle; ax=set, bx=set; use_radius=0; cache=empty | [x] |
| C148 | `c2GJK` | A=AABB; B=circle; ax=set, bx=set; use_radius=0; cache=warm | [x] |
| C149 | `c2GJK` | A=AABB; B=circle; ax=set, bx=set; use_radius=1; cache=null | [x] |
| C150 | `c2GJK` | A=AABB; B=circle; ax=set, bx=set; use_radius=1; cache=empty | [x] |
| C151 | `c2GJK` | A=AABB; B=circle; ax=set, bx=set; use_radius=1; cache=warm | [x] |
| C152 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=null; use_radius=0; cache=null | [x] |
| C153 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=null; use_radius=0; cache=empty | [x] |
| C154 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=null; use_radius=0; cache=warm | [x] |
| C155 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=null; use_radius=1; cache=null | [x] |
| C156 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=null; use_radius=1; cache=empty | [x] |
| C157 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=null; use_radius=1; cache=warm | [x] |
| C158 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=null; use_radius=0; cache=null | [x] |
| C159 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=null; use_radius=0; cache=empty | [x] |
| C160 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=null; use_radius=0; cache=warm | [x] |
| C161 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=null; use_radius=1; cache=null | [x] |
| C162 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=null; use_radius=1; cache=empty | [x] |
| C163 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=null; use_radius=1; cache=warm | [x] |
| C164 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=set; use_radius=0; cache=null | [x] |
| C165 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=set; use_radius=0; cache=empty | [x] |
| C166 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=set; use_radius=0; cache=warm | [x] |
| C167 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=set; use_radius=1; cache=null | [x] |
| C168 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=set; use_radius=1; cache=empty | [x] |
| C169 | `c2GJK` | A=AABB; B=AABB; ax=null, bx=set; use_radius=1; cache=warm | [x] |
| C170 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=set; use_radius=0; cache=null | [x] |
| C171 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=set; use_radius=0; cache=empty | [x] |
| C172 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=set; use_radius=0; cache=warm | [x] |
| C173 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=set; use_radius=1; cache=null | [x] |
| C174 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=set; use_radius=1; cache=empty | [x] |
| C175 | `c2GJK` | A=AABB; B=AABB; ax=set, bx=set; use_radius=1; cache=warm | [x] |
| C176 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=null; use_radius=0; cache=null | [x] |
| C177 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=null; use_radius=0; cache=empty | [x] |
| C178 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=null; use_radius=0; cache=warm | [x] |
| C179 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=null; use_radius=1; cache=null | [x] |
| C180 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=null; use_radius=1; cache=empty | [x] |
| C181 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=null; use_radius=1; cache=warm | [x] |
| C182 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=null; use_radius=0; cache=null | [x] |
| C183 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=null; use_radius=0; cache=empty | [x] |
| C184 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=null; use_radius=0; cache=warm | [x] |
| C185 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=null; use_radius=1; cache=null | [x] |
| C186 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=null; use_radius=1; cache=empty | [x] |
| C187 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=null; use_radius=1; cache=warm | [x] |
| C188 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=set; use_radius=0; cache=null | [x] |
| C189 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=set; use_radius=0; cache=empty | [x] |
| C190 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=set; use_radius=0; cache=warm | [x] |
| C191 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=set; use_radius=1; cache=null | [x] |
| C192 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=set; use_radius=1; cache=empty | [x] |
| C193 | `c2GJK` | A=AABB; B=capsule; ax=null, bx=set; use_radius=1; cache=warm | [x] |
| C194 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=set; use_radius=0; cache=null | [x] |
| C195 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=set; use_radius=0; cache=empty | [x] |
| C196 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=set; use_radius=0; cache=warm | [x] |
| C197 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=set; use_radius=1; cache=null | [x] |
| C198 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=set; use_radius=1; cache=empty | [x] |
| C199 | `c2GJK` | A=AABB; B=capsule; ax=set, bx=set; use_radius=1; cache=warm | [x] |
| C200 | `c2GJK` | A=capsule; B=circle; ax=null, bx=null; use_radius=0; cache=null | [x] |
| C201 | `c2GJK` | A=capsule; B=circle; ax=null, bx=null; use_radius=0; cache=empty | [x] |
| C202 | `c2GJK` | A=capsule; B=circle; ax=null, bx=null; use_radius=0; cache=warm | [x] |
| C203 | `c2GJK` | A=capsule; B=circle; ax=null, bx=null; use_radius=1; cache=null | [x] |
| C204 | `c2GJK` | A=capsule; B=circle; ax=null, bx=null; use_radius=1; cache=empty | [x] |
| C205 | `c2GJK` | A=capsule; B=circle; ax=null, bx=null; use_radius=1; cache=warm | [x] |
| C206 | `c2GJK` | A=capsule; B=circle; ax=set, bx=null; use_radius=0; cache=null | [x] |
| C207 | `c2GJK` | A=capsule; B=circle; ax=set, bx=null; use_radius=0; cache=empty | [x] |
| C208 | `c2GJK` | A=capsule; B=circle; ax=set, bx=null; use_radius=0; cache=warm | [x] |
| C209 | `c2GJK` | A=capsule; B=circle; ax=set, bx=null; use_radius=1; cache=null | [x] |
| C210 | `c2GJK` | A=capsule; B=circle; ax=set, bx=null; use_radius=1; cache=empty | [x] |
| C211 | `c2GJK` | A=capsule; B=circle; ax=set, bx=null; use_radius=1; cache=warm | [x] |
| C212 | `c2GJK` | A=capsule; B=circle; ax=null, bx=set; use_radius=0; cache=null | [x] |
| C213 | `c2GJK` | A=capsule; B=circle; ax=null, bx=set; use_radius=0; cache=empty | [x] |
| C214 | `c2GJK` | A=capsule; B=circle; ax=null, bx=set; use_radius=0; cache=warm | [x] |
| C215 | `c2GJK` | A=capsule; B=circle; ax=null, bx=set; use_radius=1; cache=null | [x] |
| C216 | `c2GJK` | A=capsule; B=circle; ax=null, bx=set; use_radius=1; cache=empty | [x] |
| C217 | `c2GJK` | A=capsule; B=circle; ax=null, bx=set; use_radius=1; cache=warm | [x] |
| C218 | `c2GJK` | A=capsule; B=circle; ax=set, bx=set; use_radius=0; cache=null | [x] |
| C219 | `c2GJK` | A=capsule; B=circle; ax=set, bx=set; use_radius=0; cache=empty | [x] |
| C220 | `c2GJK` | A=capsule; B=circle; ax=set, bx=set; use_radius=0; cache=warm | [x] |
| C221 | `c2GJK` | A=capsule; B=circle; ax=set, bx=set; use_radius=1; cache=null | [x] |
| C222 | `c2GJK` | A=capsule; B=circle; ax=set, bx=set; use_radius=1; cache=empty | [x] |
| C223 | `c2GJK` | A=capsule; B=circle; ax=set, bx=set; use_radius=1; cache=warm | [x] |
| C224 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=null; use_radius=0; cache=null | [x] |
| C225 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=null; use_radius=0; cache=empty | [x] |
| C226 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=null; use_radius=0; cache=warm | [x] |
| C227 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=null; use_radius=1; cache=null | [x] |
| C228 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=null; use_radius=1; cache=empty | [x] |
| C229 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=null; use_radius=1; cache=warm | [x] |
| C230 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=null; use_radius=0; cache=null | [x] |
| C231 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=null; use_radius=0; cache=empty | [x] |
| C232 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=null; use_radius=0; cache=warm | [x] |
| C233 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=null; use_radius=1; cache=null | [x] |
| C234 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=null; use_radius=1; cache=empty | [x] |
| C235 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=null; use_radius=1; cache=warm | [x] |
| C236 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=set; use_radius=0; cache=null | [x] |
| C237 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=set; use_radius=0; cache=empty | [x] |
| C238 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=set; use_radius=0; cache=warm | [x] |
| C239 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=set; use_radius=1; cache=null | [x] |
| C240 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=set; use_radius=1; cache=empty | [x] |
| C241 | `c2GJK` | A=capsule; B=AABB; ax=null, bx=set; use_radius=1; cache=warm | [x] |
| C242 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=set; use_radius=0; cache=null | [x] |
| C243 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=set; use_radius=0; cache=empty | [x] |
| C244 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=set; use_radius=0; cache=warm | [x] |
| C245 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=set; use_radius=1; cache=null | [x] |
| C246 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=set; use_radius=1; cache=empty | [x] |
| C247 | `c2GJK` | A=capsule; B=AABB; ax=set, bx=set; use_radius=1; cache=warm | [x] |
| C248 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=null; use_radius=0; cache=null | [x] |
| C249 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=null; use_radius=0; cache=empty | [x] |
| C250 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=null; use_radius=0; cache=warm | [x] |
| C251 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=null; use_radius=1; cache=null | [x] |
| C252 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=null; use_radius=1; cache=empty | [x] |
| C253 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=null; use_radius=1; cache=warm | [x] |
| C254 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=null; use_radius=0; cache=null | [x] |
| C255 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=null; use_radius=0; cache=empty | [x] |
| C256 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=null; use_radius=0; cache=warm | [x] |
| C257 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=null; use_radius=1; cache=null | [x] |
| C258 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=null; use_radius=1; cache=empty | [x] |
| C259 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=null; use_radius=1; cache=warm | [x] |
| C260 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=set; use_radius=0; cache=null | [x] |
| C261 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=set; use_radius=0; cache=empty | [x] |
| C262 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=set; use_radius=0; cache=warm | [x] |
| C263 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=set; use_radius=1; cache=null | [x] |
| C264 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=set; use_radius=1; cache=empty | [x] |
| C265 | `c2GJK` | A=capsule; B=capsule; ax=null, bx=set; use_radius=1; cache=warm | [x] |
| C266 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=set; use_radius=0; cache=null | [x] |
| C267 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=set; use_radius=0; cache=empty | [x] |
| C268 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=set; use_radius=0; cache=warm | [x] |
| C269 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=set; use_radius=1; cache=null | [x] |
| C270 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=set; use_radius=1; cache=empty | [x] |
| C271 | `c2GJK` | A=capsule; B=capsule; ax=set, bx=set; use_radius=1; cache=warm | [x] |
| C272 | `c2GJK` | all 16 null/non-null combinations of outA, outB, iterations, and cache with valid AABB/capsule inputs | [x] |
| C273 | `gjk` | reverse=0; randomized AABB and capsule values | [x] |
| C274 | `gjk` | reverse!=0; randomized AABB and capsule values | [x] |

Total configuration rows: **274**.
