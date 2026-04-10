use crate::tisp::*;

pub type MkFn = fn(&mut Tsp, &str) -> Val;

fn tsp_arg_num_check(args: &Val, name: &str, nargs: i32) -> bool {
    if nargs > -1 && tsp_lstlen(args) != nargs {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
            name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        false
    } else { true }
}

fn tsp_arg_min_check(args: &Val, name: &str, nargs: i32) -> bool {
    if tsp_lstlen(args) < nargs {
        eprintln!("; tisp: error: {}: expected at least {} argument{}, received {}",
            name, nargs, if nargs > 1 { "s" } else { "" }, tsp_lstlen(args));
        false
    } else { true }
}

fn tsp_arg_type_check(arg: &Val, name: &str, type_bits: u32) -> bool {
    if (arg.t as u32) & type_bits == 0 {
        eprintln!("; tisp: error: {}: expected {}, received {}",
            name, tsp_type_str_bits(type_bits), tsp_type_str(arg.t));
        false
    } else { true }
}

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut ret = String::new();
    let mut cur = &args;
    while cur.t == TspType::TspPair {
        let v = car(cur);
        match v.t {
            TspType::TspNone => {}
            TspType::TspNil => ret.push_str("Nil"),
            TspType::TspInt => ret.push_str(&format!("{}", vnum(v) as i32)),
            TspType::TspDec => ret.push_str(&format!("{:.15}", vnum(v)).trim_end_matches('0').trim_end_matches('.')),
            TspType::TspRatio => ret.push_str(&format!("{}/{}", vnum(v) as i32, vden(v) as i32)),
            TspType::TspStr | TspType::TspSym => ret.push_str(vs(v)),
            _ => {
                eprintln!("; tisp: error: could not convert type {} into string", tsp_type_str(v.t));
                return mk_err();
            }
        }
        cur = cdr(cur);
    }
    mk_fn(st, &ret)
}

#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_min_check(&args, "Str", 1) { return mk_err(); }
    val_string(st, args, mk_str_val)
}

#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_min_check(&args, "Sym", 1) { return mk_err(); }
    val_string(st, args, mk_sym_val)
}

pub fn prim_strlen(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_min_check(&args, "strlen", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "strlen", TspType::TspStr as u32 | TspType::TspSym as u32) { return mk_err(); }
    mk_int(vs(car(&args)).len() as i32)
}

pub fn form_strformat(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !tsp_arg_num_check(&args, "strformat", 1) { return mk_err(); }
    if !tsp_arg_type_check(car(&args), "strformat", TspType::TspStr as u32) { return mk_err(); }

    let str_val = vs(car(&args)).to_string();
    let mut ret = String::new();
    let bytes = str_val.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] != b'{' {
            // Find matching }
            let old_file = st.file.clone();
            let old_filec = st.filec;
            st.file = str_val[i + 1..].to_string();
            st.filec = 0;
            let v = match read_pair(st, '}') {
                Some(v) => v,
                None => return mk_err(),
            };
            let consumed = st.filec;
            st.file = old_file;
            st.filec = old_filec;
            i += 1 + consumed;

            // Eval the parsed expressions
            let evaled = match tisp_eval_list(st, env, v) {
                Some(v) => v,
                None => return mk_err(),
            };
            // Convert to string
            let s = val_string(st, evaled, mk_str_val);
            if is_err_val(&s) { return s; }
            ret.push_str(vs(&s));
        } else if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            ret.push('{');
            i += 2;
        } else if bytes[i] == b'}' && i + 1 < bytes.len() && bytes[i + 1] == b'}' {
            ret.push('}');
            i += 2;
        } else {
            ret.push(bytes[i] as char);
            i += 1;
        }
    }

    mk_str(st, &ret).unwrap_or_else(|| mk_err())
}

pub fn tib_env_string(st: &mut Tsp) {
    tisp_env_add(st, "Sym", mk_prim(TspType::TspPrim, prim_Sym, "Sym").unwrap());
    tisp_env_add(st, "Str", mk_prim(TspType::TspPrim, prim_Str, "Str").unwrap());
    tisp_env_add(st, "strlen", mk_prim(TspType::TspPrim, prim_strlen, "strlen").unwrap());
    tisp_env_add(st, "strformat", mk_prim(TspType::TspForm, form_strformat, "strformat").unwrap());
}
