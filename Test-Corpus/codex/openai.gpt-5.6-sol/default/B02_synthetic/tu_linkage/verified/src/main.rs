use std::env;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

#[derive(Default)]
struct Vm {
    stack: Vec<i32>,
    trace: Vec<i32>,
    steps: i32,
}

impl Vm {
    fn pop(&mut self) -> Option<i32> {
        self.stack.pop()
    }

    fn peek(&self, default: i32) -> i32 {
        self.stack.last().copied().unwrap_or(default)
    }

    fn mark(&mut self, value: i32) {
        self.trace.push(value);
    }
}

#[derive(Default)]
struct Runtime {
    state_a: i32,
    flipflop: i32,
}

fn target_external(code: i32) -> i32 {
    if code < 0 {
        return 7;
    }
    let m = code % 10;
    if m == 0 {
        0
    } else if m <= 3 {
        1
    } else if m <= 6 {
        2
    } else if m == 7 {
        3
    } else {
        4
    }
}

fn target_a(code: i32, runtime: &mut Runtime) -> i32 {
    if code < 0 {
        return if runtime.state_a & 1 != 0 { 6 } else { 5 };
    }
    runtime.state_a ^= code.wrapping_shl(1);
    match ((code >> 2) ^ runtime.state_a) & 7 {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 1,
        4 => 3,
        5 | 6 => 5,
        _ => 7,
    }
}

fn call_a_once(x: i32, runtime: &mut Runtime) -> i32 {
    let a = target_a(x, runtime);
    let b = target_a(a.wrapping_sub(5), runtime);
    let c = target_a(b ^ 3, runtime);
    let d = target_a((b ^ 0x55).wrapping_add(7), runtime);
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

fn process_a_stream(values: &[i32], runtime: &mut Runtime) -> i32 {
    let mut acc = 0usize;
    for &value in values {
        for j in 0..3u32 {
            let t = target_a(value.wrapping_add(j as i32), runtime);
            if t & 1 == 0 {
                acc = acc.wrapping_add(t as usize);
                continue;
            }
            acc ^= t.wrapping_shl(j) as usize;
            if t == 5 {
                break;
            }
        }
    }

    if acc > i32::MAX as usize {
        acc = i32::MAX as usize;
    }
    if acc < (-0x80000000i64) as usize {
        acc = (-0x80000000i64) as usize;
    }
    acc as i32
}

fn target_b(code: i32, runtime: &mut Runtime) -> i32 {
    runtime.flipflop ^= 1;
    if code < 0 {
        return if runtime.flipflop != 0 { 2 } else { 6 };
    }
    let z = (code ^ if runtime.flipflop != 0 { 0x7f } else { 0x1f }) % 8;
    match z {
        0 | 7 => 4,
        1 | 2 => 3,
        3 => 1,
        4 => 0,
        5 => 5,
        _ => 7,
    }
}

fn call_b_once(x: i32, runtime: &mut Runtime) -> i32 {
    let a = target_b(x, runtime);
    let b = target_b(a.wrapping_add(9), runtime);
    let c = target_b(
        (a.wrapping_add(9) ^ 0x2222).wrapping_sub(17),
        runtime,
    );
    let d = target_b(c ^ x, runtime);
    a.wrapping_shl(1)
        ^ b.wrapping_shl(2)
        ^ c.wrapping_shl(3)
        ^ d.wrapping_shl(4)
}

fn process_b_stream(values: &[i32], runtime: &mut Runtime) -> i32 {
    let mut acc = 1i32;
    for &value in values {
        let mut iter = 0i32;
        loop {
            iter = iter.wrapping_add(1);
            if iter > 4 {
                break;
            }
            let t = target_b(value.wrapping_sub(iter), runtime);
            if t == 6 {
                acc = acc.wrapping_sub(t);
                break;
            }
            if t == 3 {
                continue;
            }
            acc = acc.wrapping_mul(3) ^ t;
        }
    }
    acc
}

fn classify(implementation: i32, x: i32, runtime: &mut Runtime) -> i32 {
    match implementation {
        0 => call_a_once(x, runtime),
        1 => call_b_once(x.wrapping_add(1), runtime),
        _ => target_external(target_external(x.wrapping_add(1))),
    }
}

fn process_stream(implementation: i32, values: &[i32], runtime: &mut Runtime) -> i32 {
    match implementation {
        0 => process_a_stream(values, runtime),
        1 => process_b_stream(values, runtime),
        _ => {
            let mut acc = 0i32;
            for &value in values {
                let t = target_external(value);
                if t & 1 == 0 {
                    acc = acc.wrapping_add(t.wrapping_mul(2));
                } else {
                    acc ^= t.wrapping_add(7);
                }
            }
            acc
        }
    }
}

fn run_engine(
    implementation: i32,
    code: &[i32],
    vm: &mut Vm,
    runtime: &mut Runtime,
) -> i32 {
    let mut ip = 0usize;
    while ip < code.len() {
        let op = code[ip];
        ip += 1;
        vm.steps = vm.steps.wrapping_add(1);

        match op {
            0 => {
                if ip >= code.len() {
                    return 1;
                }
                let immediate = code[ip];
                ip += 1;
                vm.stack.push(immediate);
                vm.mark(0);
            }
            1 => {
                let Some(b) = vm.pop() else {
                    return 2;
                };
                let Some(a) = vm.pop() else {
                    return 2;
                };
                vm.stack.push(a.wrapping_add(b));
                vm.mark(1);
            }
            2 => {
                let Some(b) = vm.pop() else {
                    return 3;
                };
                let Some(a) = vm.pop() else {
                    return 3;
                };
                vm.stack.push(a.wrapping_mul(b));
                vm.mark(2);
            }
            3 => {
                let value = vm.peek(0);
                vm.stack.push(value);
                vm.mark(3);
            }
            4 => {
                if vm.pop().is_none() {
                    return 4;
                }
                vm.mark(4);
            }
            5 => {
                let x = vm.peek(0);
                let bucket = classify(implementation, x, runtime);
                vm.stack.push(bucket);
                match bucket {
                    0 => vm.mark(5),
                    1 => vm.mark(6),
                    2 => vm.mark(7),
                    3 | 4 => vm.mark(8),
                    _ => vm.mark(9),
                }
            }
            6 => {
                if ip >= code.len() {
                    return 5;
                }
                let k = code[ip];
                ip += 1;
                let Some(condition) = vm.pop() else {
                    return 6;
                };
                if condition != 0 {
                    let jump = k as usize;
                    if jump > code.len() - ip {
                        return 7;
                    }
                    ip += jump;
                    vm.mark(10);
                } else {
                    vm.mark(11);
                }
            }
            7 => {
                if ip >= code.len() {
                    return 8;
                }
                let times = code[ip];
                ip += 1;
                if ip >= code.len() {
                    return 9;
                }
                let saved_ip = ip;
                if times > 0 {
                    for _ in 0..times {
                        let rc = run_engine(
                            implementation,
                            &code[saved_ip..saved_ip + 1],
                            vm,
                            runtime,
                        );
                        if rc != 0 {
                            vm.mark(12);
                            break;
                        }
                    }
                }
                ip = saved_ip + 1;
            }
            8 => {
                let x = vm.peek(0);
                let y = classify(implementation, x, runtime);
                vm.stack.push(y);
                vm.mark(13);
            }
            9 => {
                if ip >= code.len() {
                    return 10;
                }
                let m = code[ip];
                ip += 1;
                if m < 0 || m as usize > vm.stack.len() {
                    return 11;
                }
                let mut temporary = vec![0i32; m as usize];
                for i in (0..m as usize).rev() {
                    if let Some(value) = vm.pop() {
                        temporary[i] = value;
                    }
                }
                for i in (0..m as usize).rev() {
                    if let Some(value) = vm.pop() {
                        temporary[i] = value;
                    }
                }
                let result = process_stream(implementation, &temporary, runtime);
                vm.stack.push(result);
                vm.mark(14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}

fn parse_c_long(token: &[u8]) -> Option<i32> {
    let mut pos = 0usize;
    while pos < token.len()
        && matches!(token[pos], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    {
        pos += 1;
    }

    let mut negative = false;
    if pos < token.len() && (token[pos] == b'+' || token[pos] == b'-') {
        negative = token[pos] == b'-';
        pos += 1;
    }
    let digits_start = pos;
    let mut magnitude = 0u128;
    while pos < token.len() && token[pos].is_ascii_digit() {
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add((token[pos] - b'0') as u128);
        pos += 1;
    }

    if pos == digits_start {
        return if token.is_empty() { Some(0) } else { None };
    }
    if pos != token.len() {
        return None;
    }

    let value = if negative {
        const LONG_MIN_MAGNITUDE: u128 = 1u128 << 63;
        if magnitude >= LONG_MIN_MAGNITUDE {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else if magnitude > i64::MAX as u128 {
        i64::MAX
    } else {
        magnitude as i64
    };
    Some(value as i32)
}

fn append_stdin_code(code: &mut Vec<i32>) {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let mut offset = 0usize;
    while offset < input.len() {
        let limit = (offset + 4095).min(input.len());
        let end = input[offset..limit]
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(limit, |relative| offset + relative + 1);
        let chunk = &input[offset..end];
        let visible_len = chunk.iter().position(|&byte| byte == 0).unwrap_or(chunk.len());

        let mut pos = 0usize;
        while pos < visible_len {
            let token_end = chunk[pos..visible_len]
                .iter()
                .position(|&byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))
                .map_or(visible_len, |relative| pos + relative);
            if token_end > pos {
                if let Some(value) = parse_c_long(&chunk[pos..token_end]) {
                    code.push(value);
                }
            }
            pos = if token_end < visible_len {
                token_end + 1
            } else {
                token_end
            };
        }
        offset = end;
    }
}

fn append_vm_output(output: &mut Vec<u8>, label: &[u8], vm: &Vm) {
    output.extend_from_slice(label);
    write!(
        output,
        "STACK_TOP={} STEPS={} TRACE=",
        vm.peek(-777),
        vm.steps
    )
    .unwrap();
    let alphabet = b"abcdefghijklmnopqrstuvwxyz";
    for &value in &vm.trace {
        output.push(alphabet[(value & 25) as usize]);
    }
    output.push(b'\n');
}

fn real_main() -> u8 {
    let arguments: Vec<_> = env::args_os().collect();
    let program_name = arguments
        .first()
        .map(|value| value.as_os_str().as_bytes())
        .unwrap_or_default();
    let mut code = Vec::new();
    let mut use_stdin = false;
    let mut stderr = io::stderr().lock();

    for argument in arguments.iter().skip(1) {
        let bytes = argument.as_os_str().as_bytes();
        if bytes == b"--help" {
            let _ = stderr.write_all(b"Usage: ");
            let _ = stderr.write_all(program_name);
            let _ = stderr.write_all(
                b" [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n",
            );
            return 0;
        } else if bytes == b"--stdin" {
            use_stdin = true;
        } else if let Some(value) = parse_c_long(bytes) {
            code.push(value);
        } else {
            let _ = stderr.write_all(b"skip '");
            let _ = stderr.write_all(bytes);
            let _ = stderr.write_all(b"'\n");
        }
    }

    if use_stdin {
        append_stdin_code(&mut code);
    }
    if code.is_empty() {
        let _ = stderr.write_all(b"no program\n");
        return 2;
    }
    drop(stderr);

    let mut runtime = Runtime::default();
    let mut vm_a = Vm::default();
    let mut vm_b = Vm::default();
    let mut vm_external = Vm::default();
    let rc_a = run_engine(0, &code, &mut vm_a, &mut runtime);
    let rc_b = run_engine(1, &code, &mut vm_b, &mut runtime);
    let rc_external = run_engine(2, &code, &mut vm_external, &mut runtime);

    let mut output = Vec::new();
    writeln!(
        output,
        "RC:A={} B={} EXT={}",
        rc_a, rc_b, rc_external
    )
    .unwrap();
    append_vm_output(&mut output, b"A:", &vm_a);
    append_vm_output(&mut output, b"B:", &vm_b);
    append_vm_output(&mut output, b"EXT:", &vm_external);
    let _ = io::stdout().write_all(&output);
    0
}

fn main() -> ExitCode {
    ExitCode::from(real_main())
}
