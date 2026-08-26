// Translation of c_src/src/lib.c to safe Rust.
// Produces byte-identical output to the C version for the same inputs.

use std::io::Write;

const INT_MAX: i32 = i32::MAX;
const INT_MIN: i32 = i32::MIN;

#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: i32,
    pub value: f64,
    pub label: [u8; 20],
}

impl Default for DataBlock {
    fn default() -> Self {
        DataBlock {
            id: 0,
            value: 0.0,
            label: [0u8; 20],
        }
    }
}

/// C: int safe_double_to_int(double d)
pub fn safe_double_to_int(d: f64) -> i32 {
    if d > INT_MAX as f64 {
        INT_MAX
    } else if d < INT_MIN as f64 {
        INT_MIN
    } else if d.is_nan() {
        0
    } else {
        // C: (int)d truncation toward zero. Out-of-range was already handled.
        d as i32
    }
}

/// C: int process_with_fallthrough(int code, int base_value)
pub fn process_with_fallthrough(code: i32, base_value: i32) -> i32 {
    let mut result: i32 = base_value;

    // Replicate C's switch with fall-through using wrapping arithmetic.
    match code {
        5 => {
            result = result.wrapping_add(50);
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        4 => {
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        3 => {
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        2 => {
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        1 => {
            result = result.wrapping_add(10);
        }
        0 => {
            result = 0;
        }
        _ => {
            result = -1;
        }
    }

    result
}

/// C: void copy_data_block(DataBlock *dest, const DataBlock *src) — uses memcpy
pub fn copy_data_block(dest: &mut DataBlock, src: &DataBlock) {
    *dest = *src;
}

/// C: int handle_pointer_operations(int value)
pub fn handle_pointer_operations(value: i32) -> i32 {
    let local_value: i32 = value.wrapping_mul(2);
    // ptr = &local_value; result = *ptr + 100;
    local_value.wrapping_add(100)
}

/// Helper: turn a NUL-terminated byte buffer into a printable &str slice (lossy if invalid utf-8).
fn cstr_label(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// Format a double the way C's `printf("%.2f", v)` does for finite values, including
/// negative zero, infinities, and NaN. Uses "round half to even" matching glibc's default.
fn fmt_pct_2f(v: f64) -> String {
    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v.is_sign_negative() { "-inf".to_string() } else { "inf".to_string() };
    }
    // Rust's `{:.2}` uses "round half to even" too, matching glibc default.
    format!("{:.2}", v)
}

/// Format a double the way C's `printf("%.2e", v)` does. C uses 2 digits after decimal,
/// then 'e', sign, and at least 2 digits of exponent.
fn fmt_pct_2e(v: f64) -> String {
    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v.is_sign_negative() { "-inf".to_string() } else { "inf".to_string() };
    }
    // Rust's {:.2e} produces e.g. "1.50e15" while C produces "1.50e+15".
    let s = format!("{:.2e}", v);
    // Insert '+' if no sign present after 'e', and pad exponent to at least 2 digits.
    if let Some(epos) = s.find('e') {
        let (mantissa, exp) = s.split_at(epos);
        let exp = &exp[1..]; // strip leading 'e'
        let (sign, digits) = if let Some(stripped) = exp.strip_prefix('-') {
            ("-", stripped)
        } else if let Some(stripped) = exp.strip_prefix('+') {
            ("+", stripped)
        } else {
            ("+", exp)
        };
        let mut padded = digits.to_string();
        while padded.len() < 2 {
            padded.insert(0, '0');
        }
        format!("{}e{}{}", mantissa, sign, padded)
    } else {
        s
    }
}

/// C: int overunder(int a, int b, int c, int d)
pub fn overunder(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let mut total: i32 = 0;

    // PRINT_VAR(result_1); PRINT_VAR(result_2); — these expand to
    //   printf("result_1 = %d\n", result_1);
    let result_1 = a;
    let result_2 = b;
    // result_3 / result_4 are declared but never printed.
    let _result_3 = c;
    let _result_4 = d;

    let _ = writeln!(out, "result_1 = {}", result_1);
    let _ = writeln!(out, "result_2 = {}", result_2);

    let temp1: f64 = (a as f64) * 1.5;
    let temp2: f64 = (b as f64) * 2.7;
    let temp3: f64 = (c as f64) / 3.3;
    // C: sqrt((double)(d * d + a * a)) — d*d and a*a are int multiplications
    // (with possible C undefined-behavior overflow). Reproduce with wrapping_mul + wrapping_add,
    // matching the typical 2's-complement behavior of optimizing C compilers.
    let dd = d.wrapping_mul(d);
    let aa = a.wrapping_mul(a);
    let s = dd.wrapping_add(aa);
    let temp4: f64 = (s as f64).sqrt();

    let conv1 = safe_double_to_int(temp1);
    let conv2 = safe_double_to_int(temp2);
    let conv3 = safe_double_to_int(temp3);
    let conv4 = safe_double_to_int(temp4);

    let _ = writeln!(out, "Converted values: {}, {}, {}, {}", conv1, conv2, conv3, conv4);

    // process_with_fallthrough(a % 6, b) — C `%` truncates toward zero (matches Rust `%`)
    let switch_result = process_with_fallthrough(a % 6, b);
    let _ = writeln!(out, "Switch fall-through result: {}", switch_result);

    let mut source_block = DataBlock::default();
    source_block.id = a;
    source_block.value = temp1;
    // strncpy(source_block.label, "Source", sizeof(label)-1) then NUL-terminate.
    let label_src = b"Source";
    let n = std::cmp::min(label_src.len(), source_block.label.len() - 1);
    source_block.label[..n].copy_from_slice(&label_src[..n]);
    // strncpy pads with NULs up to size-1; explicit NUL-terminate at last index.
    for i in n..source_block.label.len() {
        source_block.label[i] = 0;
    }

    let mut dest_block = DataBlock::default();
    copy_data_block(&mut dest_block, &source_block);

    let _ = writeln!(
        out,
        "Copied block: id={}, value={}, label={}",
        dest_block.id,
        fmt_pct_2f(dest_block.value),
        cstr_label(&dest_block.label),
    );

    let ptr_result = handle_pointer_operations(c);
    let _ = writeln!(out, "Pointer operation result: {}", ptr_result);

    total = total
        .wrapping_add(conv1)
        .wrapping_add(conv2)
        .wrapping_add(conv3)
        .wrapping_add(conv4)
        .wrapping_add(switch_result)
        .wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test: f64 = 1e15;
    let safe_conv = safe_double_to_int(overflow_test);
    let _ = writeln!(out, "Overflow protected conversion: {}", safe_conv);

    let underflow_test: f64 = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    let _ = writeln!(out, "Underflow protected conversion: {}", safe_conv2);

    let array1: [i32; 5] = [a, b, c, d, a.wrapping_add(b)];
    let mut array2: [i32; 5] = [0; 5];
    array2.copy_from_slice(&array1); // memcpy equivalent

    let _ = write!(out, "Array copied via memcpy: ");
    for i in 0..5 {
        let _ = write!(out, "{} ", array2[i]);
        total = total.wrapping_add(array2[i]);
    }
    let _ = writeln!(out);

    let _ = out.flush();

    // Suppress "unused" warnings on shadowed variables that the macro implies.
    let _ = (_result_3, _result_4);

    total
}

// Note about fmt_pct_2e: not used by overunder() but kept because the lib.c
// pattern uses %.2e in nearby translation units; harmless to keep.
#[allow(dead_code)]
fn _ensure_fmt_pct_2e_used(v: f64) -> String {
    fmt_pct_2e(v)
}
