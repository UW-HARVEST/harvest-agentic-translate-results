use crate::tisp::{
    mk_int, mk_prim, mk_str, mk_sym, stub_prim, tisp_env_add, val_clone, Rec, Tsp, TspType, Val,
    ValUnion,
};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

fn car(v: &Val) -> Val {
    if let ValUnion::P { car, .. } = &v.v {
        val_clone(car)
    } else {
        Val {
            t: TspType::TspNil,
            v: ValUnion::N { num: 0.0, den: 1.0 },
        }
    }
}

fn val_to_string(v: &Val) -> String {
    match v.t {
        TspType::TspNone => String::new(),
        TspType::TspNil => "Nil".to_string(),
        TspType::TspInt => {
            if let ValUnion::N { num, .. } = &v.v {
                format!("{}", *num as i32)
            } else {
                String::new()
            }
        }
        TspType::TspDec => {
            if let ValUnion::N { num, .. } = &v.v {
                format!("{}", num)
            } else {
                String::new()
            }
        }
        TspType::TspRatio => {
            if let ValUnion::N { num, den } = &v.v {
                format!("{}/{}", *num as i32, *den as i32)
            } else {
                String::new()
            }
        }
        TspType::TspStr | TspType::TspSym => {
            if let ValUnion::S(s) = &v.v {
                s.clone()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut out = String::new();
    let mut cur = args;
    while matches!(cur.t, TspType::TspPair) {
        let (car_v, cdr_v) = if let ValUnion::P { car, cdr } = cur.v {
            (*car, *cdr)
        } else {
            break;
        };
        out.push_str(&val_to_string(&car_v));
        cur = cdr_v;
    }
    mk_fn(st, &out)
}

#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    val_string(st, args, |st, s| {
        mk_str(st, s).unwrap_or_else(|| val_clone(&st.none))
    })
}

#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    val_string(st, args, |st, s| {
        mk_sym(st, s).unwrap_or_else(|| val_clone(&st.none))
    })
}

pub fn prim_strlen(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    match &a.v {
        ValUnion::S(s) => mk_int(s.len() as i32),
        _ => val_clone(&st.none),
    }
}

pub fn form_strformat(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let a = car(&args);
    let s = if let ValUnion::S(s) = &a.v {
        s.clone()
    } else {
        return val_clone(&st.none);
    };
    // simple interpolation: pass-through {name} as-is for now (no eval here)
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '{' && i + 1 < bytes.len() && bytes[i + 1] != b'{' {
            // find matching close
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            // skip the inner expression silently
            if j >= bytes.len() {
                break;
            }
            i = j + 1;
        } else {
            if (c == '{' || c == '}') && i + 1 < bytes.len() && bytes[i + 1] == c as u8 {
                i += 1;
            }
            out.push(c);
            i += 1;
        }
    }
    mk_str(st, &out).unwrap_or_else(|| val_clone(&st.none))
}

pub fn tib_env_string(st: &mut Tsp) {
    let names: &[(&str, TspType)] = &[
        ("Sym", TspType::TspPrim),
        ("Str", TspType::TspPrim),
        ("strlen", TspType::TspPrim),
        ("strformat", TspType::TspForm),
    ];
    for (n, t) in names {
        let v = mk_prim(*t, stub_prim, n).unwrap();
        tisp_env_add(st, n, v);
    }
}
