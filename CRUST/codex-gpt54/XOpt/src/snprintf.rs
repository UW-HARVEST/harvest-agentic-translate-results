pub fn rpl_vsnprintf(
    s: &mut String,
    n: usize,
    format: &str,
    args: &[&str],
) -> i32{
    let rendered = render_format(format, args);
    let rendered_len = rendered.len() as i32;

    s.clear();
    if n == 0 {
        return rendered_len;
    }

    let max_len = n.saturating_sub(1);
    if rendered.len() <= max_len {
        s.push_str(&rendered);
    } else if let Some(prefix) = rendered.get(..max_len) {
        s.push_str(prefix);
    } else {
        for ch in rendered.chars() {
            if s.len() + ch.len_utf8() > max_len {
                break;
            }
            s.push(ch);
        }
    }

    rendered_len
}

pub fn fmtstr(
    s: &mut String,
    _size: usize,
    value: &str,
    width: usize,
    precision: usize,
    flags: i32,
    ){
    let truncated = if precision > 0 && value.chars().count() > precision {
        value.chars().take(precision).collect::<String>()
    } else {
        value.to_string()
    };

    let pad = width.saturating_sub(truncated.chars().count());
    let left_justify = flags & 1 != 0;

    if !left_justify {
        s.push_str(&" ".repeat(pad));
    }
    s.push_str(&truncated);
    if left_justify {
        s.push_str(&" ".repeat(pad));
    }
}

pub fn fmtint(
    s: &mut String,
    _size: usize,
    value: i32,
    width: usize,
    precision: usize,
    flags: i32,
    ){
    let mut rendered = if precision > 0 {
        format!("{:01$}", value, precision)
    } else {
        value.to_string()
    };

    if flags & 16 != 0 && rendered.len() < width {
        rendered = format!("{:0>1$}", rendered, width);
    }

    if rendered.len() < width {
        let pad = " ".repeat(width - rendered.len());
        if flags & 1 != 0 {
            rendered.push_str(&pad);
        } else {
            rendered = format!("{}{}", pad, rendered);
        }
    }

    s.push_str(&rendered);
}

pub fn fmtflt(
    s: &mut String,
    _size: usize,
    value: f64,
    width: usize,
    precision: usize,
    flags: i32,
    ){
    let precision = if precision == 0 { 6 } else { precision };
    let mut rendered = format!("{value:.precision$}");
    if rendered.len() < width {
        let pad = " ".repeat(width - rendered.len());
        if flags & 1 != 0 {
            rendered.push_str(&pad);
        } else {
            rendered = format!("{}{}", pad, rendered);
        }
    }
    s.push_str(&rendered);
}

pub fn printsep(
    s: &mut String,
    _size:usize){
    s.push(',');
}

pub fn getnumsep(digits: i32) -> i32{
    if digits <= 0 {
        0
    } else {
        (digits - 1) / 3
    }
}

pub fn getexponent(value: f64) -> i32{
    if value == 0.0 {
        0
    } else {
        value.abs().log10().floor() as i32
    }
}

pub fn convert(
    value: usize, 
    buf: &mut String,
    base: usize,
    caps:usize){
    let rendered = match (base, caps != 0) {
        (8, _) => format!("{value:o}"),
        (16, true) => format!("{value:X}"),
        (16, false) => format!("{value:x}"),
        (_, _) => value.to_string(),
    };
    buf.clear();
    buf.push_str(&rendered);
}

pub fn cast(value: f64)->i32{
    if value.is_nan() {
        0
    } else if value > i32::MAX as f64 {
        i32::MAX
    } else if value < i32::MIN as f64 {
        i32::MIN
    } else {
        value as i32
    }
}

pub fn mypow10(exponent: i32)->f64{
    10_f64.powi(exponent)
}

pub fn rpl_vasprintf(
    s: Vec<String>,
    format: &str,
    args: &[&str],
    ) -> i32{
    let mut sink = s.join("");
    rpl_vsnprintf(&mut sink, usize::MAX, format, args)
}

pub fn rpl_asprintf(
    s: &mut String,
    format: &str,
    args: &[&str],
    ) -> i32{
    rpl_vsnprintf(s, usize::MAX, format, args)
}

pub fn main(){
}

fn render_format(format: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut chars = format.chars().peekable();
    let mut arg_idx = 0usize;

    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }

        match chars.peek().copied() {
            Some('%') => {
                chars.next();
                out.push('%');
            }
            Some(_) => {
                while let Some(spec) = chars.peek().copied() {
                    if spec.is_ascii_alphabetic() || spec == '%' {
                        let spec = chars.next().unwrap();
                        if spec != '%' {
                            if let Some(arg) = args.get(arg_idx) {
                                out.push_str(arg);
                                arg_idx += 1;
                            }
                        } else {
                            out.push('%');
                        }
                        break;
                    }
                    chars.next();
                }
            }
            None => out.push('%'),
        }
    }

    out
}
