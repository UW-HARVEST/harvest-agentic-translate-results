// This secret key is used to compute signatures for formulas, clauses,
// and (un)satisfiability certificates. See `c_src/src/trusted/secret.h`.
pub const SECRET_KEY: [u8; 16] = [
    86, 93, 1, 209, 112, 176, 13, 40,
    168, 223, 25, 22, 134, 58, 21, 211,
];
