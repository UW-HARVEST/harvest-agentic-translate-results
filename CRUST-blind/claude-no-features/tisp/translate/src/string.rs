use crate::tisp::{
    mk_int, mk_prim, mk_str, mk_sym, nil_val, none_val, pairp, tisp_env_add, tsp_lstlen,
    tsp_type_str, val_car, val_cdr, val_den, val_num, val_str, warn, Rec, Tsp, TspType, Val,
    ValUnion,
};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut ret = String::new();
    let mut cur = args;
    while pairp(&cur) {
        let (head_opt, rest) = if let ValUnion::P { car, cdr } = cur.v {
            (Some(*car), *cdr)
        } else {
            (None, nil_val())
        };
        let v = match head_opt {
            Some(v) => v,
            None => break,
        };
        match v.t {
            TspType::TspNone => {}
            TspType::TspNil => ret.push_str("Nil"),
            TspType::TspInt => ret.push_str(&format!("{}", val_num(&v) as i64)),
            TspType::TspDec => ret.push_str(&format_g(val_num(&v))),
            TspType::TspRatio => {
                ret.push_str(&format!("{}/{}", val_num(&v) as i64, val_den(&v) as i64));
            }
            TspType::TspStr | TspType::TspSym => {
                ret.push_str(val_str(&v));
            }
            _ => {
                warn(&format!(
                    "could not convert type {} into string",
                    tsp_type_str(v.t)
                ));
                return none_val();
            }
        }
        cur = rest;
    }
    mk_fn(st, &ret)
}

fn format_g(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    let abs_n = n.abs();
    if abs_n < 1e-4 || abs_n >= 1e15 {
        format!("{:e}", n)
    } else {
        let s = format!("{:.15}", n);
        let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
        if s.is_empty() {
            "0".to_string()
        } else {
            s
        }
    }
}

#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("Str: expected at least 1 argument");
        return none_val();
    }
    val_string(st, args, str_helper)
}

fn str_helper(st: &mut Tsp, s: &str) -> Val {
    mk_str(st, s).unwrap_or_else(nil_val)
}

#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("Sym: expected at least 1 argument");
        return none_val();
    }
    val_string(st, args, sym_helper)
}

fn sym_helper(st: &mut Tsp, s: &str) -> Val {
    mk_sym(st, s).unwrap_or_else(nil_val)
}

pub fn prim_strlen(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) < 1 {
        warn("strlen: expected at least 1 argument");
        return none_val();
    }
    let v = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !matches!(v.t, TspType::TspStr | TspType::TspSym) {
        warn("strlen: expected Str or Sym");
        return none_val();
    }
    mk_int(val_str(&v).len() as i32)
}

pub fn form_strformat(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_lstlen(&args) != 1 {
        warn("strformat: expected 1 argument");
        return none_val();
    }
    let v = val_car(&args).cloned().unwrap_or_else(nil_val);
    if !matches!(v.t, TspType::TspStr) {
        warn("strformat: expected Str");
        return none_val();
    }
    let str_input = val_str(&v).to_string();
    let bytes = str_input.as_bytes();
    let mut ret = String::with_capacity(bytes.len() * 2);
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '{' && i + 1 < bytes.len() && bytes[i + 1] != b'{' {
            // find matching '}'
            i += 1;
            let start = i;
            let mut depth = 1;
            while i < bytes.len() && depth > 0 {
                if bytes[i] == b'{' {
                    depth += 1;
                } else if bytes[i] == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                i += 1;
            }
            let inner = &str_input[start..i];
            // parse and evaluate inner
            let prev_file = std::mem::take(&mut st.file);
            let prev_filec = st.filec;
            st.file = format!("({})", inner);
            st.filec = 0;
            let parsed = crate::tisp::tisp_read(st);
            st.file = prev_file;
            st.filec = prev_filec;
            if let Some(parsed) = parsed {
                if let crate::tisp::ValUnion::P { car: _, cdr } = parsed.v {
                    if let Some(evaluated) = crate::tisp::tisp_eval_list(st, env, *cdr) {
                        let s = val_string(st, evaluated, str_helper);
                        ret.push_str(val_str(&s));
                    }
                }
            }
            i += 1; // skip closing brace
        } else {
            if c == '{' || c == '}' {
                i += 1; // skip the doubled brace
                if i < bytes.len() {
                    ret.push(bytes[i] as char);
                    i += 1;
                }
            } else {
                ret.push(c);
                i += 1;
            }
        }
    }
    mk_str(st, &ret).unwrap_or_else(nil_val)
}

pub fn tib_env_string(st: &mut Tsp) {
    add(st, "Sym", TspType::TspPrim);
    add(st, "Str", TspType::TspPrim);
    add(st, "strlen", TspType::TspPrim);
    add(st, "strformat", TspType::TspForm);
}

fn add(st: &mut Tsp, name: &str, t: TspType) {
    let v = mk_prim(t, dummy_prim, name).unwrap_or_else(nil_val);
    tisp_env_add(st, name, v);
}

fn dummy_prim(_st: Tsp, _env: Rec, _args: Val) -> Val {
    none_val()
}
