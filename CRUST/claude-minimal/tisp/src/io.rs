use crate::tisp::{mk_prim, tisp_env_add, Rec, Tsp, TspType, Val};
use std::cell::RefCell;
use std::rc::Rc;

pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut p = 0i32;
    let mut b = 0i32;
    let mut c = 0i32;
    for (i, ch) in s.chars().enumerate() {
        if i as i32 >= len {
            break;
        }
        match ch {
            '(' => p += 1,
            ')' => p -= 1,
            '[' => b += 1,
            ']' => b -= 1,
            '{' => c += 1,
            '}' => c -= 1,
            _ => {}
        }
    }
    if p != 0 {
        return p;
    }
    if b != 0 {
        return b;
    }
    c
}

pub fn read_file(fname: &str) -> String {
    std::fs::read_to_string(fname).unwrap_or_default()
}

pub fn prim_write(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_read(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_parse(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_load(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn tib_env_io(st: &mut Tsp) {
    let v = mk_prim(TspType::TspPrim, prim_write, "write").unwrap();
    tisp_env_add(st, "write", v);
    let v = mk_prim(TspType::TspPrim, prim_read, "read").unwrap();
    tisp_env_add(st, "read", v);
    let v = mk_prim(TspType::TspPrim, prim_parse, "parse").unwrap();
    tisp_env_add(st, "parse", v);
    let v = mk_prim(TspType::TspPrim, prim_load, "load").unwrap();
    tisp_env_add(st, "load", v);
}
