use tisp_proj::os::{prim_now, prim_pwd};
use tisp_proj::tisp::{mk_val, rec_new, tisp_env_init, TspType, ValUnion};

#[test]
fn test_prim_pwd() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let nil = mk_val(TspType::TspNil);
    let r = prim_pwd(&mut st, &mut env, nil);
    assert!(matches!(r.t, TspType::TspStr));
    if let ValUnion::S(s) = r.v {
        // current dir from std::env::current_dir
        let expected = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        assert_eq!(s, expected);
    }
}

#[test]
fn test_prim_now_returns_int() {
    let mut st = tisp_env_init(16);
    let mut env = rec_new(8, None);
    let nil = mk_val(TspType::TspNil);
    let r = prim_now(&mut st, &mut env, nil);
    assert!(matches!(r.t, TspType::TspInt));
    if let ValUnion::N { num, den } = r.v {
        assert!(num > 0.0);
        assert_eq!(den, 1.0);
    }
}

fn main() {}
