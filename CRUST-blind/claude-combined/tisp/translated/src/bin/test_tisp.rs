use tisp_proj::tisp::{
    self, hash, isnum, is_op, is_sym, mk_dec, mk_int, mk_pair, mk_rat, mk_val, rec_new, rec_add,
    rec_get, tsp_lstlen, tsp_type_str, vals_eq, esc_char, esc_str, frac_reduce, mk_list, mk_sym,
    mk_str, tisp_env_init, read_int, read_sign, TspType, ValUnion, TSP_NUM, TSP_RATIONAL, TSP_EXPR,
    TSP_OP_CHARS, TSP_SYM_CHARS, TSP_REC_FACTOR, TSP_REC_MAX_PRINT,
};

#[test]
fn test_constants() {
    assert_eq!(TSP_REC_MAX_PRINT, 64);
    assert_eq!(TSP_REC_FACTOR, 2);
    assert_eq!(TSP_OP_CHARS, "_+-*/\\|=^<>.:");
    assert_eq!(TSP_SYM_CHARS, "_!?@#$%&~*-");
}

#[test]
fn test_type_bitmasks() {
    // TSP_NUM = INT | RATIO | DEC
    let expected_num = TspType::TspInt as u32 | TspType::TspRatio as u32 | TspType::TspDec as u32;
    assert_eq!(TSP_NUM, expected_num);
    let expected_rat = TspType::TspInt as u32 | TspType::TspRatio as u32;
    assert_eq!(TSP_RATIONAL, expected_rat);
    let expected_expr = TSP_NUM | TspType::TspSym as u32 | TspType::TspPair as u32;
    assert_eq!(TSP_EXPR, expected_expr);
}

#[test]
fn test_tsp_type_str() {
    assert_eq!(tsp_type_str(TspType::TspNone), "Void");
    assert_eq!(tsp_type_str(TspType::TspNil), "Nil");
    assert_eq!(tsp_type_str(TspType::TspInt), "Int");
    assert_eq!(tsp_type_str(TspType::TspDec), "Dec");
    assert_eq!(tsp_type_str(TspType::TspRatio), "Ratio");
    assert_eq!(tsp_type_str(TspType::TspStr), "Str");
    assert_eq!(tsp_type_str(TspType::TspSym), "Sym");
    assert_eq!(tsp_type_str(TspType::TspPrim), "Prim");
    assert_eq!(tsp_type_str(TspType::TspForm), "Form");
    assert_eq!(tsp_type_str(TspType::TspFunc), "Func");
    assert_eq!(tsp_type_str(TspType::TspMacro), "Macro");
    assert_eq!(tsp_type_str(TspType::TspPair), "Pair");
    assert_eq!(tsp_type_str(TspType::TspRec), "Rec");
}

#[test]
fn test_is_sym() {
    assert!(is_sym('a'));
    assert!(is_sym('Z'));
    assert!(is_sym('5'));
    assert!(is_sym('_'));
    assert!(is_sym('!'));
    assert!(is_sym('?'));
    assert!(is_sym('@'));
    assert!(is_sym('#'));
    assert!(is_sym('$'));
    assert!(is_sym('%'));
    assert!(is_sym('&'));
    assert!(is_sym('~'));
    assert!(is_sym('*'));
    assert!(is_sym('-'));
    assert!(!is_sym(' '));
    assert!(!is_sym('('));
    assert!(!is_sym(')'));
    assert!(!is_sym('+'));
}

#[test]
fn test_is_op() {
    assert!(is_op('_'));
    assert!(is_op('+'));
    assert!(is_op('-'));
    assert!(is_op('*'));
    assert!(is_op('/'));
    assert!(is_op('\\'));
    assert!(is_op('|'));
    assert!(is_op('='));
    assert!(is_op('^'));
    assert!(is_op('<'));
    assert!(is_op('>'));
    assert!(is_op('.'));
    assert!(is_op(':'));
    assert!(!is_op('a'));
    assert!(!is_op('1'));
    assert!(!is_op(' '));
    assert!(!is_op('!'));
}

#[test]
fn test_isnum() {
    assert!(isnum("123"));
    assert!(isnum("0"));
    assert!(isnum("9foo"));
    assert!(isnum(".5"));
    assert!(isnum(".0"));
    assert!(isnum("-1"));
    assert!(isnum("-12"));
    assert!(isnum("+1"));
    assert!(isnum("+12"));
    assert!(isnum("-.5"));
    assert!(isnum("+.5"));

    assert!(!isnum("foo"));
    assert!(!isnum(""));
    assert!(!isnum("."));
    assert!(!isnum(".a"));
    assert!(!isnum("-"));
    assert!(!isnum("+"));
    assert!(!isnum("-a"));
    assert!(!isnum("+a"));
}

#[test]
fn test_esc_char() {
    assert_eq!(esc_char('n'), '\n');
    assert_eq!(esc_char('r'), '\r');
    assert_eq!(esc_char('t'), '\t');
    assert_eq!(esc_char('\n'), ' ');
    assert_eq!(esc_char('\\'), '\\');
    assert_eq!(esc_char('"'), '"');
    assert_eq!(esc_char('a'), 'a');
    assert_eq!(esc_char('z'), 'z');
}

#[test]
fn test_esc_str_no_escape() {
    let s = esc_str("hello", 5, 0);
    assert_eq!(s, "hello");
}

#[test]
fn test_esc_str_with_escape() {
    // \n should become newline
    let s = esc_str("a\\nb", 4, 1);
    assert_eq!(s, "a\nb");
    // \t becomes tab
    let s2 = esc_str("a\\tb", 4, 1);
    assert_eq!(s2, "a\tb");
}

#[test]
fn test_esc_str_partial_len() {
    let s = esc_str("hello world", 5, 0);
    assert_eq!(s, "hello");
}

#[test]
fn test_frac_reduce() {
    let mut n = 6i32;
    let mut d = 3i32;
    frac_reduce(&mut n, &mut d);
    assert_eq!(n, 2);
    assert_eq!(d, 1);

    let mut n = 4i32;
    let mut d = 8i32;
    frac_reduce(&mut n, &mut d);
    assert_eq!(n, 1);
    assert_eq!(d, 2);

    let mut n = 2384i32;
    let mut d = 7238i32;
    frac_reduce(&mut n, &mut d);
    assert_eq!(n, 1192);
    assert_eq!(d, 3619);
}

#[test]
fn test_hash_basic() {
    // hash("") = 0
    assert_eq!(hash(""), 0);
    // hash("a") = 0*33 + 'a' = 97
    assert_eq!(hash("a"), 97);
    // hash("ab") = 97*33 + 'b' = 3201 + 98 = 3299
    assert_eq!(hash("ab"), 3299);
}

#[test]
fn test_mk_int() {
    let v = mk_int(42);
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, den } = v.v {
        assert_eq!(num, 42.0);
        assert_eq!(den, 1.0);
    } else {
        panic!("expected ValUnion::N");
    }
}

#[test]
fn test_mk_dec() {
    let v = mk_dec(3.14).expect("mk_dec");
    assert!(matches!(v.t, TspType::TspDec));
    if let ValUnion::N { num, den } = v.v {
        assert_eq!(num, 3.14);
        assert_eq!(den, 1.0);
    } else {
        panic!("expected ValUnion::N");
    }
}

#[test]
fn test_mk_rat_simplifies_to_int() {
    let v = mk_rat(6, 3).expect("mk_rat");
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, den } = v.v {
        assert_eq!(num, 2.0);
        assert_eq!(den, 1.0);
    }
}

#[test]
fn test_mk_rat_keeps_ratio() {
    let v = mk_rat(1, 2).expect("mk_rat");
    assert!(matches!(v.t, TspType::TspRatio));
    if let ValUnion::N { num, den } = v.v {
        assert_eq!(num, 1.0);
        assert_eq!(den, 2.0);
    }
}

#[test]
fn test_mk_rat_negative_denom() {
    // 1/-2 should become -1/2
    let v = mk_rat(1, -2).expect("mk_rat");
    assert!(matches!(v.t, TspType::TspRatio));
    if let ValUnion::N { num, den } = v.v {
        assert_eq!(num, -1.0);
        assert_eq!(den, 2.0);
    }
}

#[test]
fn test_mk_rat_div_by_zero() {
    let v = mk_rat(5, 0);
    assert!(v.is_none());
}

#[test]
fn test_mk_rat_reduces() {
    // 4/8 = 1/2
    let v = mk_rat(4, 8).expect("mk_rat");
    if let ValUnion::N { num, den } = v.v {
        assert_eq!(num, 1.0);
        assert_eq!(den, 2.0);
    }
}

#[test]
fn test_mk_rat_negative_signs() {
    // -6/-3 = 2
    let v = mk_rat(-6, -3).expect("mk_rat");
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = v.v {
        assert_eq!(num, 2.0);
    }
}

#[test]
fn test_tsp_lstlen_nil() {
    let nil = mk_val(TspType::TspNil);
    assert_eq!(tsp_lstlen(&nil), 0);
}

#[test]
fn test_tsp_lstlen_pair() {
    let nil = mk_val(TspType::TspNil);
    let pair = mk_pair(mk_int(1), nil).unwrap();
    assert_eq!(tsp_lstlen(&pair), 1);
}

#[test]
fn test_tsp_lstlen_three() {
    let nil = mk_val(TspType::TspNil);
    let p3 = mk_pair(mk_int(3), nil).unwrap();
    let p2 = mk_pair(mk_int(2), p3).unwrap();
    let p1 = mk_pair(mk_int(1), p2).unwrap();
    assert_eq!(tsp_lstlen(&p1), 3);
}

#[test]
fn test_tsp_lstlen_improper() {
    // (1 . 2) -> -2
    let p = mk_pair(mk_int(1), mk_int(2)).unwrap();
    assert_eq!(tsp_lstlen(&p), -2);
}

#[test]
fn test_mk_pair() {
    let p = mk_pair(mk_int(1), mk_int(2)).unwrap();
    assert!(matches!(p.t, TspType::TspPair));
    if let ValUnion::P { car, cdr } = p.v {
        assert!(matches!(car.t, TspType::TspInt));
        assert!(matches!(cdr.t, TspType::TspInt));
    }
}

#[test]
fn test_vals_eq_int() {
    let a = mk_int(5);
    let b = mk_int(5);
    let c = mk_int(6);
    assert!(vals_eq(&a, &b));
    assert!(!vals_eq(&a, &c));
}

#[test]
fn test_vals_eq_ratio_equal() {
    // 2/4 == 1/2
    let a = mk_rat(2, 4).unwrap();
    let b = mk_rat(1, 2).unwrap();
    assert!(vals_eq(&a, &b));
}

#[test]
fn test_vals_eq_ratio_int() {
    // 2/1 == 2
    let a = mk_rat(2, 1).unwrap();
    let b = mk_int(2);
    assert!(vals_eq(&a, &b));
}

#[test]
fn test_vals_eq_pair() {
    let nil = mk_val(TspType::TspNil);
    let p1 = mk_pair(mk_int(1), mk_pair(mk_int(2), tisp::clone_val(&nil)).unwrap()).unwrap();
    let p2 = mk_pair(mk_int(1), mk_pair(mk_int(2), nil).unwrap()).unwrap();
    assert!(vals_eq(&p1, &p2));
}

#[test]
fn test_read_sign_minus() {
    let mut st = tisp_env_init(16);
    st.file = "-5".to_string();
    st.filec = 0;
    assert_eq!(read_sign(&mut st), -1);
    assert_eq!(st.filec, 1);
}

#[test]
fn test_read_sign_plus() {
    let mut st = tisp_env_init(16);
    st.file = "+5".to_string();
    st.filec = 0;
    assert_eq!(read_sign(&mut st), 1);
    assert_eq!(st.filec, 1);
}

#[test]
fn test_read_sign_neither() {
    let mut st = tisp_env_init(16);
    st.file = "5".to_string();
    st.filec = 0;
    assert_eq!(read_sign(&mut st), 1);
    assert_eq!(st.filec, 0);
}

#[test]
fn test_read_int() {
    let mut st = tisp_env_init(16);
    st.file = "123abc".to_string();
    st.filec = 0;
    assert_eq!(read_int(&mut st), 123);
    assert_eq!(st.filec, 3);
}

#[test]
fn test_read_int_zero() {
    let mut st = tisp_env_init(16);
    st.file = "0".to_string();
    st.filec = 0;
    assert_eq!(read_int(&mut st), 0);
}

#[test]
fn test_rec_new_size_cap() {
    let r = rec_new(8, None);
    assert_eq!(r.size, 0);
    assert_eq!(r.cap, 8);
    assert_eq!(r.items.len(), 8);
    assert!(r.next.is_none());
}

#[test]
fn test_rec_add_get() {
    let mut r = rec_new(8, None);
    rec_add(&mut r, "foo", mk_int(42));
    assert_eq!(r.size, 1);
    let g = rec_get(&r, "foo").expect("get foo");
    assert!(matches!(g.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = g.v {
        assert_eq!(num, 42.0);
    }
}

#[test]
fn test_rec_get_missing() {
    let r = rec_new(8, None);
    assert!(rec_get(&r, "missing").is_none());
}

#[test]
fn test_rec_add_overwrite() {
    let mut r = rec_new(8, None);
    rec_add(&mut r, "k", mk_int(1));
    rec_add(&mut r, "k", mk_int(2));
    let g = rec_get(&r, "k").expect("get");
    if let ValUnion::N { num, .. } = g.v {
        assert_eq!(num, 2.0);
    }
}

#[test]
fn test_mk_str_dedups() {
    let mut st = tisp_env_init(16);
    let _ = mk_str(&mut st, "hello").expect("first");
    let v2 = mk_str(&mut st, "hello").expect("second");
    if let ValUnion::S(s) = v2.v {
        assert_eq!(s, "hello");
    }
}

#[test]
fn test_mk_sym_basic() {
    let mut st = tisp_env_init(16);
    let v = mk_sym(&mut st, "foo").expect("sym");
    assert!(matches!(v.t, TspType::TspSym));
    if let ValUnion::S(s) = v.v {
        assert_eq!(s, "foo");
    }
}

#[test]
fn test_tisp_env_init_constants() {
    let st = tisp_env_init(16);
    assert!(matches!(st.t.t, TspType::TspSym));
    if let ValUnion::S(s) = &st.t.v {
        assert_eq!(s, "True");
    }
    assert!(matches!(st.nil.t, TspType::TspNil));
    assert!(matches!(st.none.t, TspType::TspNone));
    // env should have True, Nil, Void, bt, version
    assert!(rec_get(&st.env, "True").is_some());
    assert!(rec_get(&st.env, "Nil").is_some());
    assert!(rec_get(&st.env, "Void").is_some());
    assert!(rec_get(&st.env, "bt").is_some());
    assert!(rec_get(&st.env, "version").is_some());
}

#[test]
fn test_mk_list_basic() {
    let mut st = tisp_env_init(16);
    let lst = mk_list(&mut st, 3, vec![mk_int(1), mk_int(2), mk_int(3)]).unwrap();
    assert!(matches!(lst.t, TspType::TspPair));
    assert_eq!(tsp_lstlen(&lst), 3);
}

#[test]
fn test_mk_list_empty() {
    let mut st = tisp_env_init(16);
    let lst = mk_list(&mut st, 0, vec![]).unwrap();
    assert!(matches!(lst.t, TspType::TspNil));
}

fn main() {}
