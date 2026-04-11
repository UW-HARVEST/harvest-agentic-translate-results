#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn fabs(__x: libc::c_double) -> libc::c_double;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn scanf(__format: *const libc::c_char, ...) -> libc::c_int;
}
static mut stb__perlin_randtab: [libc::c_uchar; 512] = [
    23 as libc::c_int as libc::c_uchar,
    125 as libc::c_int as libc::c_uchar,
    161 as libc::c_int as libc::c_uchar,
    52 as libc::c_int as libc::c_uchar,
    103 as libc::c_int as libc::c_uchar,
    117 as libc::c_int as libc::c_uchar,
    70 as libc::c_int as libc::c_uchar,
    37 as libc::c_int as libc::c_uchar,
    247 as libc::c_int as libc::c_uchar,
    101 as libc::c_int as libc::c_uchar,
    203 as libc::c_int as libc::c_uchar,
    169 as libc::c_int as libc::c_uchar,
    124 as libc::c_int as libc::c_uchar,
    126 as libc::c_int as libc::c_uchar,
    44 as libc::c_int as libc::c_uchar,
    123 as libc::c_int as libc::c_uchar,
    152 as libc::c_int as libc::c_uchar,
    238 as libc::c_int as libc::c_uchar,
    145 as libc::c_int as libc::c_uchar,
    45 as libc::c_int as libc::c_uchar,
    171 as libc::c_int as libc::c_uchar,
    114 as libc::c_int as libc::c_uchar,
    253 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    192 as libc::c_int as libc::c_uchar,
    136 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    157 as libc::c_int as libc::c_uchar,
    249 as libc::c_int as libc::c_uchar,
    30 as libc::c_int as libc::c_uchar,
    35 as libc::c_int as libc::c_uchar,
    72 as libc::c_int as libc::c_uchar,
    175 as libc::c_int as libc::c_uchar,
    63 as libc::c_int as libc::c_uchar,
    77 as libc::c_int as libc::c_uchar,
    90 as libc::c_int as libc::c_uchar,
    181 as libc::c_int as libc::c_uchar,
    16 as libc::c_int as libc::c_uchar,
    96 as libc::c_int as libc::c_uchar,
    111 as libc::c_int as libc::c_uchar,
    133 as libc::c_int as libc::c_uchar,
    104 as libc::c_int as libc::c_uchar,
    75 as libc::c_int as libc::c_uchar,
    162 as libc::c_int as libc::c_uchar,
    93 as libc::c_int as libc::c_uchar,
    56 as libc::c_int as libc::c_uchar,
    66 as libc::c_int as libc::c_uchar,
    240 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    50 as libc::c_int as libc::c_uchar,
    84 as libc::c_int as libc::c_uchar,
    229 as libc::c_int as libc::c_uchar,
    49 as libc::c_int as libc::c_uchar,
    210 as libc::c_int as libc::c_uchar,
    173 as libc::c_int as libc::c_uchar,
    239 as libc::c_int as libc::c_uchar,
    141 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    87 as libc::c_int as libc::c_uchar,
    18 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    198 as libc::c_int as libc::c_uchar,
    143 as libc::c_int as libc::c_uchar,
    57 as libc::c_int as libc::c_uchar,
    225 as libc::c_int as libc::c_uchar,
    160 as libc::c_int as libc::c_uchar,
    58 as libc::c_int as libc::c_uchar,
    217 as libc::c_int as libc::c_uchar,
    168 as libc::c_int as libc::c_uchar,
    206 as libc::c_int as libc::c_uchar,
    245 as libc::c_int as libc::c_uchar,
    204 as libc::c_int as libc::c_uchar,
    199 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    73 as libc::c_int as libc::c_uchar,
    60 as libc::c_int as libc::c_uchar,
    20 as libc::c_int as libc::c_uchar,
    230 as libc::c_int as libc::c_uchar,
    211 as libc::c_int as libc::c_uchar,
    233 as libc::c_int as libc::c_uchar,
    94 as libc::c_int as libc::c_uchar,
    200 as libc::c_int as libc::c_uchar,
    88 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    74 as libc::c_int as libc::c_uchar,
    155 as libc::c_int as libc::c_uchar,
    33 as libc::c_int as libc::c_uchar,
    15 as libc::c_int as libc::c_uchar,
    219 as libc::c_int as libc::c_uchar,
    130 as libc::c_int as libc::c_uchar,
    226 as libc::c_int as libc::c_uchar,
    202 as libc::c_int as libc::c_uchar,
    83 as libc::c_int as libc::c_uchar,
    236 as libc::c_int as libc::c_uchar,
    42 as libc::c_int as libc::c_uchar,
    172 as libc::c_int as libc::c_uchar,
    165 as libc::c_int as libc::c_uchar,
    218 as libc::c_int as libc::c_uchar,
    55 as libc::c_int as libc::c_uchar,
    222 as libc::c_int as libc::c_uchar,
    46 as libc::c_int as libc::c_uchar,
    107 as libc::c_int as libc::c_uchar,
    98 as libc::c_int as libc::c_uchar,
    154 as libc::c_int as libc::c_uchar,
    109 as libc::c_int as libc::c_uchar,
    67 as libc::c_int as libc::c_uchar,
    196 as libc::c_int as libc::c_uchar,
    178 as libc::c_int as libc::c_uchar,
    127 as libc::c_int as libc::c_uchar,
    158 as libc::c_int as libc::c_uchar,
    13 as libc::c_int as libc::c_uchar,
    243 as libc::c_int as libc::c_uchar,
    65 as libc::c_int as libc::c_uchar,
    79 as libc::c_int as libc::c_uchar,
    166 as libc::c_int as libc::c_uchar,
    248 as libc::c_int as libc::c_uchar,
    25 as libc::c_int as libc::c_uchar,
    224 as libc::c_int as libc::c_uchar,
    115 as libc::c_int as libc::c_uchar,
    80 as libc::c_int as libc::c_uchar,
    68 as libc::c_int as libc::c_uchar,
    51 as libc::c_int as libc::c_uchar,
    184 as libc::c_int as libc::c_uchar,
    128 as libc::c_int as libc::c_uchar,
    232 as libc::c_int as libc::c_uchar,
    208 as libc::c_int as libc::c_uchar,
    151 as libc::c_int as libc::c_uchar,
    122 as libc::c_int as libc::c_uchar,
    26 as libc::c_int as libc::c_uchar,
    212 as libc::c_int as libc::c_uchar,
    105 as libc::c_int as libc::c_uchar,
    43 as libc::c_int as libc::c_uchar,
    179 as libc::c_int as libc::c_uchar,
    213 as libc::c_int as libc::c_uchar,
    235 as libc::c_int as libc::c_uchar,
    148 as libc::c_int as libc::c_uchar,
    146 as libc::c_int as libc::c_uchar,
    89 as libc::c_int as libc::c_uchar,
    14 as libc::c_int as libc::c_uchar,
    195 as libc::c_int as libc::c_uchar,
    28 as libc::c_int as libc::c_uchar,
    78 as libc::c_int as libc::c_uchar,
    112 as libc::c_int as libc::c_uchar,
    76 as libc::c_int as libc::c_uchar,
    250 as libc::c_int as libc::c_uchar,
    47 as libc::c_int as libc::c_uchar,
    24 as libc::c_int as libc::c_uchar,
    251 as libc::c_int as libc::c_uchar,
    140 as libc::c_int as libc::c_uchar,
    108 as libc::c_int as libc::c_uchar,
    186 as libc::c_int as libc::c_uchar,
    190 as libc::c_int as libc::c_uchar,
    228 as libc::c_int as libc::c_uchar,
    170 as libc::c_int as libc::c_uchar,
    183 as libc::c_int as libc::c_uchar,
    139 as libc::c_int as libc::c_uchar,
    39 as libc::c_int as libc::c_uchar,
    188 as libc::c_int as libc::c_uchar,
    244 as libc::c_int as libc::c_uchar,
    246 as libc::c_int as libc::c_uchar,
    132 as libc::c_int as libc::c_uchar,
    48 as libc::c_int as libc::c_uchar,
    119 as libc::c_int as libc::c_uchar,
    144 as libc::c_int as libc::c_uchar,
    180 as libc::c_int as libc::c_uchar,
    138 as libc::c_int as libc::c_uchar,
    134 as libc::c_int as libc::c_uchar,
    193 as libc::c_int as libc::c_uchar,
    82 as libc::c_int as libc::c_uchar,
    182 as libc::c_int as libc::c_uchar,
    120 as libc::c_int as libc::c_uchar,
    121 as libc::c_int as libc::c_uchar,
    86 as libc::c_int as libc::c_uchar,
    220 as libc::c_int as libc::c_uchar,
    209 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    91 as libc::c_int as libc::c_uchar,
    241 as libc::c_int as libc::c_uchar,
    149 as libc::c_int as libc::c_uchar,
    85 as libc::c_int as libc::c_uchar,
    205 as libc::c_int as libc::c_uchar,
    150 as libc::c_int as libc::c_uchar,
    113 as libc::c_int as libc::c_uchar,
    216 as libc::c_int as libc::c_uchar,
    31 as libc::c_int as libc::c_uchar,
    100 as libc::c_int as libc::c_uchar,
    41 as libc::c_int as libc::c_uchar,
    164 as libc::c_int as libc::c_uchar,
    177 as libc::c_int as libc::c_uchar,
    214 as libc::c_int as libc::c_uchar,
    153 as libc::c_int as libc::c_uchar,
    231 as libc::c_int as libc::c_uchar,
    38 as libc::c_int as libc::c_uchar,
    71 as libc::c_int as libc::c_uchar,
    185 as libc::c_int as libc::c_uchar,
    174 as libc::c_int as libc::c_uchar,
    97 as libc::c_int as libc::c_uchar,
    201 as libc::c_int as libc::c_uchar,
    29 as libc::c_int as libc::c_uchar,
    95 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    92 as libc::c_int as libc::c_uchar,
    54 as libc::c_int as libc::c_uchar,
    254 as libc::c_int as libc::c_uchar,
    191 as libc::c_int as libc::c_uchar,
    118 as libc::c_int as libc::c_uchar,
    34 as libc::c_int as libc::c_uchar,
    221 as libc::c_int as libc::c_uchar,
    131 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    163 as libc::c_int as libc::c_uchar,
    99 as libc::c_int as libc::c_uchar,
    234 as libc::c_int as libc::c_uchar,
    81 as libc::c_int as libc::c_uchar,
    227 as libc::c_int as libc::c_uchar,
    147 as libc::c_int as libc::c_uchar,
    156 as libc::c_int as libc::c_uchar,
    176 as libc::c_int as libc::c_uchar,
    17 as libc::c_int as libc::c_uchar,
    142 as libc::c_int as libc::c_uchar,
    69 as libc::c_int as libc::c_uchar,
    12 as libc::c_int as libc::c_uchar,
    110 as libc::c_int as libc::c_uchar,
    62 as libc::c_int as libc::c_uchar,
    27 as libc::c_int as libc::c_uchar,
    255 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    194 as libc::c_int as libc::c_uchar,
    59 as libc::c_int as libc::c_uchar,
    116 as libc::c_int as libc::c_uchar,
    242 as libc::c_int as libc::c_uchar,
    252 as libc::c_int as libc::c_uchar,
    19 as libc::c_int as libc::c_uchar,
    21 as libc::c_int as libc::c_uchar,
    187 as libc::c_int as libc::c_uchar,
    53 as libc::c_int as libc::c_uchar,
    207 as libc::c_int as libc::c_uchar,
    129 as libc::c_int as libc::c_uchar,
    64 as libc::c_int as libc::c_uchar,
    135 as libc::c_int as libc::c_uchar,
    61 as libc::c_int as libc::c_uchar,
    40 as libc::c_int as libc::c_uchar,
    167 as libc::c_int as libc::c_uchar,
    237 as libc::c_int as libc::c_uchar,
    102 as libc::c_int as libc::c_uchar,
    223 as libc::c_int as libc::c_uchar,
    106 as libc::c_int as libc::c_uchar,
    159 as libc::c_int as libc::c_uchar,
    197 as libc::c_int as libc::c_uchar,
    189 as libc::c_int as libc::c_uchar,
    215 as libc::c_int as libc::c_uchar,
    137 as libc::c_int as libc::c_uchar,
    36 as libc::c_int as libc::c_uchar,
    32 as libc::c_int as libc::c_uchar,
    22 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    23 as libc::c_int as libc::c_uchar,
    125 as libc::c_int as libc::c_uchar,
    161 as libc::c_int as libc::c_uchar,
    52 as libc::c_int as libc::c_uchar,
    103 as libc::c_int as libc::c_uchar,
    117 as libc::c_int as libc::c_uchar,
    70 as libc::c_int as libc::c_uchar,
    37 as libc::c_int as libc::c_uchar,
    247 as libc::c_int as libc::c_uchar,
    101 as libc::c_int as libc::c_uchar,
    203 as libc::c_int as libc::c_uchar,
    169 as libc::c_int as libc::c_uchar,
    124 as libc::c_int as libc::c_uchar,
    126 as libc::c_int as libc::c_uchar,
    44 as libc::c_int as libc::c_uchar,
    123 as libc::c_int as libc::c_uchar,
    152 as libc::c_int as libc::c_uchar,
    238 as libc::c_int as libc::c_uchar,
    145 as libc::c_int as libc::c_uchar,
    45 as libc::c_int as libc::c_uchar,
    171 as libc::c_int as libc::c_uchar,
    114 as libc::c_int as libc::c_uchar,
    253 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    192 as libc::c_int as libc::c_uchar,
    136 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    157 as libc::c_int as libc::c_uchar,
    249 as libc::c_int as libc::c_uchar,
    30 as libc::c_int as libc::c_uchar,
    35 as libc::c_int as libc::c_uchar,
    72 as libc::c_int as libc::c_uchar,
    175 as libc::c_int as libc::c_uchar,
    63 as libc::c_int as libc::c_uchar,
    77 as libc::c_int as libc::c_uchar,
    90 as libc::c_int as libc::c_uchar,
    181 as libc::c_int as libc::c_uchar,
    16 as libc::c_int as libc::c_uchar,
    96 as libc::c_int as libc::c_uchar,
    111 as libc::c_int as libc::c_uchar,
    133 as libc::c_int as libc::c_uchar,
    104 as libc::c_int as libc::c_uchar,
    75 as libc::c_int as libc::c_uchar,
    162 as libc::c_int as libc::c_uchar,
    93 as libc::c_int as libc::c_uchar,
    56 as libc::c_int as libc::c_uchar,
    66 as libc::c_int as libc::c_uchar,
    240 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    50 as libc::c_int as libc::c_uchar,
    84 as libc::c_int as libc::c_uchar,
    229 as libc::c_int as libc::c_uchar,
    49 as libc::c_int as libc::c_uchar,
    210 as libc::c_int as libc::c_uchar,
    173 as libc::c_int as libc::c_uchar,
    239 as libc::c_int as libc::c_uchar,
    141 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    87 as libc::c_int as libc::c_uchar,
    18 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    198 as libc::c_int as libc::c_uchar,
    143 as libc::c_int as libc::c_uchar,
    57 as libc::c_int as libc::c_uchar,
    225 as libc::c_int as libc::c_uchar,
    160 as libc::c_int as libc::c_uchar,
    58 as libc::c_int as libc::c_uchar,
    217 as libc::c_int as libc::c_uchar,
    168 as libc::c_int as libc::c_uchar,
    206 as libc::c_int as libc::c_uchar,
    245 as libc::c_int as libc::c_uchar,
    204 as libc::c_int as libc::c_uchar,
    199 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    73 as libc::c_int as libc::c_uchar,
    60 as libc::c_int as libc::c_uchar,
    20 as libc::c_int as libc::c_uchar,
    230 as libc::c_int as libc::c_uchar,
    211 as libc::c_int as libc::c_uchar,
    233 as libc::c_int as libc::c_uchar,
    94 as libc::c_int as libc::c_uchar,
    200 as libc::c_int as libc::c_uchar,
    88 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    74 as libc::c_int as libc::c_uchar,
    155 as libc::c_int as libc::c_uchar,
    33 as libc::c_int as libc::c_uchar,
    15 as libc::c_int as libc::c_uchar,
    219 as libc::c_int as libc::c_uchar,
    130 as libc::c_int as libc::c_uchar,
    226 as libc::c_int as libc::c_uchar,
    202 as libc::c_int as libc::c_uchar,
    83 as libc::c_int as libc::c_uchar,
    236 as libc::c_int as libc::c_uchar,
    42 as libc::c_int as libc::c_uchar,
    172 as libc::c_int as libc::c_uchar,
    165 as libc::c_int as libc::c_uchar,
    218 as libc::c_int as libc::c_uchar,
    55 as libc::c_int as libc::c_uchar,
    222 as libc::c_int as libc::c_uchar,
    46 as libc::c_int as libc::c_uchar,
    107 as libc::c_int as libc::c_uchar,
    98 as libc::c_int as libc::c_uchar,
    154 as libc::c_int as libc::c_uchar,
    109 as libc::c_int as libc::c_uchar,
    67 as libc::c_int as libc::c_uchar,
    196 as libc::c_int as libc::c_uchar,
    178 as libc::c_int as libc::c_uchar,
    127 as libc::c_int as libc::c_uchar,
    158 as libc::c_int as libc::c_uchar,
    13 as libc::c_int as libc::c_uchar,
    243 as libc::c_int as libc::c_uchar,
    65 as libc::c_int as libc::c_uchar,
    79 as libc::c_int as libc::c_uchar,
    166 as libc::c_int as libc::c_uchar,
    248 as libc::c_int as libc::c_uchar,
    25 as libc::c_int as libc::c_uchar,
    224 as libc::c_int as libc::c_uchar,
    115 as libc::c_int as libc::c_uchar,
    80 as libc::c_int as libc::c_uchar,
    68 as libc::c_int as libc::c_uchar,
    51 as libc::c_int as libc::c_uchar,
    184 as libc::c_int as libc::c_uchar,
    128 as libc::c_int as libc::c_uchar,
    232 as libc::c_int as libc::c_uchar,
    208 as libc::c_int as libc::c_uchar,
    151 as libc::c_int as libc::c_uchar,
    122 as libc::c_int as libc::c_uchar,
    26 as libc::c_int as libc::c_uchar,
    212 as libc::c_int as libc::c_uchar,
    105 as libc::c_int as libc::c_uchar,
    43 as libc::c_int as libc::c_uchar,
    179 as libc::c_int as libc::c_uchar,
    213 as libc::c_int as libc::c_uchar,
    235 as libc::c_int as libc::c_uchar,
    148 as libc::c_int as libc::c_uchar,
    146 as libc::c_int as libc::c_uchar,
    89 as libc::c_int as libc::c_uchar,
    14 as libc::c_int as libc::c_uchar,
    195 as libc::c_int as libc::c_uchar,
    28 as libc::c_int as libc::c_uchar,
    78 as libc::c_int as libc::c_uchar,
    112 as libc::c_int as libc::c_uchar,
    76 as libc::c_int as libc::c_uchar,
    250 as libc::c_int as libc::c_uchar,
    47 as libc::c_int as libc::c_uchar,
    24 as libc::c_int as libc::c_uchar,
    251 as libc::c_int as libc::c_uchar,
    140 as libc::c_int as libc::c_uchar,
    108 as libc::c_int as libc::c_uchar,
    186 as libc::c_int as libc::c_uchar,
    190 as libc::c_int as libc::c_uchar,
    228 as libc::c_int as libc::c_uchar,
    170 as libc::c_int as libc::c_uchar,
    183 as libc::c_int as libc::c_uchar,
    139 as libc::c_int as libc::c_uchar,
    39 as libc::c_int as libc::c_uchar,
    188 as libc::c_int as libc::c_uchar,
    244 as libc::c_int as libc::c_uchar,
    246 as libc::c_int as libc::c_uchar,
    132 as libc::c_int as libc::c_uchar,
    48 as libc::c_int as libc::c_uchar,
    119 as libc::c_int as libc::c_uchar,
    144 as libc::c_int as libc::c_uchar,
    180 as libc::c_int as libc::c_uchar,
    138 as libc::c_int as libc::c_uchar,
    134 as libc::c_int as libc::c_uchar,
    193 as libc::c_int as libc::c_uchar,
    82 as libc::c_int as libc::c_uchar,
    182 as libc::c_int as libc::c_uchar,
    120 as libc::c_int as libc::c_uchar,
    121 as libc::c_int as libc::c_uchar,
    86 as libc::c_int as libc::c_uchar,
    220 as libc::c_int as libc::c_uchar,
    209 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    91 as libc::c_int as libc::c_uchar,
    241 as libc::c_int as libc::c_uchar,
    149 as libc::c_int as libc::c_uchar,
    85 as libc::c_int as libc::c_uchar,
    205 as libc::c_int as libc::c_uchar,
    150 as libc::c_int as libc::c_uchar,
    113 as libc::c_int as libc::c_uchar,
    216 as libc::c_int as libc::c_uchar,
    31 as libc::c_int as libc::c_uchar,
    100 as libc::c_int as libc::c_uchar,
    41 as libc::c_int as libc::c_uchar,
    164 as libc::c_int as libc::c_uchar,
    177 as libc::c_int as libc::c_uchar,
    214 as libc::c_int as libc::c_uchar,
    153 as libc::c_int as libc::c_uchar,
    231 as libc::c_int as libc::c_uchar,
    38 as libc::c_int as libc::c_uchar,
    71 as libc::c_int as libc::c_uchar,
    185 as libc::c_int as libc::c_uchar,
    174 as libc::c_int as libc::c_uchar,
    97 as libc::c_int as libc::c_uchar,
    201 as libc::c_int as libc::c_uchar,
    29 as libc::c_int as libc::c_uchar,
    95 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    92 as libc::c_int as libc::c_uchar,
    54 as libc::c_int as libc::c_uchar,
    254 as libc::c_int as libc::c_uchar,
    191 as libc::c_int as libc::c_uchar,
    118 as libc::c_int as libc::c_uchar,
    34 as libc::c_int as libc::c_uchar,
    221 as libc::c_int as libc::c_uchar,
    131 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    163 as libc::c_int as libc::c_uchar,
    99 as libc::c_int as libc::c_uchar,
    234 as libc::c_int as libc::c_uchar,
    81 as libc::c_int as libc::c_uchar,
    227 as libc::c_int as libc::c_uchar,
    147 as libc::c_int as libc::c_uchar,
    156 as libc::c_int as libc::c_uchar,
    176 as libc::c_int as libc::c_uchar,
    17 as libc::c_int as libc::c_uchar,
    142 as libc::c_int as libc::c_uchar,
    69 as libc::c_int as libc::c_uchar,
    12 as libc::c_int as libc::c_uchar,
    110 as libc::c_int as libc::c_uchar,
    62 as libc::c_int as libc::c_uchar,
    27 as libc::c_int as libc::c_uchar,
    255 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    194 as libc::c_int as libc::c_uchar,
    59 as libc::c_int as libc::c_uchar,
    116 as libc::c_int as libc::c_uchar,
    242 as libc::c_int as libc::c_uchar,
    252 as libc::c_int as libc::c_uchar,
    19 as libc::c_int as libc::c_uchar,
    21 as libc::c_int as libc::c_uchar,
    187 as libc::c_int as libc::c_uchar,
    53 as libc::c_int as libc::c_uchar,
    207 as libc::c_int as libc::c_uchar,
    129 as libc::c_int as libc::c_uchar,
    64 as libc::c_int as libc::c_uchar,
    135 as libc::c_int as libc::c_uchar,
    61 as libc::c_int as libc::c_uchar,
    40 as libc::c_int as libc::c_uchar,
    167 as libc::c_int as libc::c_uchar,
    237 as libc::c_int as libc::c_uchar,
    102 as libc::c_int as libc::c_uchar,
    223 as libc::c_int as libc::c_uchar,
    106 as libc::c_int as libc::c_uchar,
    159 as libc::c_int as libc::c_uchar,
    197 as libc::c_int as libc::c_uchar,
    189 as libc::c_int as libc::c_uchar,
    215 as libc::c_int as libc::c_uchar,
    137 as libc::c_int as libc::c_uchar,
    36 as libc::c_int as libc::c_uchar,
    32 as libc::c_int as libc::c_uchar,
    22 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
];
static mut stb__perlin_randtab_grad_idx: [libc::c_uchar; 512] = [
    7 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    1 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    0 as libc::c_int as libc::c_uchar,
    11 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    10 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    2 as libc::c_int as libc::c_uchar,
    3 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    7 as libc::c_int as libc::c_uchar,
    9 as libc::c_int as libc::c_uchar,
    8 as libc::c_int as libc::c_uchar,
    4 as libc::c_int as libc::c_uchar,
    6 as libc::c_int as libc::c_uchar,
    5 as libc::c_int as libc::c_uchar,
];
unsafe extern "C" fn stb__perlin_lerp(
    mut a: libc::c_float,
    mut b: libc::c_float,
    mut t: libc::c_float,
) -> libc::c_float {
    return a + (b - a) * t;
}
unsafe extern "C" fn stb__perlin_fastfloor(mut a: libc::c_float) -> libc::c_int {
    let mut ai: libc::c_int = a as libc::c_int;
    return if a < ai as libc::c_float {
        ai - 1 as libc::c_int
    } else {
        ai
    };
}
unsafe extern "C" fn stb__perlin_grad(
    mut grad_idx: libc::c_int,
    mut x: libc::c_float,
    mut y: libc::c_float,
    mut z: libc::c_float,
) -> libc::c_float {
    static mut basis: [[libc::c_float; 4]; 12] = [
        [
            1 as libc::c_int as libc::c_float,
            1 as libc::c_int as libc::c_float,
            0 as libc::c_int as libc::c_float,
            0.,
        ],
        [
            -(1 as libc::c_int) as libc::c_float,
            1 as libc::c_int as libc::c_float,
            0 as libc::c_int as libc::c_float,
            0.,
        ],
        [
            1 as libc::c_int as libc::c_float,
            -(1 as libc::c_int) as libc::c_float,
            0 as libc::c_int as libc::c_float,
            0.,
        ],
        [
            -(1 as libc::c_int) as libc::c_float,
            -(1 as libc::c_int) as libc::c_float,
            0 as libc::c_int as libc::c_float,
            0.,
        ],
        [
            1 as libc::c_int as libc::c_float,
            0 as libc::c_int as libc::c_float,
            1 as libc::c_int as libc::c_float,
            0.,
        ],
        [
            -(1 as libc::c_int) as libc::c_float,
            0 as libc::c_int as libc::c_float,
            1 as libc::c_int as libc::c_float,
            0.,
        ],
        [
            1 as libc::c_int as libc::c_float,
            0 as libc::c_int as libc::c_float,
            -(1 as libc::c_int) as libc::c_float,
            0.,
        ],
        [
            -(1 as libc::c_int) as libc::c_float,
            0 as libc::c_int as libc::c_float,
            -(1 as libc::c_int) as libc::c_float,
            0.,
        ],
        [
            0 as libc::c_int as libc::c_float,
            1 as libc::c_int as libc::c_float,
            1 as libc::c_int as libc::c_float,
            0.,
        ],
        [
            0 as libc::c_int as libc::c_float,
            -(1 as libc::c_int) as libc::c_float,
            1 as libc::c_int as libc::c_float,
            0.,
        ],
        [
            0 as libc::c_int as libc::c_float,
            1 as libc::c_int as libc::c_float,
            -(1 as libc::c_int) as libc::c_float,
            0.,
        ],
        [
            0 as libc::c_int as libc::c_float,
            -(1 as libc::c_int) as libc::c_float,
            -(1 as libc::c_int) as libc::c_float,
            0.,
        ],
    ];
    let mut grad: *mut libc::c_float =
        &raw mut *(&raw mut basis as *mut [libc::c_float; 4]).offset(grad_idx as isize)
            as *mut libc::c_float;
    return *grad.offset(0 as libc::c_int as isize) * x
        + *grad.offset(1 as libc::c_int as isize) * y
        + *grad.offset(2 as libc::c_int as isize) * z;
}
#[no_mangle]
pub unsafe extern "C" fn stb_perlin_noise3_internal(
    mut x: libc::c_float,
    mut y: libc::c_float,
    mut z: libc::c_float,
    mut x_wrap: libc::c_int,
    mut y_wrap: libc::c_int,
    mut z_wrap: libc::c_int,
    mut seed: libc::c_uchar,
) -> libc::c_float {
    let mut u: libc::c_float = 0.;
    let mut v: libc::c_float = 0.;
    let mut w: libc::c_float = 0.;
    let mut n000: libc::c_float = 0.;
    let mut n001: libc::c_float = 0.;
    let mut n010: libc::c_float = 0.;
    let mut n011: libc::c_float = 0.;
    let mut n100: libc::c_float = 0.;
    let mut n101: libc::c_float = 0.;
    let mut n110: libc::c_float = 0.;
    let mut n111: libc::c_float = 0.;
    let mut n00: libc::c_float = 0.;
    let mut n01: libc::c_float = 0.;
    let mut n10: libc::c_float = 0.;
    let mut n11: libc::c_float = 0.;
    let mut n0: libc::c_float = 0.;
    let mut n1: libc::c_float = 0.;
    let mut x_mask: libc::c_uint =
        (x_wrap - 1 as libc::c_int & 255 as libc::c_int) as libc::c_uint;
    let mut y_mask: libc::c_uint =
        (y_wrap - 1 as libc::c_int & 255 as libc::c_int) as libc::c_uint;
    let mut z_mask: libc::c_uint =
        (z_wrap - 1 as libc::c_int & 255 as libc::c_int) as libc::c_uint;
    let mut px: libc::c_int = stb__perlin_fastfloor(x);
    let mut py: libc::c_int = stb__perlin_fastfloor(y);
    let mut pz: libc::c_int = stb__perlin_fastfloor(z);
    let mut x0: libc::c_int = (px as libc::c_uint & x_mask) as libc::c_int;
    let mut x1: libc::c_int =
        ((px + 1 as libc::c_int) as libc::c_uint & x_mask) as libc::c_int;
    let mut y0: libc::c_int = (py as libc::c_uint & y_mask) as libc::c_int;
    let mut y1: libc::c_int =
        ((py + 1 as libc::c_int) as libc::c_uint & y_mask) as libc::c_int;
    let mut z0: libc::c_int = (pz as libc::c_uint & z_mask) as libc::c_int;
    let mut z1: libc::c_int =
        ((pz + 1 as libc::c_int) as libc::c_uint & z_mask) as libc::c_int;
    let mut r0: libc::c_int = 0;
    let mut r1: libc::c_int = 0;
    let mut r00: libc::c_int = 0;
    let mut r01: libc::c_int = 0;
    let mut r10: libc::c_int = 0;
    let mut r11: libc::c_int = 0;
    x -= px as libc::c_float;
    u = ((x * 6 as libc::c_int as libc::c_float
        - 15 as libc::c_int as libc::c_float)
        * x
        + 10 as libc::c_int as libc::c_float)
        * x
        * x
        * x;
    y -= py as libc::c_float;
    v = ((y * 6 as libc::c_int as libc::c_float
        - 15 as libc::c_int as libc::c_float)
        * y
        + 10 as libc::c_int as libc::c_float)
        * y
        * y
        * y;
    z -= pz as libc::c_float;
    w = ((z * 6 as libc::c_int as libc::c_float
        - 15 as libc::c_int as libc::c_float)
        * z
        + 10 as libc::c_int as libc::c_float)
        * z
        * z
        * z;
    r0 = stb__perlin_randtab[(x0 + seed as libc::c_int) as usize] as libc::c_int;
    r1 = stb__perlin_randtab[(x1 + seed as libc::c_int) as usize] as libc::c_int;
    r00 = stb__perlin_randtab[(r0 + y0) as usize] as libc::c_int;
    r01 = stb__perlin_randtab[(r0 + y1) as usize] as libc::c_int;
    r10 = stb__perlin_randtab[(r1 + y0) as usize] as libc::c_int;
    r11 = stb__perlin_randtab[(r1 + y1) as usize] as libc::c_int;
    n000 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r00 + z0) as usize] as libc::c_int,
        x,
        y,
        z,
    );
    n001 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r00 + z1) as usize] as libc::c_int,
        x,
        y,
        z - 1 as libc::c_int as libc::c_float,
    );
    n010 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r01 + z0) as usize] as libc::c_int,
        x,
        y - 1 as libc::c_int as libc::c_float,
        z,
    );
    n011 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r01 + z1) as usize] as libc::c_int,
        x,
        y - 1 as libc::c_int as libc::c_float,
        z - 1 as libc::c_int as libc::c_float,
    );
    n100 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r10 + z0) as usize] as libc::c_int,
        x - 1 as libc::c_int as libc::c_float,
        y,
        z,
    );
    n101 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r10 + z1) as usize] as libc::c_int,
        x - 1 as libc::c_int as libc::c_float,
        y,
        z - 1 as libc::c_int as libc::c_float,
    );
    n110 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r11 + z0) as usize] as libc::c_int,
        x - 1 as libc::c_int as libc::c_float,
        y - 1 as libc::c_int as libc::c_float,
        z,
    );
    n111 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r11 + z1) as usize] as libc::c_int,
        x - 1 as libc::c_int as libc::c_float,
        y - 1 as libc::c_int as libc::c_float,
        z - 1 as libc::c_int as libc::c_float,
    );
    n00 = stb__perlin_lerp(n000, n001, w);
    n01 = stb__perlin_lerp(n010, n011, w);
    n10 = stb__perlin_lerp(n100, n101, w);
    n11 = stb__perlin_lerp(n110, n111, w);
    n0 = stb__perlin_lerp(n00, n01, v);
    n1 = stb__perlin_lerp(n10, n11, v);
    return stb__perlin_lerp(n0, n1, u);
}
#[no_mangle]
pub unsafe extern "C" fn stb_perlin_noise3(
    mut x: libc::c_float,
    mut y: libc::c_float,
    mut z: libc::c_float,
    mut x_wrap: libc::c_int,
    mut y_wrap: libc::c_int,
    mut z_wrap: libc::c_int,
) -> libc::c_float {
    return stb_perlin_noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, 0 as libc::c_uchar);
}
#[no_mangle]
pub unsafe extern "C" fn stb_perlin_noise3_seed(
    mut x: libc::c_float,
    mut y: libc::c_float,
    mut z: libc::c_float,
    mut x_wrap: libc::c_int,
    mut y_wrap: libc::c_int,
    mut z_wrap: libc::c_int,
    mut seed: libc::c_int,
) -> libc::c_float {
    return stb_perlin_noise3_internal(
        x,
        y,
        z,
        x_wrap,
        y_wrap,
        z_wrap,
        seed as libc::c_uchar,
    );
}
#[no_mangle]
pub unsafe extern "C" fn stb_perlin_ridge_noise3(
    mut x: libc::c_float,
    mut y: libc::c_float,
    mut z: libc::c_float,
    mut lacunarity: libc::c_float,
    mut gain: libc::c_float,
    mut offset: libc::c_float,
    mut octaves: libc::c_int,
) -> libc::c_float {
    let mut i: libc::c_int = 0;
    let mut frequency: libc::c_float = 1.0f32;
    let mut prev: libc::c_float = 1.0f32;
    let mut amplitude: libc::c_float = 0.5f32;
    let mut sum: libc::c_float = 0.0f32;
    i = 0 as libc::c_int;
    while i < octaves {
        let mut r: libc::c_float = stb_perlin_noise3_internal(
            x * frequency,
            y * frequency,
            z * frequency,
            0 as libc::c_int,
            0 as libc::c_int,
            0 as libc::c_int,
            i as libc::c_uchar,
        );
        r = offset - fabs(r as libc::c_double) as libc::c_float;
        r = r * r;
        sum += r * amplitude * prev;
        prev = r;
        frequency *= lacunarity;
        amplitude *= gain;
        i += 1;
    }
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn stb_perlin_fbm_noise3(
    mut x: libc::c_float,
    mut y: libc::c_float,
    mut z: libc::c_float,
    mut lacunarity: libc::c_float,
    mut gain: libc::c_float,
    mut octaves: libc::c_int,
) -> libc::c_float {
    let mut i: libc::c_int = 0;
    let mut frequency: libc::c_float = 1.0f32;
    let mut amplitude: libc::c_float = 1.0f32;
    let mut sum: libc::c_float = 0.0f32;
    i = 0 as libc::c_int;
    while i < octaves {
        sum += stb_perlin_noise3_internal(
            x * frequency,
            y * frequency,
            z * frequency,
            0 as libc::c_int,
            0 as libc::c_int,
            0 as libc::c_int,
            i as libc::c_uchar,
        ) * amplitude;
        frequency *= lacunarity;
        amplitude *= gain;
        i += 1;
    }
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn stb_perlin_turbulence_noise3(
    mut x: libc::c_float,
    mut y: libc::c_float,
    mut z: libc::c_float,
    mut lacunarity: libc::c_float,
    mut gain: libc::c_float,
    mut octaves: libc::c_int,
) -> libc::c_float {
    let mut i: libc::c_int = 0;
    let mut frequency: libc::c_float = 1.0f32;
    let mut amplitude: libc::c_float = 1.0f32;
    let mut sum: libc::c_float = 0.0f32;
    i = 0 as libc::c_int;
    while i < octaves {
        let mut r: libc::c_float = stb_perlin_noise3_internal(
            x * frequency,
            y * frequency,
            z * frequency,
            0 as libc::c_int,
            0 as libc::c_int,
            0 as libc::c_int,
            i as libc::c_uchar,
        ) * amplitude;
        sum += fabs(r as libc::c_double) as libc::c_float;
        frequency *= lacunarity;
        amplitude *= gain;
        i += 1;
    }
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn stb_perlin_noise3_wrap_nonpow2(
    mut x: libc::c_float,
    mut y: libc::c_float,
    mut z: libc::c_float,
    mut x_wrap: libc::c_int,
    mut y_wrap: libc::c_int,
    mut z_wrap: libc::c_int,
    mut seed: libc::c_uchar,
) -> libc::c_float {
    let mut u: libc::c_float = 0.;
    let mut v: libc::c_float = 0.;
    let mut w: libc::c_float = 0.;
    let mut n000: libc::c_float = 0.;
    let mut n001: libc::c_float = 0.;
    let mut n010: libc::c_float = 0.;
    let mut n011: libc::c_float = 0.;
    let mut n100: libc::c_float = 0.;
    let mut n101: libc::c_float = 0.;
    let mut n110: libc::c_float = 0.;
    let mut n111: libc::c_float = 0.;
    let mut n00: libc::c_float = 0.;
    let mut n01: libc::c_float = 0.;
    let mut n10: libc::c_float = 0.;
    let mut n11: libc::c_float = 0.;
    let mut n0: libc::c_float = 0.;
    let mut n1: libc::c_float = 0.;
    let mut px: libc::c_int = stb__perlin_fastfloor(x);
    let mut py: libc::c_int = stb__perlin_fastfloor(y);
    let mut pz: libc::c_int = stb__perlin_fastfloor(z);
    let mut x_wrap2: libc::c_int = if x_wrap != 0 {
        x_wrap
    } else {
        256 as libc::c_int
    };
    let mut y_wrap2: libc::c_int = if y_wrap != 0 {
        y_wrap
    } else {
        256 as libc::c_int
    };
    let mut z_wrap2: libc::c_int = if z_wrap != 0 {
        z_wrap
    } else {
        256 as libc::c_int
    };
    let mut x0: libc::c_int = px % x_wrap2;
    let mut x1: libc::c_int = 0;
    let mut y0: libc::c_int = py % y_wrap2;
    let mut y1: libc::c_int = 0;
    let mut z0: libc::c_int = pz % z_wrap2;
    let mut z1: libc::c_int = 0;
    let mut r0: libc::c_int = 0;
    let mut r1: libc::c_int = 0;
    let mut r00: libc::c_int = 0;
    let mut r01: libc::c_int = 0;
    let mut r10: libc::c_int = 0;
    let mut r11: libc::c_int = 0;
    if x0 < 0 as libc::c_int {
        x0 += x_wrap2;
    }
    if y0 < 0 as libc::c_int {
        y0 += y_wrap2;
    }
    if z0 < 0 as libc::c_int {
        z0 += z_wrap2;
    }
    x1 = (x0 + 1 as libc::c_int) % x_wrap2;
    y1 = (y0 + 1 as libc::c_int) % y_wrap2;
    z1 = (z0 + 1 as libc::c_int) % z_wrap2;
    x -= px as libc::c_float;
    u = ((x * 6 as libc::c_int as libc::c_float
        - 15 as libc::c_int as libc::c_float)
        * x
        + 10 as libc::c_int as libc::c_float)
        * x
        * x
        * x;
    y -= py as libc::c_float;
    v = ((y * 6 as libc::c_int as libc::c_float
        - 15 as libc::c_int as libc::c_float)
        * y
        + 10 as libc::c_int as libc::c_float)
        * y
        * y
        * y;
    z -= pz as libc::c_float;
    w = ((z * 6 as libc::c_int as libc::c_float
        - 15 as libc::c_int as libc::c_float)
        * z
        + 10 as libc::c_int as libc::c_float)
        * z
        * z
        * z;
    r0 = stb__perlin_randtab[x0 as usize] as libc::c_int;
    r0 = stb__perlin_randtab[(r0 + seed as libc::c_int) as usize] as libc::c_int;
    r1 = stb__perlin_randtab[x1 as usize] as libc::c_int;
    r1 = stb__perlin_randtab[(r1 + seed as libc::c_int) as usize] as libc::c_int;
    r00 = stb__perlin_randtab[(r0 + y0) as usize] as libc::c_int;
    r01 = stb__perlin_randtab[(r0 + y1) as usize] as libc::c_int;
    r10 = stb__perlin_randtab[(r1 + y0) as usize] as libc::c_int;
    r11 = stb__perlin_randtab[(r1 + y1) as usize] as libc::c_int;
    n000 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r00 + z0) as usize] as libc::c_int,
        x,
        y,
        z,
    );
    n001 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r00 + z1) as usize] as libc::c_int,
        x,
        y,
        z - 1 as libc::c_int as libc::c_float,
    );
    n010 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r01 + z0) as usize] as libc::c_int,
        x,
        y - 1 as libc::c_int as libc::c_float,
        z,
    );
    n011 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r01 + z1) as usize] as libc::c_int,
        x,
        y - 1 as libc::c_int as libc::c_float,
        z - 1 as libc::c_int as libc::c_float,
    );
    n100 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r10 + z0) as usize] as libc::c_int,
        x - 1 as libc::c_int as libc::c_float,
        y,
        z,
    );
    n101 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r10 + z1) as usize] as libc::c_int,
        x - 1 as libc::c_int as libc::c_float,
        y,
        z - 1 as libc::c_int as libc::c_float,
    );
    n110 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r11 + z0) as usize] as libc::c_int,
        x - 1 as libc::c_int as libc::c_float,
        y - 1 as libc::c_int as libc::c_float,
        z,
    );
    n111 = stb__perlin_grad(
        stb__perlin_randtab_grad_idx[(r11 + z1) as usize] as libc::c_int,
        x - 1 as libc::c_int as libc::c_float,
        y - 1 as libc::c_int as libc::c_float,
        z - 1 as libc::c_int as libc::c_float,
    );
    n00 = stb__perlin_lerp(n000, n001, w);
    n01 = stb__perlin_lerp(n010, n011, w);
    n10 = stb__perlin_lerp(n100, n101, w);
    n11 = stb__perlin_lerp(n110, n111, w);
    n0 = stb__perlin_lerp(n00, n01, v);
    n1 = stb__perlin_lerp(n10, n11, v);
    return stb__perlin_lerp(n0, n1, u);
}
#[no_mangle]
pub unsafe extern "C" fn inner(
    mut which: libc::c_int,
    mut x: libc::c_float,
    mut y: libc::c_float,
    mut z: libc::c_float,
    mut x_wrap: libc::c_int,
    mut y_wrap: libc::c_int,
    mut z_wrap: libc::c_int,
    mut seed: libc::c_int,
    mut lacunarity: libc::c_float,
    mut gain: libc::c_float,
    mut offset: libc::c_float,
    mut octaves: libc::c_int,
) -> libc::c_float {
    match which {
        0 => return stb_perlin_noise3(x, y, z, x_wrap, y_wrap, z_wrap),
        1 => return stb_perlin_noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed),
        2 => return stb_perlin_ridge_noise3(x, y, z, lacunarity, gain, offset, octaves),
        3 => return stb_perlin_fbm_noise3(x, y, z, lacunarity, gain, octaves),
        4 => return stb_perlin_turbulence_noise3(x, y, z, lacunarity, gain, octaves),
        5 => {
            return stb_perlin_noise3_wrap_nonpow2(
                x,
                y,
                z,
                x_wrap,
                y_wrap,
                z_wrap,
                seed as libc::c_uchar,
            );
        }
        _ => return ::core::f32::NAN,
    };
}
unsafe fn main_0() -> libc::c_int {
    let mut which: libc::c_int = 0 as libc::c_int;
    let mut x: libc::c_float = 0.0f32;
    let mut y: libc::c_float = 0.0f32;
    let mut z: libc::c_float = 0.0f32;
    let mut x_wrap: libc::c_int = 0 as libc::c_int;
    let mut y_wrap: libc::c_int = 0 as libc::c_int;
    let mut z_wrap: libc::c_int = 0 as libc::c_int;
    let mut seed: libc::c_int = 0 as libc::c_int;
    let mut lacunarity: libc::c_float = 0.0f32;
    let mut gain: libc::c_float = 0.0f32;
    let mut offset: libc::c_float = 0.0f32;
    let mut octaves: libc::c_int = 0 as libc::c_int;
    scanf(
        b"%d%f%f%f%d%d%d%d%f%f%f%d\0" as *const u8 as *const libc::c_char,
        &raw mut which,
        &raw mut x,
        &raw mut y,
        &raw mut z,
        &raw mut x_wrap,
        &raw mut y_wrap,
        &raw mut z_wrap,
        &raw mut seed,
        &raw mut lacunarity,
        &raw mut gain,
        &raw mut offset,
        &raw mut octaves,
    );
    let mut res: libc::c_float = inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );
    printf(
        b"%.9g\n\0" as *const u8 as *const libc::c_char,
        res as libc::c_double,
    );
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
