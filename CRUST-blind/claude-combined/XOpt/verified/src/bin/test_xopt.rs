use XOpt::xopt::{
    self, ParsedValues, XoptAutohelpOptions, XoptOption, XOPT_CTX_KEEPFIRST,
    XOPT_CTX_NOCONDENSE, XOPT_CTX_POSIXMEHARDER, XOPT_CTX_SLOPPYSHORTS, XOPT_CTX_STRICT,
    XOPT_TYPE_BOOL, XOPT_TYPE_DOUBLE, XOPT_TYPE_FLOAT, XOPT_TYPE_INT, XOPT_TYPE_LONG,
    XOPT_TYPE_STRING,
};

// Construct the same options layout as in c_src/test/simple-test.c
fn simple_options() -> Vec<XoptOption> {
    vec![
        XoptOption {
            long_arg: Some("some-int".to_string()),
            short_arg: 'i',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_INT,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some integer value. Can set to whatever number you like.".to_string()),
        },
        XoptOption {
            long_arg: Some("some-float".to_string()),
            short_arg: 'f',
            offset: 1,
            callback: None,
            options: XOPT_TYPE_FLOAT,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some float value.".to_string()),
        },
        XoptOption {
            long_arg: Some("some-double".to_string()),
            short_arg: 'd',
            offset: 2,
            callback: None,
            options: XOPT_TYPE_DOUBLE,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some double value.".to_string()),
        },
        XoptOption {
            long_arg: Some("help".to_string()),
            short_arg: '?',
            offset: 3,
            callback: None,
            options: XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: Some("Shows this help message".to_string()),
        },
        xopt::XOPT_NULLOPTION(),
    ]
}

#[test]
fn test_xopt_context_creates_context() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let ctx = xopt::xopt_context(Some("xopt-test"), &opts, XOPT_CTX_STRICT, &mut err);
    assert!(err.is_none());
    let ctx = ctx.expect("context");
    assert_eq!(ctx.name, Some("xopt-test".to_string()));
    assert_eq!(ctx.flags, XOPT_CTX_STRICT);
    assert!(!ctx.doubledash);
    assert_eq!(ctx.options.len(), 5);
}

// From running C test:
//   ./simple-test --some-int=42 --some-float=3.14 --some-double=2.71828 file1 file2
//   someInt:	42
//   someFloat:	3.140000
//   someDouble:	2.718280
//   help:	0
//   extra count: 2 (file1, file2)
#[test]
fn test_xopt_parse_long_with_eq() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("xopt-test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec![
        "prog",
        "--some-int=42",
        "--some-float=3.14",
        "--some-double=2.71828",
        "file1",
        "file2",
    ];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    let n = xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none(), "err = {:?}", err);
    assert_eq!(n, 2);
    let extras = extras.unwrap();
    assert_eq!(extras.len(), 2);
    assert_eq!(extras[0], "file1");
    assert_eq!(extras[1], "file2");
    assert_eq!(data.ints.get(&0).copied(), Some(42));
    assert!((data.floats.get(&1).copied().unwrap() - 3.14).abs() < 1e-6);
    assert!((data.floats.get(&2).copied().unwrap() - 2.71828).abs() < 1e-9);
    assert_eq!(data.bools.get(&3).copied(), None);
}

// From C: ./simple-test -i 100 file1
//   someInt:	100
//   extra count: 1 (file1)
#[test]
fn test_xopt_parse_short_split() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-i", "100", "file1"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    let n = xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none());
    assert_eq!(n, 1);
    let extras = extras.unwrap();
    assert_eq!(extras.len(), 1);
    assert_eq!(extras[0], "file1");
    assert_eq!(data.ints.get(&0).copied(), Some(100));
}

// From C: ./simple-test -i 100 -f 0.5 -d 1.5
//   someInt:	100
//   someFloat:	0.500000
//   someDouble:	1.500000
//   extra count: 0
#[test]
fn test_xopt_parse_multiple_short() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-i", "100", "-f", "0.5", "-d", "1.5"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    let n = xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none());
    assert_eq!(n, 0);
    assert_eq!(extras.unwrap().len(), 0);
    assert_eq!(data.ints.get(&0).copied(), Some(100));
    assert!((data.floats.get(&1).copied().unwrap() - 0.5).abs() < 1e-6);
    assert!((data.floats.get(&2).copied().unwrap() - 1.5).abs() < 1e-9);
}

// From C: ./simple-test --invalid
//   Error: invalid option: --invalid
#[test]
fn test_xopt_parse_invalid_long() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "--invalid"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    let n = xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert_eq!(n, 0);
    assert_eq!(err.as_deref(), Some("invalid option: --invalid"));
}

// From C: ./simple-test --some-int=foo
//   Error: value isn't a valid number: --some-int=foo
#[test]
fn test_xopt_parse_invalid_number() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "--some-int=foo"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert_eq!(
        err.as_deref(),
        Some("value isn't a valid number: --some-int=foo")
    );
}

// From C: ./simple-test -i 100 -- --notparsed
//   someInt:	100
//   extra count: 1 (--notparsed)
#[test]
fn test_xopt_parse_doubledash() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-i", "100", "--", "--notparsed"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    let n = xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none());
    assert_eq!(n, 1);
    let extras = extras.unwrap();
    assert_eq!(extras.len(), 1);
    assert_eq!(extras[0], "--notparsed");
    assert_eq!(data.ints.get(&0).copied(), Some(100));
}

// From C: ./simple-test file1 -i 100
//   Error: options cannot be specified after arguments: 100
#[test]
fn test_xopt_parse_posixmeharder() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "file1", "-i", "100"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert_eq!(
        err.as_deref(),
        Some("options cannot be specified after arguments: 100")
    );
}

// Test boolean: `-?`
//  ./simple-test -?  -> shows help message (config.help=true)
#[test]
fn test_xopt_parse_bool_short() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-?"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    let n = xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none());
    assert_eq!(n, 0);
    assert_eq!(data.bools.get(&3).copied(), Some(true));
}

// xopt_autohelp produces the same text from C run:
// usage: simple-test [options] [extras...]
//
// A simple demonstration of the XOpt options parser library.
//
// -i, --some-int=n             Some integer value. Can set to whatever number you like.
// -f, --some-float=n           Some float value.
// -d, --some-double=n          Some double value.
// -?, --help                   Shows this help message
//
// End argument list.
#[test]
fn test_xopt_autohelp_output() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let auto = XoptAutohelpOptions {
        usage: Some("usage: simple-test [options] [extras...]".to_string()),
        prefix: Some(
            "A simple demonstration of the XOpt options parser library.".to_string(),
        ),
        suffix: Some("End argument list.".to_string()),
        spacer: 10,
    };
    let mut buf: Vec<u8> = Vec::new();
    xopt::xopt_autohelp(&mut ctx, &mut buf, Some(&auto), &mut err);
    assert!(err.is_none());
    let out = String::from_utf8(buf).unwrap();
    let expected = "usage: simple-test [options] [extras...]\n\
        \n\
        A simple demonstration of the XOpt options parser library.\n\
        \n\
        -i, --some-int=n             Some integer value. Can set to whatever number you like.\n\
        -f, --some-float=n           Some float value.\n\
        -d, --some-double=n          Some double value.\n\
        -?, --help                   Shows this help message\n\
        \n\
        End argument list.\n";
    assert_eq!(out, expected);
}

#[test]
fn test_xopt_autohelp_minimal() {
    // No options struct passed -> only options list output
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(Some("p"), &opts, 0, &mut err).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    xopt::xopt_autohelp(&mut ctx, &mut buf, None, &mut err);
    let out = String::from_utf8(buf).unwrap();
    // expected: just option list with default spacer 2
    let expected = "-i, --some-int=n     Some integer value. Can set to whatever number you like.\n\
        -f, --some-float=n   Some float value.\n\
        -d, --some-double=n  Some double value.\n\
        -?, --help           Shows this help message\n";
    assert_eq!(out, expected);
}

#[test]
fn test_string_option() {
    // Test XOPT_TYPE_STRING handling
    let opts = vec![
        XoptOption {
            long_arg: Some("name".to_string()),
            short_arg: 'n',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_STRING,
            arg_descrip: None,
            descrip: None,
        },
        xopt::XOPT_NULLOPTION(),
    ];
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(Some("p"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--name=alice"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none());
    assert_eq!(data.strings.get(&0).map(String::as_str), Some("alice"));
}

#[test]
fn test_long_type() {
    // XOPT_TYPE_LONG
    let opts = vec![
        XoptOption {
            long_arg: Some("count".to_string()),
            short_arg: 'c',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_LONG,
            arg_descrip: None,
            descrip: None,
        },
        xopt::XOPT_NULLOPTION(),
    ];
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(Some("p"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-c", "12345"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none());
    assert_eq!(data.ints.get(&0).copied(), Some(12345));
}

#[test]
fn test_keepfirst() {
    // KEEPFIRST: don't skip argv[0]
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("p"),
        &opts,
        XOPT_CTX_KEEPFIRST | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["file_or_arg", "-i", "5"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    let n = xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none());
    assert_eq!(n, 1);
    assert_eq!(extras.unwrap()[0], "file_or_arg");
    assert_eq!(data.ints.get(&0).copied(), Some(5));
}

#[test]
fn test_nocondense_combined_short() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("p"),
        &opts,
        XOPT_CTX_NOCONDENSE | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    // Combining short args is not allowed
    let argv = vec!["prog", "-if"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert_eq!(err.as_deref(), Some("short options cannot be combined: -if"));
}

#[test]
fn test_sloppyshorts() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(
        Some("p"),
        &opts,
        XOPT_CTX_SLOPPYSHORTS | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-i42"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none(), "err={:?}", err);
    assert_eq!(data.ints.get(&0).copied(), Some(42));
}

#[test]
fn test_single_dash_is_extra() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(Some("p"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    let n = xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert!(err.is_none());
    assert_eq!(n, 1);
    assert_eq!(extras.unwrap()[0], "-");
}

#[test]
fn test_missing_value_short() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(Some("p"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-i"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert_eq!(err.as_deref(), Some("missing option value: -i"));
}

#[test]
fn test_missing_value_long() {
    let opts = simple_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt::xopt_context(Some("p"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--some-int"];
    let mut extras: Option<Vec<String>> = None;
    let mut data = ParsedValues::new();
    xopt::xopt_parse(
        &mut ctx,
        argv.len() as i32,
        &argv,
        (&mut data) as *mut _ as *mut u8,
        &mut extras,
        &mut err,
    );
    assert_eq!(err.as_deref(), Some("missing option value: --some-int"));
}

#[test]
fn test_xopt_nulloption() {
    let n = xopt::XOPT_NULLOPTION();
    assert!(n.long_arg.is_none());
    assert_eq!(n.short_arg, '\0');
    assert_eq!(n.offset, 0);
    assert!(n.callback.is_none());
    assert_eq!(n.options, 0);
    assert!(n.arg_descrip.is_none());
    assert!(n.descrip.is_none());
}

fn main() {}
