use crate::tisp::{mk_prim, tisp_env_add, Rec, Tsp, TspType, Val};
use std::cell::RefCell;
use std::rc::Rc;

pub type MkFn = fn(&mut Tsp, &str) -> Option<Val>;

pub fn val_string(_st: &mut Tsp, _args: Val, _mk_fn: MkFn) -> Option<Val> {
    None
}

pub fn prim_Str(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_Sym(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_strlen(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn form_strformat(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn tib_env_string(st: &mut Tsp) {
    let v = mk_prim(TspType::TspPrim, prim_Str, "Str").unwrap();
    tisp_env_add(st, "Str", v);
    let v = mk_prim(TspType::TspPrim, prim_Sym, "Sym").unwrap();
    tisp_env_add(st, "Sym", v);
    let v = mk_prim(TspType::TspPrim, prim_strlen, "strlen").unwrap();
    tisp_env_add(st, "strlen", v);
    let v = mk_prim(TspType::TspForm, form_strformat, "strformat").unwrap();
    tisp_env_add(st, "strformat", v);
}
