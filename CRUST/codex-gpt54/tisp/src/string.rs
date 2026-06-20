use crate::tisp::{self, Prim, Rec, Tsp, TspType, Val, ValUnion};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

fn dummy(_: tisp::Tsp, _: tisp::Rec, _: tisp::Val) -> tisp::Val {
    tisp::mk_val(TspType::TspNone)
}

fn fallback(st: &Tsp) -> Val {
    let mut v = tisp::mk_val(TspType::TspNone);
    v.t = st.none.t;
    v
}

fn call(st: &mut Tsp, env: &mut Rec, name: &str, args: Val, form: bool) -> Val {
    let kind = if form { TspType::TspForm } else { TspType::TspPrim };
    let proc = tisp::mk_prim(kind, dummy as Prim, name).unwrap_or_else(|| fallback(st));
    tisp::eval_proc(st, env, proc, args).unwrap_or_else(|| fallback(st))
}

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut ret = String::new();
    let mut cur = &args;
    while cur.t == TspType::TspPair {
        let v = match &cur.v {
            ValUnion::P { car, .. } => car.as_ref(),
            _ => break,
        };
        match v.t {
            TspType::TspNone => {}
            _ => ret.push_str(&match v.t {
                TspType::TspNil => "Nil".to_string(),
                TspType::TspInt => format!("{}", match &v.v { ValUnion::N { num, .. } => *num as i32, _ => 0 }),
                TspType::TspDec => match &v.v {
                    ValUnion::N { num, .. } => {
                        let mut s = format!("{num}");
                        if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                            s.push_str(".0");
                        }
                        s
                    }
                    _ => String::new(),
                },
                TspType::TspRatio => match &v.v {
                    ValUnion::N { num, den } => format!("{}/{}", *num as i32, *den as i32),
                    _ => String::new(),
                },
                TspType::TspStr | TspType::TspSym => match &v.v {
                    ValUnion::S(s) => s.clone(),
                    _ => String::new(),
                },
                _ => String::new(),
            }),
        }
        cur = match &cur.v {
            ValUnion::P { cdr, .. } => cdr.as_ref(),
            _ => break,
        };
    }
    mk_fn(st, &ret)
}
pub fn prim_Str(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "Str", args, false)
}
pub fn prim_Sym(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "Sym", args, false)
}
pub fn prim_strlen(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "strlen", args, false)
}
pub fn form_strformat(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    call(st, env, "strformat", args, true)
}
pub fn tib_env_string(st: &mut Tsp) {
    tisp::tib_env_string(st);
}
