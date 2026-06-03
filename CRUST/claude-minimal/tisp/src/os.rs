use crate::tisp::{mk_prim, tisp_env_add, Rec, Tsp, TspType, Val};
use std::cell::RefCell;
use std::rc::Rc;

pub fn prim_cd(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_pwd(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_exit(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn prim_now(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn form_time(_st: &mut Tsp, _env: &Rc<RefCell<Rec>>, _args: Val) -> Option<Val> {
    None
}

pub fn tib_env_os(st: &mut Tsp) {
    let v = mk_prim(TspType::TspPrim, prim_cd, "cd!").unwrap();
    tisp_env_add(st, "cd!", v);
    let v = mk_prim(TspType::TspPrim, prim_pwd, "pwd").unwrap();
    tisp_env_add(st, "pwd", v);
    let v = mk_prim(TspType::TspPrim, prim_exit, "exit!").unwrap();
    tisp_env_add(st, "exit!", v);
    let v = mk_prim(TspType::TspPrim, prim_now, "now").unwrap();
    tisp_env_add(st, "now", v);
    let v = mk_prim(TspType::TspForm, form_time, "time").unwrap();
    tisp_env_add(st, "time", v);
}
