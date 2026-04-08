use bhshell::dynamicarr::{get_args, get_string, destroy_args, ArgList, Str};

#[test]
fn test_get_string_basic() {
    let s = Str {
        items: "abc".to_string(),
        position: 3,
        bufsize: 16,
    };
    assert_eq!(get_string(&s), "abc");
}

#[test]
fn test_get_string_partial() {
    let s = Str {
        items: "hello world".to_string(),
        position: 5,
        bufsize: 16,
    };
    assert_eq!(get_string(&s), "hello");
}

#[test]
fn test_get_string_empty() {
    let s = Str {
        items: String::new(),
        position: 0,
        bufsize: 0,
    };
    assert_eq!(get_string(&s), "");
}

#[test]
fn test_get_args_basic() {
    let l = ArgList {
        items: vec!["hello".to_string(), "world".to_string()],
        position: 2,
        bufsize: 16,
    };
    let args = get_args(&l);
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "hello");
    assert_eq!(args[1], "world");
}

#[test]
fn test_get_args_empty() {
    let l = ArgList {
        items: Vec::new(),
        position: 0,
        bufsize: 0,
    };
    let args = get_args(&l);
    assert!(args.is_empty());
}

#[test]
fn test_get_args_single() {
    let l = ArgList {
        items: vec!["only".to_string()],
        position: 1,
        bufsize: 16,
    };
    let args = get_args(&l);
    assert_eq!(args.len(), 1);
    assert_eq!(args[0], "only");
}

#[test]
fn test_get_args_partial_position() {
    let l = ArgList {
        items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        position: 2,
        bufsize: 16,
    };
    let args = get_args(&l);
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "a");
    assert_eq!(args[1], "b");
}

#[test]
fn test_destroy_args_no_panic() {
    let args = vec!["hello".to_string(), "world".to_string()];
    destroy_args(args);
}

#[test]
fn test_default_str() {
    let s = Str::default();
    assert_eq!(s.position, 0);
    assert_eq!(s.bufsize, 0);
    assert!(s.items.is_empty());
}

#[test]
fn test_default_arglist() {
    let l = ArgList::default();
    assert_eq!(l.position, 0);
    assert_eq!(l.bufsize, 0);
    assert!(l.items.is_empty());
}

fn main() {}
