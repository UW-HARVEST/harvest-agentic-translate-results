use tisp_proj::tisp::*;

fn setup() -> Tsp {
    let mut st = tisp_env_init(1024);
    tib_env_core(&mut st);
    tib_env_math(&mut st);
    tib_env_string(&mut st);
    st
}

fn eval_str(st: &mut Tsp, input: &str) -> String {
    st.file = input.to_string();
    st.filec = 0;
    let v = tisp_read(st).expect(&format!("read failed for: {}", input));
    let mut env = clone_rec(&st.env);
    let v = tisp_eval_with_env(st, &mut env, v).expect(&format!("eval failed for: {}", input));
    st.env = env;
    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &v);
    String::from_utf8(buf).unwrap()
}

#[test]
fn test_str_from_string() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(= (Str \"hello\") \"hello\")"), "True");
}

#[test]
fn test_str_from_int() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(= (Str 42) \"42\")"), "True");
}

#[test]
fn test_str_from_ratio() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(= (Str 3/4) \"3/4\")"), "True");
}

#[test]
fn test_str_from_decimal() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(= (Str 1.5) \"1.5\")"), "True");
}

#[test]
fn test_str_concat() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(= (Str \"hello\" \" \" \"world\") \"hello world\")"), "True");
}

#[test]
fn test_sym() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(typeof (Sym \"hello\"))"), "Sym");
    assert_eq!(eval_str(&mut st, "(typeof (Sym 42))"), "Sym");
}

#[test]
fn test_strlen() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(strlen \"hello\")"), "5");
    assert_eq!(eval_str(&mut st, "(strlen \"\")"), "0");
    assert_eq!(eval_str(&mut st, "(strlen \"foo bar\")"), "7");
}

fn main() {}
