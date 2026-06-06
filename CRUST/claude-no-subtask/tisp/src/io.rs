use crate::tisp::{Tsp, Val, Rec, TspType, ValUnion};

#[allow(unused)]
fn empty() -> Val {
    Val { t: TspType::TspNone, v: ValUnion::S(String::new()) }
}

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0;
    let mut bcount = 0;
    let mut ccount = 0;
    let bytes = s.as_bytes();
    let limit = (len as usize).min(bytes.len());
    for i in 0..limit {
        if bytes[i] == 0 {
            break;
        }
        match bytes[i] as char {
            '(' => pcount += 1,
            '[' => bcount += 1,
            '{' => ccount += 1,
            ')' => pcount -= 1,
            ']' => bcount -= 1,
            '}' => ccount -= 1,
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
    std::fs::read_to_string(fname).unwrap_or_default()
}

pub fn prim_write(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_read(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_parse(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn prim_load(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    empty()
}

pub fn tib_env_io(_st: &mut Tsp) {
    // Stub
}
