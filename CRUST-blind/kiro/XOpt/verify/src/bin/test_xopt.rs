use XOpt::xopt;

macro_rules! offset_of_field {
    ($type:ty, $field:ident) => {
        unsafe {
            let base = std::ptr::null::<$type>();
            let field_ptr = std::ptr::addr_of!((*base).$field);
            (field_ptr as *const u8).offset_from(base as *const u8) as usize
        }
    };
}

// ---- Constants tests ----

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

// ---- XOPT_NULLOPTION tests ----

#[test]
fn test_null_option() {
    let opt = xopt::XOPT_NULLOPTION();
    assert!(opt.long_arg.is_none());
    assert_eq!(opt.short_arg, '\0');
    assert_eq!(opt.offset, 0);
    assert!(opt.callback.is_none());
    assert_eq!(opt.options, 0);
    assert!(opt.arg_descrip.is_none());
    assert!(opt.descrip.is_none());
}

// ---- xopt_context tests ----

#[test]
fn test_xopt_context_basic() {
    let options = vec![xopt::XOPT_NULLOPTION()];
    let mut err: Option<String> = None;
    let ctx = xopt::xopt_context(Some("test"), &options, 0, &mut err);
    assert!(ctx.is_some());
    assert!(err.is_none());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.name, Some("test".to_string()));
    assert_eq!(ctx.flags, 0);
    assert!(!ctx.doubledash);
}

#[test]
fn test_xopt_context_with_flags() {
    let options = vec![xopt::XOPT_NULLOPTION()];
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err);
    assert!(ctx.is_some());
    assert!(err.is_none());
    assert_eq!(ctx.unwrap().flags, flags);
}

// ---- Helper to build test options matching C simple-test ----

#[repr(C)]
struct SimpleConfig {
    some_int: i32,
    some_float: f32,
    some_double: f64,
    help: u8,
}

fn make_test_options() -> Vec<xopt::XoptOption> {
    vec![
        xopt::XoptOption {
            long_arg: Some("some-int".to_string()),
            short_arg: 'i',
            offset: offset_of_field!(SimpleConfig, some_int),
            callback: None,
            options: xopt::XOPT_TYPE_INT,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some integer value. Can set to whatever number you like.".to_string()),
        },
        xopt::XoptOption {
            long_arg: Some("some-float".to_string()),
            short_arg: 'f',
            offset: offset_of_field!(SimpleConfig, some_float),
            callback: None,
            options: xopt::XOPT_TYPE_FLOAT,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some float value.".to_string()),
        },
        xopt::XoptOption {
            long_arg: Some("some-double".to_string()),
            short_arg: 'd',
            offset: offset_of_field!(SimpleConfig, some_double),
            callback: None,
            options: xopt::XOPT_TYPE_DOUBLE,
            arg_descrip: Some("n".to_string()),
            descrip: Some("Some double value.".to_string()),
        },
        xopt::XoptOption {
            long_arg: Some("help".to_string()),
            short_arg: '?',
            offset: offset_of_field!(SimpleConfig, help),
            callback: None,
            options: xopt::XOPT_TYPE_BOOL,
            arg_descrip: None,
            descrip: Some("Shows this help message".to_string()),
        },
        xopt::XOPT_NULLOPTION(),
    ]
}

// ---- xopt_parse tests ----

#[test]
fn test_parse_long_int() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--some-int=42"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(config.some_int, 42);
    assert_eq!(count, 0);
    assert!(extras.is_some());
}

#[test]
fn test_parse_short_int() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "-i", "42"];
    let count = xopt::xopt_parse(&mut ctx, 3, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(config.some_int, 42);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_float() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--some-float=3.14"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert!((config.some_float - 3.14f32).abs() < 0.001);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_double() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--some-double=2.718"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert!((config.some_double - 2.718).abs() < 0.001);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_bool() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--help"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(config.help, 1);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_bool_short() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "-?"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(config.help, 1);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_extras() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--some-int=42", "extra1", "extra2"];
    let count = xopt::xopt_parse(&mut ctx, 4, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 2);
    let extras = extras.unwrap();
    assert_eq!(extras.len(), 2);
    assert_eq!(extras[0], "extra1");
    assert_eq!(extras[1], "extra2");
}

#[test]
fn test_parse_no_args() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test"];
    let count = xopt::xopt_parse(&mut ctx, 1, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 0);
    assert_eq!(config.some_int, 0);
}

// ---- Double dash ----

#[test]
fn test_parse_double_dash() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--", "--some-int=5"];
    let count = xopt::xopt_parse(&mut ctx, 3, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(config.some_int, 0); // not parsed as option
    assert_eq!(count, 1);
    let extras = extras.unwrap();
    assert_eq!(extras[0], "--some-int=5");
}

// ---- Singular dash treated as extra ----

#[test]
fn test_parse_singular_dash() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "-", "extra"];
    let count = xopt::xopt_parse(&mut ctx, 3, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(count, 2);
    let extras = extras.unwrap();
    assert_eq!(extras[0], "-");
    assert_eq!(extras[1], "extra");
}

// ---- Error cases ----

#[test]
fn test_parse_invalid_option_strict() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--invalid"];
    xopt::xopt_parse(&mut ctx, 2, &argv, data, &mut extras, &mut err);

    assert!(err.is_some());
    assert!(err.as_ref().unwrap().contains("invalid option"));
}

#[test]
fn test_parse_invalid_number() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--some-int=abc"];
    xopt::xopt_parse(&mut ctx, 2, &argv, data, &mut extras, &mut err);

    assert!(err.is_some());
    assert!(err.as_ref().unwrap().contains("valid number"));
}

#[test]
fn test_parse_posixmeharder_violation() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "extra1", "--some-int=5"];
    xopt::xopt_parse(&mut ctx, 3, &argv, data, &mut extras, &mut err);

    assert!(err.is_some());
    assert!(err.as_ref().unwrap().contains("options cannot be specified after arguments"));
}

// ---- KEEPFIRST flag ----

#[test]
fn test_parse_keepfirst() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_KEEPFIRST;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    // With KEEPFIRST, argv[0] is also parsed (as an extra since "test" is not an option)
    let argv = ["extra0", "--some-int=42"];
    let count = xopt::xopt_parse(&mut ctx, 2, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(config.some_int, 42);
    // "extra0" is treated as an extra
    assert_eq!(count, 1);
    let extras = extras.unwrap();
    assert_eq!(extras[0], "extra0");
}

// ---- Hex/Octal int parsing ----

#[test]
fn test_parse_hex_int() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "-i", "0x1A"];
    let count = xopt::xopt_parse(&mut ctx, 3, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(config.some_int, 26);
    assert_eq!(count, 0);
}

#[test]
fn test_parse_octal_int() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "-i", "010"];
    let count = xopt::xopt_parse(&mut ctx, 3, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(config.some_int, 8);
    assert_eq!(count, 0);
}

// ---- xopt_autohelp tests ----

#[test]
fn test_autohelp_output() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let autohelp = xopt::XoptAutohelpOptions {
        usage: Some("usage: simple-test [options] [extras...]".to_string()),
        prefix: Some("A simple demonstration of the XOpt options parser library.".to_string()),
        suffix: Some("End argument list.".to_string()),
        spacer: 10,
    };

    let mut output = Vec::new();
    xopt::xopt_autohelp(&mut ctx, &mut output, Some(&autohelp), &mut err);
    assert!(err.is_none());

    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains("usage: simple-test [options] [extras...]"));
    assert!(output_str.contains("A simple demonstration of the XOpt options parser library."));
    assert!(output_str.contains("-i, --some-int=n"));
    assert!(output_str.contains("Some integer value."));
    assert!(output_str.contains("-?, --help"));
    assert!(output_str.contains("Shows this help message"));
    assert!(output_str.contains("End argument list."));
}

#[test]
fn test_autohelp_no_options() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = 0;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut output = Vec::new();
    xopt::xopt_autohelp(&mut ctx, &mut output, None, &mut err);
    assert!(err.is_none());

    let output_str = String::from_utf8(output).unwrap();
    // Should still print options even without autohelp options
    assert!(output_str.contains("--some-int"));
}

// ---- Multiple options ----

#[test]
fn test_parse_multiple_options() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--some-int=42", "--some-float=3.14", "--some-double=2.718", "extra1", "extra2"];
    let count = xopt::xopt_parse(&mut ctx, 6, &argv, data, &mut extras, &mut err);

    assert!(err.is_none());
    assert_eq!(config.some_int, 42);
    assert!((config.some_float - 3.14f32).abs() < 0.001);
    assert!((config.some_double - 2.718).abs() < 0.001);
    assert_eq!(count, 2);
}

// ---- Missing required value ----

#[test]
fn test_parse_missing_long_value() {
    let options = make_test_options();
    let mut err: Option<String> = None;
    let flags = xopt::XOPT_CTX_POSIXMEHARDER | xopt::XOPT_CTX_STRICT;
    let mut ctx = xopt::xopt_context(Some("test"), &options, flags, &mut err).unwrap();

    let mut config = SimpleConfig {
        some_int: 0,
        some_float: 0.0,
        some_double: 0.0,
        help: 0,
    };
    let data = &mut config as *mut SimpleConfig as *mut u8;
    let mut extras: Option<Vec<String>> = None;
    let argv = ["test", "--some-int"];
    xopt::xopt_parse(&mut ctx, 2, &argv, data, &mut extras, &mut err);

    assert!(err.is_some());
    assert!(err.as_ref().unwrap().contains("missing option value"));
}

fn main() {}
