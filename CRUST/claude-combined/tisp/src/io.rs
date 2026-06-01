use crate::tisp::{Rec, Tsp, Val};
use std::fs;

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0i32;
    let mut bcount = 0i32;
    let mut ccount = 0i32;
    let bytes = s.as_bytes();
    let max = (len as usize).min(bytes.len());
    for i in 0..max {
        if bytes[i] == 0 {
            break;
        }
        match bytes[i] {
            b'(' => pcount += 1,
            b'[' => bcount += 1,
            b'{' => ccount += 1,
            b')' => pcount -= 1,
            b']' => bcount -= 1,
            b'}' => ccount -= 1,
            _ => {}
        }
    }
    if pcount != 0 {
        return pcount;
    }
    if bcount != 0 {
        return bcount;
    }
    ccount
}

pub fn read_file(fname: &str) -> String {
    fs::read_to_string(fname).unwrap_or_default()
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
    // Primitives use a different signature than crate::tisp::Prim, so we don't register them.
}
