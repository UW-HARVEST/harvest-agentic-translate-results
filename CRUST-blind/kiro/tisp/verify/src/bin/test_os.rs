use tisp_proj::tisp::*;

fn setup() -> Tsp {
    let mut st = tisp_env_init(1024);
    tib_env_core(&mut st);
    tib_env_math(&mut st);
    tib_env_string(&mut st);
    tib_env_os(&mut st);
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
fn test_pwd() {
    let mut st = setup();
    let result = eval_str(&mut st, "(pwd)");
    assert!(!result.is_empty());
}

#[test]
fn test_cd_and_pwd() {
    let mut st = setup();
    let original = eval_str(&mut st, "(pwd)");
    assert_eq!(eval_str(&mut st, "(cd! \"/tmp\")"), "Void");
    let after = eval_str(&mut st, "(pwd)");
    assert_eq!(after, "/tmp");
    let restore = format!("(cd! \"{}\")", original);
    eval_str(&mut st, &restore);
}

#[test]
fn test_now() {
    let mut st = setup();
    let result = eval_str(&mut st, "(now)");
    let ts: i64 = result.parse().expect("now should return an integer");
    assert!(ts > 1000000000);
}

#[test]
fn test_time() {
    let mut st = setup();
    let result = eval_str(&mut st, "(time (+ 1 1))");
    let t: f64 = result.parse().expect("time should return a decimal");
    assert!(t >= 0.0);
}

fn main() {}
