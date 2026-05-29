use tisp_proj::tisp::*;

#[allow(dead_code)]
fn make_st() -> Tsp {
    let mut st = tisp_env_init(64);
    tib_env_core(&mut st);
    tib_env_math(&mut st);
    tib_env_string(&mut st);
    tib_env_io(&mut st);
    tib_env_os(&mut st);
    st
}

#[test]
fn test_tsp_type_str_all() {
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
fn test_isnum_basic() {
    assert!(isnum("0"));
    assert!(isnum("9"));
    assert!(isnum("123"));
    assert!(isnum(".5"));
    assert!(isnum("-5"));
    assert!(isnum("+5"));
    assert!(isnum("-.5"));
    assert!(isnum("+.5"));
    assert!(!isnum(""));
    assert!(!isnum("a"));
    assert!(!isnum("-"));
    assert!(!isnum("+"));
    assert!(!isnum("."));
    assert!(!isnum("-a"));
    assert!(!isnum("foo"));
}

#[test]
fn test_is_sym() {
    assert!(is_sym('a'));
    assert!(is_sym('Z'));
    assert!(is_sym('0'));
    assert!(is_sym('9'));
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
}

#[test]
fn test_hash() {
    assert_eq!(hash(""), 0);
    // C: h * 33 + c
    // "a" -> 0*33 + 97 = 97
    assert_eq!(hash("a"), 97);
    // "ab" -> 97*33 + 98 = 3201 + 98 = 3299
    assert_eq!(hash("ab"), 3299);
    // "abc" -> 3299*33 + 99 = 108867 + 99 = 108966
    assert_eq!(hash("abc"), 108966);
}

#[test]
fn test_mk_int() {
    let v = mk_int(42);
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, den } = &v.v {
        assert_eq!(*num, 42.0);
        assert_eq!(*den, 1.0);
    } else {
        panic!("expected N union");
    }

    let zero = mk_int(0);
    assert!(matches!(zero.t, TspType::TspInt));
    if let ValUnion::N { num, den } = &zero.v {
        assert_eq!(*num, 0.0);
        assert_eq!(*den, 1.0);
    } else {
        panic!("expected N union");
    }

    let neg = mk_int(-7);
    if let ValUnion::N { num, den } = &neg.v {
        assert_eq!(*num, -7.0);
        assert_eq!(*den, 1.0);
    } else {
        panic!("expected N union");
    }
}

#[test]
fn test_mk_dec() {
    let v = mk_dec(3.14).unwrap();
    assert!(matches!(v.t, TspType::TspDec));
    if let ValUnion::N { num, den } = &v.v {
        assert_eq!(*num, 3.14);
        assert_eq!(*den, 1.0);
    } else {
        panic!("expected N union");
    }
}

#[test]
fn test_mk_rat_basic() {
    // 4/8 should reduce to 1/2 (Ratio)
    let v = mk_rat(4, 8).unwrap();
    assert!(matches!(v.t, TspType::TspRatio));
    if let ValUnion::N { num, den } = &v.v {
        assert_eq!(*num, 1.0);
        assert_eq!(*den, 2.0);
    } else {
        panic!("expected N union");
    }

    // 8/4 should simplify to int 2
    let v2 = mk_rat(8, 4).unwrap();
    assert!(matches!(v2.t, TspType::TspInt));
    if let ValUnion::N { num, den } = &v2.v {
        assert_eq!(*num, 2.0);
        assert_eq!(*den, 1.0);
    }

    // 2/1 should simplify to int 2
    let v3 = mk_rat(2, 1).unwrap();
    assert!(matches!(v3.t, TspType::TspInt));

    // 1/-2 should become -1/2 (numerator gets negative)
    let v4 = mk_rat(1, -2).unwrap();
    assert!(matches!(v4.t, TspType::TspRatio));
    if let ValUnion::N { num, den } = &v4.v {
        assert_eq!(*num, -1.0);
        assert_eq!(*den, 2.0);
    } else {
        panic!("expected N union");
    }

    // -6/-3 reduces to 2 (positive)
    let v5 = mk_rat(-6, -3).unwrap();
    assert!(matches!(v5.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &v5.v {
        assert_eq!(*num, 2.0);
    }

    // -6/3 reduces to -2
    let v6 = mk_rat(-6, 3).unwrap();
    assert!(matches!(v6.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &v6.v {
        assert_eq!(*num, -2.0);
    }

    // Division by zero
    assert!(mk_rat(1, 0).is_none());
}

#[test]
fn test_frac_reduce() {
    let mut a = 4i32;
    let mut b = 8i32;
    frac_reduce(&mut a, &mut b);
    assert_eq!(a, 1);
    assert_eq!(b, 2);

    let mut a = 6i32;
    let mut b = 9i32;
    frac_reduce(&mut a, &mut b);
    assert_eq!(a, 2);
    assert_eq!(b, 3);

    let mut a = 7i32;
    let mut b = 5i32;
    frac_reduce(&mut a, &mut b);
    assert_eq!(a, 7);
    assert_eq!(b, 5);
}

#[test]
fn test_tsp_lstlen() {
    let mut st = make_st();
    // empty list (Nil) => length 0
    let nil = Val { t: TspType::TspNil, v: ValUnion::N { num: 0.0, den: 1.0 } };
    assert_eq!(tsp_lstlen(&nil), 0);

    // (1 2 3) => length 3
    let one = mk_int(1);
    let two = mk_int(2);
    let three = mk_int(3);
    let nil2 = Val { t: TspType::TspNil, v: ValUnion::N { num: 0.0, den: 1.0 } };
    let lst = mk_pair(one, mk_pair(two, mk_pair(three, nil2).unwrap()).unwrap()).unwrap();
    assert_eq!(tsp_lstlen(&lst), 3);

    // improper list (1 . 2) => -2
    let p = mk_pair(mk_int(1), mk_int(2)).unwrap();
    assert_eq!(tsp_lstlen(&p), -2);

    // single int (not pair) => -1
    assert_eq!(tsp_lstlen(&mk_int(5)), -1);

    let _ = &mut st;
}

#[test]
fn test_mk_pair_and_accessors() {
    let p = mk_pair(mk_int(1), mk_int(2)).unwrap();
    assert!(matches!(p.t, TspType::TspPair));
    if let ValUnion::P { car, cdr } = &p.v {
        assert!(matches!(car.t, TspType::TspInt));
        assert!(matches!(cdr.t, TspType::TspInt));
    } else {
        panic!("expected pair");
    }
}

#[test]
fn test_mk_sym_intern() {
    let mut st = make_st();
    let v1 = mk_sym(&mut st, "foo").unwrap();
    let v2 = mk_sym(&mut st, "foo").unwrap();
    assert!(matches!(v1.t, TspType::TspSym));
    assert!(matches!(v2.t, TspType::TspSym));
    if let (ValUnion::S(s1), ValUnion::S(s2)) = (&v1.v, &v2.v) {
        assert_eq!(s1, "foo");
        assert_eq!(s2, "foo");
    } else {
        panic!("expected sym");
    }
}

#[test]
fn test_mk_str_intern() {
    let mut st = make_st();
    let v = mk_str(&mut st, "hello").unwrap();
    assert!(matches!(v.t, TspType::TspStr));
    if let ValUnion::S(s) = &v.v {
        assert_eq!(s, "hello");
    } else {
        panic!("expected str");
    }
}

#[test]
fn test_vals_eq_ints() {
    assert!(vals_eq(&mk_int(1), &mk_int(1)));
    assert!(!vals_eq(&mk_int(1), &mk_int(2)));
    // 2/1 == 2 (ratio reduces to int)
    let r = mk_rat(2, 1).unwrap();
    assert!(vals_eq(&r, &mk_int(2)));
    // 1/2 == 1/2
    let r1 = mk_rat(1, 2).unwrap();
    let r2 = mk_rat(2, 4).unwrap();
    assert!(vals_eq(&r1, &r2));
    // 4/5 != 1/2
    let r3 = mk_rat(4, 5).unwrap();
    assert!(!vals_eq(&r1, &r3));
}

#[test]
fn test_vals_eq_pairs() {
    let p1 = mk_pair(mk_int(1), mk_pair(mk_int(2), Val{ t: TspType::TspNil, v: ValUnion::N{num:0.0,den:1.0}}).unwrap()).unwrap();
    let p2 = mk_pair(mk_int(1), mk_pair(mk_int(2), Val{ t: TspType::TspNil, v: ValUnion::N{num:0.0,den:1.0}}).unwrap()).unwrap();
    assert!(vals_eq(&p1, &p2));
    let p3 = mk_pair(mk_int(1), mk_pair(mk_int(3), Val{ t: TspType::TspNil, v: ValUnion::N{num:0.0,den:1.0}}).unwrap()).unwrap();
    assert!(!vals_eq(&p1, &p3));
}

#[test]
fn test_read_int_basic() {
    let mut st = make_st();
    st.file = "12345".to_string();
    st.filec = 0;
    assert_eq!(read_int(&mut st), 12345);
    assert_eq!(st.filec, 5);

    st.file = "0".to_string();
    st.filec = 0;
    assert_eq!(read_int(&mut st), 0);

    st.file = "abc".to_string();
    st.filec = 0;
    assert_eq!(read_int(&mut st), 0);
    assert_eq!(st.filec, 0);
}

#[test]
fn test_read_sign() {
    let mut st = make_st();
    st.file = "-".to_string();
    st.filec = 0;
    assert_eq!(read_sign(&mut st), -1);
    assert_eq!(st.filec, 1);

    st.file = "+".to_string();
    st.filec = 0;
    assert_eq!(read_sign(&mut st), 1);
    assert_eq!(st.filec, 1);

    st.file = "5".to_string();
    st.filec = 0;
    assert_eq!(read_sign(&mut st), 1);
    assert_eq!(st.filec, 0);
}

#[test]
fn test_skip_ws() {
    let mut st = make_st();
    st.file = "   abc".to_string();
    st.filec = 0;
    skip_ws(&mut st, 1);
    assert_eq!(st.filec, 3);

    st.file = "  ; comment\nabc".to_string();
    st.filec = 0;
    skip_ws(&mut st, 1);
    assert_eq!(st.filec, 12);
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
}

#[test]
fn test_rec_new_and_get() {
    let r = rec_new(8, None);
    assert_eq!(r.cap, 8);
    assert_eq!(r.size, 0);
    // get on empty rec returns None
    assert!(rec_get(&r, "foo").is_none());
}

#[test]
fn test_rec_add_get() {
    let mut r = rec_new(8, None);
    rec_add(&mut r, "foo", mk_int(42));
    let v = rec_get(&r, "foo").unwrap();
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &v.v {
        assert_eq!(*num, 42.0);
    }
    // overwrite same key
    rec_add(&mut r, "foo", mk_int(100));
    let v2 = rec_get(&r, "foo").unwrap();
    if let ValUnion::N { num, .. } = &v2.v {
        assert_eq!(*num, 100.0);
    }
    // missing key
    assert!(rec_get(&r, "bar").is_none());
}

#[test]
fn test_tisp_env_init_basic() {
    let st = tisp_env_init(64);
    assert!(matches!(st.nil.t, TspType::TspNil));
    assert!(matches!(st.none.t, TspType::TspNone));
    assert!(matches!(st.t.t, TspType::TspSym));
    assert_eq!(st.filec, 0);
    // env should have True, Nil, Void, bt, version
    assert!(rec_get(&st.env, "True").is_some());
    assert!(rec_get(&st.env, "Nil").is_some());
    assert!(rec_get(&st.env, "Void").is_some());
    assert!(rec_get(&st.env, "bt").is_some());
    let ver = rec_get(&st.env, "version").unwrap();
    if let ValUnion::S(s) = &ver.v {
        assert_eq!(s, "0.1");
    } else {
        panic!("expected str for version");
    }
}

#[test]
fn test_read_num_basic() {
    let mut st = make_st();
    st.file = "42".to_string();
    st.filec = 0;
    let v = read_num(&mut st);
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &v.v {
        assert_eq!(*num, 42.0);
    }

    st.file = "3.14".to_string();
    st.filec = 0;
    let v = read_num(&mut st);
    assert!(matches!(v.t, TspType::TspDec));
    if let ValUnion::N { num, .. } = &v.v {
        assert!((*num - 3.14).abs() < 1e-12);
    }

    st.file = "1/2".to_string();
    st.filec = 0;
    let v = read_num(&mut st);
    assert!(matches!(v.t, TspType::TspRatio));
    if let ValUnion::N { num, den } = &v.v {
        assert_eq!(*num, 1.0);
        assert_eq!(*den, 2.0);
    }

    st.file = "-7".to_string();
    st.filec = 0;
    let v = read_num(&mut st);
    assert!(matches!(v.t, TspType::TspInt));
    if let ValUnion::N { num, .. } = &v.v {
        assert_eq!(*num, -7.0);
    }
}

#[test]
fn test_mk_list_basic() {
    let mut st = make_st();
    let one = mk_int(1);
    let two = mk_int(2);
    let three = mk_int(3);
    let lst = mk_list(&mut st, 3, vec![one, two, three]).unwrap();
    assert_eq!(tsp_lstlen(&lst), 3);
}

fn main() {}
