use bhshell::dynamicarr;

#[test]
fn test_get_args_empty_list() {
    let l = dynamicarr::ArgList::default();
    let args = dynamicarr::get_args(&l);
    assert!(args.is_empty());
}

#[test]
fn test_get_args_with_items() {
    let l = dynamicarr::ArgList {
        items: vec!["hello".to_string(), "world".to_string()],
        position: 2,
        bufsize: 16,
    };
    let args = dynamicarr::get_args(&l);
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "hello");
    assert_eq!(args[1], "world");
}

#[test]
fn test_get_args_single_item() {
    let l = dynamicarr::ArgList {
        items: vec!["single".to_string()],
        position: 1,
        bufsize: 16,
    };
    let args = dynamicarr::get_args(&l);
    assert_eq!(args.len(), 1);
    assert_eq!(args[0], "single");
}

#[test]
fn test_get_args_three_items() {
    let l = dynamicarr::ArgList {
        items: vec!["foo".to_string(), "bar".to_string(), "baz".to_string()],
        position: 3,
        bufsize: 16,
    };
    let args = dynamicarr::get_args(&l);
    assert_eq!(args.len(), 3);
    assert_eq!(args[0], "foo");
    assert_eq!(args[1], "bar");
    assert_eq!(args[2], "baz");
}

#[test]
fn test_get_string() {
    let s = dynamicarr::Str {
        items: "hello".to_string(),
        position: 5,
        bufsize: 16,
    };
    let result = dynamicarr::get_string(&s);
    assert_eq!(result, "hello");
    assert_eq!(result.len(), 5);
}

#[test]
fn test_get_string_single_char() {
    let s = dynamicarr::Str {
        items: "x".to_string(),
        position: 1,
        bufsize: 16,
    };
    let result = dynamicarr::get_string(&s);
    assert_eq!(result, "x");
    assert_eq!(result.len(), 1);
}

#[test]
fn test_get_string_empty() {
    let s = dynamicarr::Str::default();
    let result = dynamicarr::get_string(&s);
    assert_eq!(result, "");
}

#[test]
fn test_destroy_args_does_not_panic() {
    let args = vec!["a".to_string(), "b".to_string()];
    dynamicarr::destroy_args(args);
}

fn main() {}
