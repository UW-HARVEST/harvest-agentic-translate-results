use crate::tisp::*;

pub type MkFn = fn(&mut Tsp, &str) -> Val;

fn format_g15(n: f64) -> String {
    // Mimic C's snprintf(s, 22, "%.15g", v)
    // %.15g means 15 significant digits, shortest representation
    let s = format!("{:.15e}", n);
    let parts: Vec<&str> = s.split('e').collect();
    let mantissa = parts[0].trim_end_matches('0').trim_end_matches('.');
    let exp: i32 = parts[1].parse().unwrap();
    if exp >= 0 && exp < 15 {
        let digits: String = mantissa.replace('-', "").replace('.', "");
        let is_neg = n < 0.0;
        let dot_pos = if mantissa.replace('-', "").contains('.') {
            mantissa.replace('-', "").find('.').unwrap()
        } else { mantissa.replace('-', "").len() };
        let new_dot = dot_pos as i32 + exp;
        let mut r = if is_neg { "-".to_string() } else { String::new() };
        let clean_digits: String = digits.chars().filter(|c| *c != '-').collect();
        if new_dot >= clean_digits.len() as i32 {
            r.push_str(&clean_digits);
            for _ in 0..(new_dot as usize - clean_digits.len()) { r.push('0'); }
        } else {
            let (l, ri) = clean_digits.split_at(new_dot as usize);
            r.push_str(l);
            if !ri.is_empty() { r.push('.'); r.push_str(ri); }
        }
        r
    } else if exp < 0 && exp > -5 {
        let is_neg = n < 0.0;
        let digits: String = mantissa.replace('-', "").replace('.', "");
        let dot_pos = if mantissa.replace('-', "").contains('.') {
            mantissa.replace('-', "").find('.').unwrap()
        } else { mantissa.replace('-', "").len() };
        let new_dot = dot_pos as i32 + exp;
        let mut r = if is_neg { "-".to_string() } else { String::new() };
        if new_dot <= 0 {
            r.push_str("0.");
            for _ in 0..(-new_dot) { r.push('0'); }
            r.push_str(&digits);
        } else {
            let (l, ri) = digits.split_at(new_dot as usize);
            r.push_str(l); r.push('.'); r.push_str(ri);
        }
        r
    } else {
        format!("{}e{:+03}", mantissa, exp)
    }
}

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut ret = String::new();
    let mut cur = &args;
    while cur.t == TspType::TspPair {
        let v = car_pub(cur);
        match v.t {
            TspType::TspNone => {}
            TspType::TspNil => ret.push_str("Nil"),
            TspType::TspInt => ret.push_str(&format!("{}", num_pub(v) as i32)),
            TspType::TspDec => {
                let n = num_pub(v);
                let s = format!("{:.15e}", n);
                // Mimic C's %.15g: use shortest representation
                let formatted = format_g15(n);
                ret.push_str(&formatted);
            }
            TspType::TspRatio => ret.push_str(&format!("{}/{}", num_pub(v) as i32, den_pub(v) as i32)),
            TspType::TspStr | TspType::TspSym => ret.push_str(sym_str_pub(v)),
            _ => {
                eprintln!("; tisp: error: could not convert type {} into string", tsp_type_str(v.t));
                return mk_error();
            }
        }
        cur = cdr_pub(cur);
    }
    mk_fn(st, &ret)
}

fn mk_str_fn(st: &mut Tsp, s: &str) -> Val { mk_str(st, s) }
fn mk_sym_fn(st: &mut Tsp, s: &str) -> Val { mk_sym(st, s) }

#[allow(non_snake_case)]
pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "Str", 1).is_none() { return mk_error(); }
    val_string(st, args, mk_str_fn)
}
#[allow(non_snake_case)]
pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "Sym", 1).is_none() { return mk_error(); }
    val_string(st, args, mk_sym_fn)
}
pub fn prim_strlen(_st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if tsp_arg_min_check_pub(&args, "strlen", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "strlen", TspType::TspStr as u32 | TspType::TspSym as u32).is_none() { return mk_error(); }
    mk_int(sym_str_pub(car_pub(&args)).len() as i32)
}
pub fn form_strformat(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if tsp_arg_num_check_pub(&args, "strformat", 1).is_none() { return mk_error(); }
    if tsp_arg_type_check_pub(car_pub(&args), "strformat", TspType::TspStr as u32).is_none() { return mk_error(); }
    let s = sym_str_pub(car_pub(&args)).to_string();
    let mut ret = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i+1] != b'{' {
            // Find matching }
            let start = i + 1;
            let inner = &s[start..];
            // Parse the inner expression
            let old_file = std::mem::replace(&mut st.file, inner.to_string());
            let old_filec = std::mem::replace(&mut st.filec, 0);
            let v = read_pair(st, '}');
            let consumed = st.filec;
            st.file = old_file;
            st.filec = old_filec;
            i = start + consumed;
            if let Some(v) = v {
                if let Some(evaled) = tisp_eval_list_pub(st, env, v) {
                    let s_val = val_string(st, evaled, mk_str_fn);
                    if is_error(&s_val) { return mk_error(); }
                    ret.push_str(sym_str_pub(&s_val));
                } else { return mk_error(); }
            } else { return mk_error(); }
        } else if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i+1] == b'{' {
            ret.push('{'); i += 2;
        } else if bytes[i] == b'}' && i + 1 < bytes.len() && bytes[i+1] == b'}' {
            ret.push('}'); i += 2;
        } else {
            ret.push(bytes[i] as char); i += 1;
        }
    }
    mk_str(st, &ret)
}

pub fn tib_env_string(st: &mut Tsp) {
    tisp_env_add(st, "Sym", mk_prim(TspType::TspPrim, prim_Sym, "Sym"));
    tisp_env_add(st, "Str", mk_prim(TspType::TspPrim, prim_Str, "Str"));
    tisp_env_add(st, "strlen", mk_prim(TspType::TspPrim, prim_strlen, "strlen"));
    tisp_env_add(st, "strformat", mk_prim(TspType::TspForm, form_strformat, "strformat"));
}
