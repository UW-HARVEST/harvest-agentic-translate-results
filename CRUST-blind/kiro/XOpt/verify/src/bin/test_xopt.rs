use XOpt::xopt;

fn make_options() -> Vec<xopt::XoptOption> {
    vec![
        xopt::XoptOption {
            long_arg: Some("some-int".to_string()),
            short_arg: 'i',
            offset: 0, // offset of someInt (i32) in data buffer
            callback: None,
            options: xopt::XOPT_TYPE_INT,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some integer value.".to_string()),
        },
        xopt::XoptOption {
            long_arg: Some("some-float".to_string()),
            short_arg: 'f',
            offset: 4, // offset of someFloat (f32)
            callback: None,
            options: xopt::XOPT_TYPE_FLOAT,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some float value.".to_string()),
        },
        xopt::XoptOption {
            long_arg: Some("some-double".to_string()),
            short_arg: 'd',
            offset: 8, // offset of someDouble (f64)
            callback: None,
            options: xopt::XOPT_TYPE_DOUBLE,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some double value.".to_string()),
        },
        xopt::XoptOption {
            long_arg: Some("help".to_string()),
            short_arg: '?',
            offset: 16, // offset of help (bool)
            callback: None,
            options: xopt::XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: Some("Shows this help message".to_string()),
        },
        xopt::XOPT_NULLOPTION(),
    ]
}

// Data buffer layout: [i32 @ 0, f32 @ 4, f64 @ 8, bool @ 16]
const DATA_SIZE: usize = 24;

fn new_data() -> Vec<u8> {
    vec![0u8; DATA_SIZE]
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_ne_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_f32(data: &[u8], offset: usize) -> f32 {
    f32::from_ne_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_f64(data: &[u8], offset: usize) -> f64 {
    f64::from_ne_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_bool(data: &[u8], offset: usize) -> bool {
    data[offset] != 0
}

#[test]
fn test_xopt_context_creation() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let ctx = xopt::xopt_context(
        Some("test-prog"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    );
    assert!(err.is_none());
    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.name, Some("test-prog".to_string()));
    assert_eq!(ctx.flags, xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT);
    assert!(!ctx.doubledash);
}

#[test]
fn test_parse_no_args() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test"];
    let count = xopt::xopt_parse(&mut ctx, 1, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 0);
    assert!(extras.is_some());
    assert_eq!(extras.unwrap().len(), 0);
    assert_eq!(read_i32(&data, 0), 0);
    assert_eq!(read_f32(&data, 4), 0.0);
    assert_eq!(read_f64(&data, 8), 0.0);
    assert!(!read_bool(&data, 16));
}

#[test]
fn test_parse_short_int() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "-i", "42"];
    let count = xopt::xopt_parse(&mut ctx, 3, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 0);
    assert_eq!(read_i32(&data, 0), 42);
}

#[test]
fn test_parse_long_float() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "--some-float=3.14"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 0);
    let f = read_f32(&data, 4);
    assert!((f - 3.14f32).abs() < 0.001);
}

#[test]
fn test_parse_long_double() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "--some-double=2.718"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 0);
    let d = read_f64(&data, 8);
    assert!((d - 2.718).abs() < 0.001);
}

#[test]
fn test_parse_extras() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "-i", "10", "extra1", "extra2"];
    let count = xopt::xopt_parse(&mut ctx, 5, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 2);
    assert_eq!(read_i32(&data, 0), 10);
    let ex = extras.unwrap();
    assert_eq!(ex.len(), 2);
    assert_eq!(ex[0], "extra1");
    assert_eq!(ex[1], "extra2");
}

#[test]
fn test_parse_bool_help() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "--help"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 0);
    assert!(read_bool(&data, 16));
}

#[test]
fn test_parse_invalid_option_strict() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "--bogus"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_some());
    assert!(err.as_ref().unwrap().contains("invalid option"));
    assert!(err.as_ref().unwrap().contains("bogus"));
    assert_eq!(count, 0);
    assert!(extras.is_none());
}

#[test]
fn test_parse_double_dash() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "--", "--not-an-option"];
    let count = xopt::xopt_parse(&mut ctx, 3, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 1);
    let ex = extras.unwrap();
    assert_eq!(ex[0], "--not-an-option");
}

#[test]
fn test_parse_missing_value() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "-i"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_some());
    assert!(err.as_ref().unwrap().contains("missing option value"));
    assert_eq!(count, 0);
}

#[test]
fn test_parse_bad_number() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "-i", "abc"];
    let count = xopt::xopt_parse(&mut ctx, 3, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_some());
    assert!(err.as_ref().unwrap().contains("valid number"));
    assert_eq!(count, 0);
}

#[test]
fn test_parse_posixmeharder() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "extra1", "-i", "5"];
    let count = xopt::xopt_parse(&mut ctx, 4, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_some());
    assert!(err.as_ref().unwrap().contains("options cannot be specified after arguments"));
    assert_eq!(count, 0);
}

#[test]
fn test_parse_keepfirst() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_KEEPFIRST,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    // With KEEPFIRST, argv[0] is treated as an extra
    let argv: Vec<&str> = vec!["test", "-i", "5"];
    let count = xopt::xopt_parse(&mut ctx, 3, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    // "test" is an extra, then -i 5 is parsed
    assert_eq!(count, 1);
    let ex = extras.unwrap();
    assert_eq!(ex[0], "test");
    assert_eq!(read_i32(&data, 0), 5);
}

#[test]
fn test_parse_singular_dash() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    // A singular "-" should be treated as an extra
    let argv: Vec<&str> = vec!["test", "-"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 1);
    let ex = extras.unwrap();
    assert_eq!(ex[0], "-");
}

#[test]
fn test_autohelp() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let autohelp = xopt::XoptAutohelpOptions {
        usage: Some("usage: test [options]".to_string()),
        prefix: Some("A test program.".to_string()),
        suffix: Some("End.".to_string()),
        spacer: 2,
    };

    let mut output = Vec::new();
    xopt::xopt_autohelp(&mut ctx, &mut output, Some(&autohelp), &mut err);
    assert!(err.is_none());

    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("usage: test [options]"));
    assert!(text.contains("A test program."));
    assert!(text.contains("End."));
    assert!(text.contains("--some-int=n"));
    assert!(text.contains("--some-float=n"));
    assert!(text.contains("--some-double=n"));
    assert!(text.contains("--help"));
    assert!(text.contains("-i"));
    assert!(text.contains("-f"));
    assert!(text.contains("-d"));
    assert!(text.contains("-?"));
}

#[test]
fn test_autohelp_no_options() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        0,
        &mut err,
    ).unwrap();

    let mut output = Vec::new();
    xopt::xopt_autohelp(&mut ctx, &mut output, None, &mut err);
    assert!(err.is_none());
    let text = String::from_utf8(output).unwrap();
    // Should still print options even without autohelp options
    assert!(text.contains("--some-int"));
}

#[test]
fn test_nulloption() {
    let null = xopt::XOPT_NULLOPTION();
    assert!(null.long_arg.is_none());
    assert_eq!(null.short_arg, '\0');
    assert_eq!(null.offset, 0);
    assert!(null.callback.is_none());
    assert_eq!(null.options, 0);
    assert!(null.arg_descrip.is_none());
    assert!(null.descrip.is_none());
}

#[test]
fn test_constants() {
    assert_eq!(xopt::XOPT_TYPE_STRING, 0x1);
    assert_eq!(xopt::XOPT_TYPE_INT, 0x2);
    assert_eq!(xopt::XOPT_TYPE_LONG, 0x4);
    assert_eq!(xopt::XOPT_TYPE_FLOAT, 0x8);
    assert_eq!(xopt::XOPT_TYPE_DOUBLE, 0x10);
    assert_eq!(xopt::XOPT_TYPE_BOOL, 0x20);
    assert_eq!(xopt::XOPT_OPTIONAL, 0x40);
    assert_eq!(xopt::XOPT_CTX_KEEPFIRST, 0x1);
    assert_eq!(xopt::XOPT_CTX_POSIXMEHARDER, 0x2);
    assert_eq!(xopt::XOPT_CTX_NOCONDENSE, 0x4);
    assert_eq!(xopt::XOPT_CTX_SLOPPYSHORTS, 0x8 | 0x4);
    assert_eq!(xopt::XOPT_CTX_STRICT, 0x10);
}

#[test]
fn test_parse_long_missing_value() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "--some-int"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_some());
    assert!(err.as_ref().unwrap().contains("missing option value"));
    assert_eq!(count, 0);
}

#[test]
fn test_parse_short_bool() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        xopt::XOPT_CTX_STRICT,
        &mut err,
    ).unwrap();

    let mut data = new_data();
    let mut extras: Option<Vec<String>> = None;
    let argv: Vec<&str> = vec!["test", "-?"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data.as_mut_ptr(), &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 0);
    assert!(read_bool(&data, 16));
}

fn main() {}
