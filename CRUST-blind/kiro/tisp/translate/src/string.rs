use crate::tisp::*;

pub type MkFn = fn(&mut Tsp, &str) -> Val;

fn tsp_arg_check(args: &Val, name: &str, n: i32) {
    let len = tsp_lstlen(args);
    if n > -1 && len != n {
        eprintln!("; tisp: error: {}: expected {} argument{}, received {}",
            name, n, if n > 1 { "s" } else { "" }, len);
    }
}
fn tsp_arg_min_check(args: &Val, name: &str, n: i32) {
    let len = tsp_lstlen(args);
    if len < n {
        eprintln!("; tisp: error: {}: expected at least {} argument{}, received {}",
            name, n, if n > 1 { "s" } else { "" }, len);
    }
}
fn tsp_type_check(v: &Val, name: &str, type_mask: u32) {
    if (v.t as u32) & type_mask == 0 {
        eprintln!("; tisp: error: {}: expected {}, received {}",
            name, tsp_type_str_mask(type_mask), tsp_type_str(v.t));
    }
}
fn tsp_type_str_mask(t: u32) -> &'static str {
    if t == TspType::TspStr as u32 { return "Str"; }
    if t == (TspType::TspStr as u32 | TspType::TspSym as u32) { return "Str"; }
    "Invalid"
}

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {

fn format_g_15(n: f64) -> String {
    // Emulate C's %.15g
    if n == 0.0 { return "0".to_string(); }
    let abs_n = n.abs();
    let exp = abs_n.log10().floor() as i32;
    if exp >= -4 && exp < 15 {
        let decimal_places = if 14 - exp > 0 { (14 - exp) as usize } else { 0 };
        let s = format!("{:.prec$}", n, prec = decimal_places);
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else { s }
    } else {
        let s = format!("{:.14e}", n);
        if let Some(epos) = s.find('e') {
            let mantissa = s[..epos].trim_end_matches('0').trim_end_matches('.');
            let exp_val: i32 = s[epos+1..].parse().unwrap_or(0);
            format!("{}e{:+03}", mantissa, exp_val)
        } else { s }
    }
}

    let mut ret = String::new();
    let mut cur = &args;
    while cur.t == TspType::TspPair {
        let v = car_ref(cur);
        match v.t {
            TspType::TspNone => {}
            TspType::TspNil => ret.push_str("Nil"),
            TspType::TspInt => ret.push_str(&format!("{}", num_of(v) as i32)),
            TspType::TspDec => {
                // Emulate C's %.15g
                let n = num_of(v);
                let s = format_g_15(n);
                ret.push_str(&s);
            }
            TspType::TspRatio => ret.push_str(&format!("{}/{}", num_of(v) as i32, den_of(v) as i32)),
            TspType::TspStr | TspType::TspSym => ret.push_str(sym_str(v)),
            _ => {
                eprintln!("; tisp: error: could not convert type {} into string", tsp_type_str(v.t));
                return mk_val(TspType::TspNone);
            }
        }
        cur = cdr_ref(cur);
    }
    mk_fn(st, &ret)
}

#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "Str", 1);
    val_string(st, args, mk_str_val)
}

#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "Sym", 1);
    val_string(st, args, mk_sym_val)
}

pub fn prim_strlen(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    tsp_arg_min_check(&args, "strlen", 1);
    tsp_type_check(car_ref(&args), "strlen", TspType::TspStr as u32 | TspType::TspSym as u32);
    mk_int(sym_str(car_ref(&args)).len() as i32)
}

pub fn form_strformat(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    tsp_arg_check(&args, "strformat", 1);
    tsp_type_check(car_ref(&args), "strformat", TspType::TspStr as u32);
    let input = sym_str(car_ref(&args)).to_string();
    let mut ret = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] != b'{' {
            // Find matching }
            i += 1;
            let expr_start = i;
            // Parse the expression inside braces using tisp reader
            let old_file = st.file.clone();
            let old_filec = st.filec;
            st.file = input[expr_start..].to_string();
            st.filec = 0;
            let v = read_pair(st, '}');
            let consumed = st.filec;
            st.file = old_file;
            st.filec = old_filec;
            i = expr_start + consumed;

            match v {
                Some(v) => {
                    let evaled = tisp_eval_list(st, env, v);
                    match evaled {
                        Some(evaled) => {
                            let s = val_string(st, evaled, mk_str_val);
                            ret.push_str(sym_str(&s));
                        }
                        None => return mk_val(TspType::TspNone),
                    }
                }
                None => return mk_val(TspType::TspNone),
            }
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
    mk_str(st, &ret).unwrap()
}

pub fn tib_env_string(st: &mut Tsp) {
    tisp_env_add(st, "Sym", mk_prim(TspType::TspPrim, prim_Sym, "Sym").unwrap());
    tisp_env_add(st, "Str", mk_prim(TspType::TspPrim, prim_Str, "Str").unwrap());
    tisp_env_add(st, "strlen", mk_prim(TspType::TspPrim, prim_strlen, "strlen").unwrap());
    tisp_env_add(st, "strformat", mk_prim(TspType::TspForm, form_strformat, "strformat").unwrap());
}
