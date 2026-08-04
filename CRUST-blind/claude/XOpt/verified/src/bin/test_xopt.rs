use XOpt::xopt::{
    XoptAutohelpOptions, XoptOption, XOPT_CTX_KEEPFIRST, XOPT_CTX_NOCONDENSE,
    XOPT_CTX_POSIXMEHARDER, XOPT_CTX_SLOPPYSHORTS, XOPT_CTX_STRICT, XOPT_NULLOPTION,
    XOPT_OPTIONAL, XOPT_TYPE_BOOL, XOPT_TYPE_DOUBLE, XOPT_TYPE_FLOAT, XOPT_TYPE_INT,
    XOPT_TYPE_LONG, XOPT_TYPE_STRING, xopt_autohelp, xopt_context, xopt_parse,
};

fn make_options() -> Vec<XoptOption> {
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
            offset: 0,
            callback: None,
            options: XOPT_TYPE_FLOAT,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some float value.".to_string()),
        },
        XoptOption {
            long_arg: Some("some-double".to_string()),
            short_arg: 'd',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_DOUBLE,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some double value.".to_string()),
        },
        XoptOption {
            long_arg: Some("help".to_string()),
            short_arg: '?',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: Some("Shows this help message".to_string()),
        },
        XOPT_NULLOPTION(),
    ]
}

#[test]
fn test_constants() {
    assert_eq!(XOPT_TYPE_STRING, 0x1);
    assert_eq!(XOPT_TYPE_INT, 0x2);
    assert_eq!(XOPT_TYPE_LONG, 0x4);
    assert_eq!(XOPT_TYPE_FLOAT, 0x8);
    assert_eq!(XOPT_TYPE_DOUBLE, 0x10);
    assert_eq!(XOPT_TYPE_BOOL, 0x20);
    assert_eq!(XOPT_OPTIONAL, 0x40);
    assert_eq!(XOPT_CTX_KEEPFIRST, 0x1);
    assert_eq!(XOPT_CTX_POSIXMEHARDER, 0x2);
    assert_eq!(XOPT_CTX_NOCONDENSE, 0x4);
    assert_eq!(XOPT_CTX_SLOPPYSHORTS, 0xC); // 0x8 | 0x4
    assert_eq!(XOPT_CTX_STRICT, 0x10);
}

#[test]
fn test_xopt_nulloption() {
    let term = XOPT_NULLOPTION();
    assert!(term.long_arg.is_none());
    assert_eq!(term.short_arg, '\0');
    assert_eq!(term.offset, 0);
    assert!(term.callback.is_none());
    assert_eq!(term.options, 0);
    assert!(term.arg_descrip.is_none());
    assert!(term.descrip.is_none());
}

#[test]
fn test_xopt_context_basic() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let ctx = xopt_context(Some("xopt-test"), &opts, XOPT_CTX_STRICT, &mut err);
    assert!(err.is_none());
    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.name, Some("xopt-test".to_string()));
    assert_eq!(ctx.flags, XOPT_CTX_STRICT);
    assert_eq!(ctx.doubledash, false);
    assert_eq!(ctx.options.len(), opts.len());
}

#[test]
fn test_xopt_context_no_name() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let ctx = xopt_context(None, &opts, 0, &mut err);
    assert!(err.is_none());
    assert!(ctx.is_some());
    assert_eq!(ctx.unwrap().name, None);
}

#[test]
fn test_parse_long_option_with_value() {
    // C: --some-int=42 --some-float=3.14 --some-double=2.71828 extra1 extra2
    // Output:
    //   someInt: 42, someFloat: 3.14, someDouble: 2.71828, extra count: 2 (extra1, extra2)
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    assert!(err.is_none());

    let argv = vec!["prog", "--some-int=42", "--some-float=3.14", "--some-double=2.71828", "extra1", "extra2"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none(), "err: {:?}", err);
    assert_eq!(count, 2);
    let extras = extras.unwrap();
    assert_eq!(extras.len(), 2);
    assert_eq!(extras[0], "extra1");
    assert_eq!(extras[1], "extra2");
}

#[test]
fn test_parse_short_options() {
    // C: -i 42 -f 3.14 -d 2.71828 extra1 extra2 -> succeeds
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-i", "42", "-f", "3.14", "-d", "2.71828", "extra1", "extra2"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none(), "err: {:?}", err);
    assert_eq!(count, 2);
    let extras = extras.unwrap();
    assert_eq!(extras.len(), 2);
    assert_eq!(extras[0], "extra1");
    assert_eq!(extras[1], "extra2");
}

#[test]
fn test_parse_double_dash() {
    // C: -- --not-an-option produces 1 extra "--not-an-option"
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--", "--not-an-option"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none(), "err: {:?}", err);
    assert_eq!(count, 1);
    let extras = extras.unwrap();
    assert_eq!(extras[0], "--not-an-option");
}

#[test]
fn test_parse_invalid_long_arg() {
    // C: --bad-arg => Error: invalid option: --bad-arg
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--bad-arg"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "invalid option: --bad-arg");
    assert_eq!(count, 0);
    assert!(extras.is_none());
}

#[test]
fn test_parse_invalid_short_arg() {
    // C: -x => Error: invalid option: -x
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-x"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "invalid option: -x");
}

#[test]
fn test_parse_posix_violation() {
    // C: extra1 --some-int=42 with POSIXMEHARDER => "options cannot be specified after arguments: --some-int=42"
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "extra1", "--some-int=42"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "options cannot be specified after arguments: --some-int=42");
}

#[test]
fn test_parse_invalid_number_long() {
    // C: --some-int=abc => "value isn't a valid number: --some-int=abc"
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--some-int=abc"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "value isn't a valid number: --some-int=abc");
}

#[test]
fn test_parse_bool_with_value() {
    // C: --help=foo => "option doesn't take a value: --help=foo"
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--help=foo"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "option doesn't take a value: --help=foo");
}

#[test]
fn test_parse_bool_no_value() {
    // C: --help (or "-?") => succeeds with no extras
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--help"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none(), "err: {:?}", err);
    assert_eq!(count, 0);
    assert_eq!(extras.unwrap().len(), 0);
}

#[test]
fn test_parse_short_bool() {
    // C: "-?" => succeeds
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-?"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none(), "err: {:?}", err);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_missing_long_value() {
    // C: --some-int (no value) => "missing option value: --some-int"
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--some-int"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "missing option value: --some-int");
}

#[test]
fn test_parse_missing_short_value() {
    // C: -i (no following value) => "missing option value: -i"
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-i"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "missing option value: -i");
}

#[test]
fn test_parse_singular_dash_as_extra() {
    // C: '-' is treated as an extra.
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none());
    assert_eq!(count, 1);
    let extras = extras.unwrap();
    assert_eq!(extras[0], "-");
}

#[test]
fn test_parse_extras_only() {
    // No options, just extras
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "a", "b", "c"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none());
    assert_eq!(count, 3);
    let extras = extras.unwrap();
    assert_eq!(extras, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn test_parse_keepfirst() {
    // KEEPFIRST: argv[0] is parsed as well
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_KEEPFIRST, &mut err).unwrap();
    let argv = vec!["a", "b", "c"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none());
    assert_eq!(count, 3);
}

#[test]
fn test_parse_combined_short_bools() {
    // Combined short bools
    let opts = vec![
        XoptOption {
            long_arg: None,
            short_arg: 'a',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: None,
        },
        XoptOption {
            long_arg: None,
            short_arg: 'b',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: None,
        },
        XoptOption {
            long_arg: None,
            short_arg: 'c',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: None,
        },
        XOPT_NULLOPTION(),
    ];
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-abc"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none(), "err: {:?}", err);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_nocondense_combined_fails() {
    // With NOCONDENSE flag, "-abc" is treated as an error
    let opts = vec![
        XoptOption {
            long_arg: None,
            short_arg: 'a',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: None,
        },
        XoptOption {
            long_arg: None,
            short_arg: 'b',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: None,
        },
        XOPT_NULLOPTION(),
    ];
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_STRICT | XOPT_CTX_NOCONDENSE,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-ab"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "short options cannot be combined: -ab");
}

#[test]
fn test_parse_short_value_combined_not_last() {
    // Short option requiring value combined with another bool but not last.
    let opts = vec![
        XoptOption {
            long_arg: None,
            short_arg: 'i',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_INT,
            arg_descrip: None,
            descrip: None,
        },
        XoptOption {
            long_arg: None,
            short_arg: 'a',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: None,
        },
        XOPT_NULLOPTION(),
    ];
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-ia"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "combined short option requiring value is not last: -i");
}

#[test]
fn test_parse_sloppy_shorts() {
    // SLOPPYSHORTS allows -i42 (value directly after short char)
    let opts = vec![
        XoptOption {
            long_arg: Some("some-int".to_string()),
            short_arg: 'i',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_INT,
            arg_descrip: None,
            descrip: None,
        },
        XOPT_NULLOPTION(),
    ];
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_STRICT | XOPT_CTX_SLOPPYSHORTS,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-i42"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none(), "err: {:?}", err);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_sloppy_shorts_invalid_int() {
    // SLOPPYSHORTS with a value that isn't a valid int
    let opts = vec![
        XoptOption {
            long_arg: Some("some-int".to_string()),
            short_arg: 'i',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_INT,
            arg_descrip: None,
            descrip: None,
        },
        XOPT_NULLOPTION(),
    ];
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_STRICT | XOPT_CTX_SLOPPYSHORTS,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-iabc"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    // C error format: "value isn't a valid number: -i abc"
    assert_eq!(err.unwrap(), "value isn't a valid number: -i abc");
}

#[test]
fn test_parse_sloppy_shorts_bool_value() {
    // SLOPPYSHORTS: bool with value -> error
    let opts = vec![
        XoptOption {
            long_arg: None,
            short_arg: 'b',
            offset: 0,
            callback: None,
            options: XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: None,
        },
        XOPT_NULLOPTION(),
    ];
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(
        Some("test"),
        &opts,
        XOPT_CTX_STRICT | XOPT_CTX_SLOPPYSHORTS,
        &mut err,
    )
    .unwrap();
    let argv = vec!["prog", "-bX"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "option doesn't take a value: -b");
}

#[test]
fn test_parse_invalid_float_long() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--some-float=notanumber"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "value isn't a valid number: --some-float=notanumber");
}

#[test]
fn test_parse_invalid_float_short() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "-f", "notanumber"];
    let mut extras: Option<Vec<String>> = None;
    let _ = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_some());
    assert_eq!(err.unwrap(), "value isn't a valid number: -f notanumber");
}

#[test]
fn test_parse_double_dash_with_normal_extras() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "extra1", "--", "extra2", "-x"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none());
    assert_eq!(count, 3);
    let extras = extras.unwrap();
    assert_eq!(extras[0], "extra1");
    assert_eq!(extras[1], "extra2");
    assert_eq!(extras[2], "-x");
}

#[test]
fn test_parse_long_arg_int_with_hex() {
    // Value "0xff" should be recognized as a valid int (matches C strtol w/ base 0)
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--some-int=0xff"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none(), "err: {:?}", err);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_long_arg_int_with_octal() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, XOPT_CTX_STRICT, &mut err).unwrap();
    let argv = vec!["prog", "--some-int=0755"];
    let mut extras: Option<Vec<String>> = None;
    let count = xopt_parse(&mut ctx, argv.len() as i32, &argv, std::ptr::null_mut(), &mut extras, &mut err);
    assert!(err.is_none(), "err: {:?}", err);
    assert_eq!(count, 0);
}

#[test]
fn test_xopt_autohelp_basic() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, 0, &mut err).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    let auto_opts = XoptAutohelpOptions {
        usage: Some("usage: simple-test [options] [extras...]".to_string()),
        prefix: Some("A simple demonstration of the XOpt options parser library.".to_string()),
        suffix: Some("End argument list.".to_string()),
        spacer: 10,
    };
    xopt_autohelp(&mut ctx, &mut buf, Some(&auto_opts), &mut err);
    assert!(err.is_none());
    let text = String::from_utf8(buf).unwrap();
    // The C output starts with the usage line:
    let expected_start = "usage: simple-test [options] [extras...]\n";
    assert!(text.starts_with(expected_start), "actual: {:?}", text);
    assert!(text.contains("\nA simple demonstration of the XOpt options parser library.\n\n"));
    assert!(text.contains("-i, --some-int=n"));
    assert!(text.contains("-f, --some-float=n"));
    assert!(text.contains("-d, --some-double=n"));
    assert!(text.contains("-?, --help"));
    assert!(text.contains("Some integer value. Can set to whatever number you like."));
    assert!(text.contains("Shows this help message"));
    assert!(text.ends_with("End argument list.\n"));
}

#[test]
fn test_xopt_autohelp_no_options() {
    let opts = make_options();
    let mut err: Option<String> = None;
    let mut ctx = xopt_context(Some("test"), &opts, 0, &mut err).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    xopt_autohelp(&mut ctx, &mut buf, None, &mut err);
    assert!(err.is_none());
    let text = String::from_utf8(buf).unwrap();
    // Without auto_opts (None), only the option list is printed - no usage/prefix/suffix.
    assert!(text.contains("-i, --some-int=n"));
    assert!(text.contains("-f, --some-float=n"));
    assert!(!text.contains("usage:"));
}

fn main() {}
