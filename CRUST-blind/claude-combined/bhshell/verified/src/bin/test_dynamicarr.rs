use bhshell::dynamicarr::{
    self, da_append_arg, da_append_char, destroy_args, get_args, get_string, ArgList, Str,
    DA_BUFFER_SIZE,
};

#[test]
fn test_da_buffer_size_constant() {
    assert_eq!(DA_BUFFER_SIZE, 16);
}

#[test]
fn test_str_default() {
    let s = Str::default();
    assert_eq!(s.position, 0);
    assert_eq!(s.bufsize, 0);
    assert_eq!(s.items, "");
}

#[test]
fn test_arg_list_default() {
    let l = ArgList::default();
    assert_eq!(l.position, 0);
    assert_eq!(l.bufsize, 0);
    assert!(l.items.is_empty());
}

#[test]
fn test_str_append_chars() {
    let mut s = Str::default();
    da_append_char(&mut s, 'a');
    assert_eq!(s.items.as_bytes()[0], b'a');
    da_append_char(&mut s, 'b');
    assert_eq!(s.items.as_bytes()[1], b'b');
    da_append_char(&mut s, 'c');
    assert_eq!(s.position, 3);
    let result = get_string(&s);
    assert_eq!(result.len(), 3);
    assert_eq!(result, "abc");
}

#[test]
fn test_get_string_empty() {
    let s = Str::default();
    let result = get_string(&s);
    assert_eq!(result, "");
    assert_eq!(result.len(), 0);
}

#[test]
fn test_arg_list_append() {
    let mut l = ArgList::default();
    da_append_arg(&mut l, "hello".to_string());
    da_append_arg(&mut l, "world".to_string());
    assert_eq!(l.items[0], "hello");
    assert_eq!(l.items[1], "world");
    assert_eq!(l.position, 2);
}

#[test]
fn test_get_args_basic() {
    let mut l = ArgList::default();
    da_append_arg(&mut l, "hello".to_string());
    da_append_arg(&mut l, "world".to_string());
    let args = get_args(&l);
    assert_eq!(args[0], "hello");
    assert_eq!(args[1], "world");
    assert_eq!(args.len(), 2);
}

#[test]
fn test_get_args_empty() {
    let l = ArgList::default();
    let args = get_args(&l);
    assert!(args.is_empty());
}

#[test]
fn test_destroy_args_does_not_panic() {
    let v = vec!["a".to_string(), "b".to_string()];
    destroy_args(v);
    // Should not panic and should drop cleanly.
}

#[test]
fn test_destroy_args_empty() {
    let v: Vec<String> = Vec::new();
    destroy_args(v);
}

#[test]
fn test_str_grows_past_initial_buffer() {
    let mut s = Str::default();
    // Append 20 characters > DA_BUFFER_SIZE = 16
    for c in "abcdefghijklmnopqrst".chars() {
        da_append_char(&mut s, c);
    }
    assert_eq!(s.position, 20);
    let result = get_string(&s);
    assert_eq!(result, "abcdefghijklmnopqrst");
    assert_eq!(result.len(), 20);
}

#[test]
fn test_get_string_constructs_from_partial() {
    let mut s = Str::default();
    da_append_char(&mut s, 'x');
    da_append_char(&mut s, 'y');
    da_append_char(&mut s, 'z');
    let result = get_string(&s);
    assert_eq!(result, "xyz");
}

// Keep the dynamicarr import live in test binary linkage
#[test]
fn test_module_constant_again() {
    assert_eq!(dynamicarr::DA_BUFFER_SIZE, 16);
}

fn main() {}
