use crate::tisp::{
    Rec, Tsp, TspType, Val, eval_in_env, expect_len, expect_min_len, expect_type, mk_int, mk_prim,
    mk_str, mk_sym, pair_car, pair_cdr, render_val, tisp_env_add, tisp_eval_list, tisp_read_line,
    val_is_nil, val_num, val_str,
};

pub type MkFn = fn(&mut Tsp, &str) -> Val;

fn mk_str_val(st: &mut Tsp, s: &str) -> Val {
    mk_str(st, s).unwrap_or_else(|| st.none.clone())
}

fn mk_sym_val(st: &mut Tsp, s: &str) -> Val {
    mk_sym(st, s).unwrap_or_else(|| st.none.clone())
}

pub fn val_string(st: &mut Tsp, args: Val, mk_fn: MkFn) -> Val {
    let mut out = String::new();
    let mut cur = args;
    while cur.t == TspType::TspPair {
        let v = pair_car(&cur).clone();
        match v.t {
            TspType::TspNone => {}
            TspType::TspNil => out.push_str("Nil"),
            TspType::TspInt => out.push_str(&(val_num(&v) as i32).to_string()),
            TspType::TspDec | TspType::TspRatio | TspType::TspStr | TspType::TspSym => {
                out.push_str(&render_val(&v))
            }
            _ => return st.none.clone(),
        }
        cur = pair_cdr(&cur).clone();
    }
    mk_fn(st, &out)
}

pub fn prim_Str(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_min_len(st, &args, "Str", 1) {
        return st.none.clone();
    }
    val_string(st, args, mk_str_val)
}

pub fn prim_Sym(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_min_len(st, &args, "Sym", 1) {
        return st.none.clone();
    }
    val_string(st, args, mk_sym_val)
}

pub fn prim_strlen(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if !expect_min_len(st, &args, "strlen", 1) {
        return st.none.clone();
    }
    let arg = pair_car(&args).clone();
    if !expect_type(st, &arg, "strlen", TspType::TspStr as u32 | TspType::TspSym as u32) {
        return st.none.clone();
    }
    mk_int(val_str(&arg).unwrap_or_default().len() as i32)
}

pub fn form_strformat(st: &mut Tsp, env: &mut Rec, args: Val) -> Val {
    if !expect_len(st, &args, "strformat", 1) {
        return st.none.clone();
    }
    let arg = pair_car(&args).clone();
    if !expect_type(st, &arg, "strformat", TspType::TspStr as u32) {
        return st.none.clone();
    }
    let input = val_str(&arg).unwrap_or_default();
    let mut out = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '{' && chars.get(i + 1) != Some(&'{') {
            let mut depth = 1usize;
            let mut j = i + 1;
            while j < chars.len() && depth > 0 {
                match chars[j] {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    break;
                }
                j += 1;
            }
            if j >= chars.len() {
                return st.none.clone();
            }
            let expr: String = chars[i + 1..j].iter().collect();
            let old_file = st.file.clone();
            let old_filec = st.filec;
            st.file = expr;
            st.filec = 0;
            let parsed = crate::tisp::read_pair(st, '}').or_else(|| crate::tisp::tisp_read(st));
            st.file = old_file;
            st.filec = old_filec;
            if let Some(v) = parsed.and_then(|v| tisp_eval_list(st, env, v)) {
                out.push_str(&render_val(&val_string(st, v, mk_str_val)));
            }
            i = j + 1;
            continue;
        }
        if (chars[i] == '{' || chars[i] == '}') && chars.get(i + 1) == Some(&chars[i]) {
            out.push(chars[i]);
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    mk_str(st, &out).unwrap_or_else(|| st.none.clone())
}

pub fn tib_env_string(st: &mut Tsp) {
    tisp_env_add(st, "Sym", mk_prim(TspType::TspPrim, prim_Sym, "Sym").unwrap());
    tisp_env_add(st, "Str", mk_prim(TspType::TspPrim, prim_Str, "Str").unwrap());
    tisp_env_add(st, "strlen", mk_prim(TspType::TspPrim, prim_strlen, "strlen").unwrap());
    tisp_env_add(st, "strformat", mk_prim(TspType::TspForm, form_strformat, "strformat").unwrap());
}
