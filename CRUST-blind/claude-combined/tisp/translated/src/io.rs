use crate::tisp::{mk_val, Rec, Tsp, TspType, Val};

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0i32;
    let mut bcount = 0i32;
    let mut ccount = 0i32;
    let bytes = s.as_bytes();
    let max = (len as usize).min(bytes.len());
    for &c in bytes.iter().take(max) {
        if c == 0 {
            break;
        }
        match c as char {
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
    if fname.is_empty() {
        return String::new();
    }
    std::fs::read_to_string(fname).unwrap_or_default()
}

pub fn prim_write(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    crate::tisp::clone_val(&st.none)
}

pub fn prim_read(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    crate::tisp::clone_val(&st.nil)
}

pub fn prim_parse(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspNone)
}

pub fn prim_load(st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    crate::tisp::clone_val(&st.none)
}

pub fn tib_env_io(_st: &mut Tsp) {
    // Registration would happen here in full impl
}
