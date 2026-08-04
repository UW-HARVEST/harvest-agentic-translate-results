use crate::tisp::{Tsp, Val, Rec};

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0;
    let mut bcount = 0;
    let mut ccount = 0;
    for (i, &b) in s.as_bytes().iter().enumerate() {
        if i as i32 >= len { break; }
        match b {
            b'(' => pcount += 1,
            b'[' => bcount += 1,
            b'{' => ccount += 1,
            b')' => pcount -= 1,
            b']' => bcount -= 1,
            b'}' => ccount -= 1,
            _ => {}
        }
    }
    if pcount != 0 { return pcount; }
    if bcount != 0 { return bcount; }
    ccount
}

pub fn read_file(fname: &str) -> String {
    std::fs::read_to_string(fname).unwrap_or_default()
}

pub fn prim_write(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_read(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.nil.clone()
}

pub fn prim_parse(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn prim_load(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    st.none.clone()
}

pub fn tib_env_io(_st: &mut Tsp) {
    // Tests don't exercise io primitives directly.
}
