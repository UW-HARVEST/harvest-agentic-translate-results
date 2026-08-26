use std::env;
use std::ffi::OsStr;
use std::io::{self, BufRead, Write};
use std::os::unix::ffi::OsStrExt;

#[derive(Default)]
struct Vm {
    stack: Vec<i32>,
    trace: Vec<i32>,
    steps: i32,
}

#[derive(Default)]
struct Engines {
    state_a: i32,
    flipflop: i32,
}

fn external_target(code: i32) -> i32 {
    if code < 0 {
        return 7;
    }
    match code % 10 {
        0 => 0,
        1..=3 => 1,
        4..=6 => 2,
        7 => 3,
        _ => 4,
    }
}

impl Engines {
    fn target_a(&mut self, code: i32) -> i32 {
        if code < 0 {
            return if self.state_a & 1 != 0 { 6 } else { 5 };
        }
        self.state_a ^= code.wrapping_shl(1);
        match ((code >> 2) ^ self.state_a) & 7 {
            0 => 0,
            1 => 2,
            2 => 4,
            3 => 1,
            4 => 3,
            5 | 6 => 5,
            _ => 7,
        }
    }

    fn call_a_once(&mut self, x: i32) -> i32 {
        let a = self.target_a(x);
        let b = self.target_a(a.wrapping_sub(5));
        let c = self.target_a(b ^ 3);
        let d = self.target_a((b ^ 0x55).wrapping_add(7));
        a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
    }

    fn process_a_stream(&mut self, xs: &[i32]) -> i32 {
        let mut acc = 0usize;
        for &v in xs {
            for j in 0..3 {
                let t = self.target_a(v.wrapping_add(j));
                if t & 1 == 0 {
                    acc = acc.wrapping_add(t as usize);
                    continue;
                }
                acc ^= (t.wrapping_shl(j as u32)) as usize;
                if t == 5 {
                    break;
                }
            }
        }

        if acc > i32::MAX as usize {
            acc = i32::MAX as usize;
        }
        #[cfg(target_pointer_width = "64")]
        {
            // In the C source, size_t is compared with a negative long long.
            // The usual conversions make the second comparison true on LP64.
            let _ = acc;
            i32::MIN
        }
        #[cfg(target_pointer_width = "32")]
        {
            acc as i32
        }
    }

    fn target_b(&mut self, code: i32) -> i32 {
        self.flipflop ^= 1;
        if code < 0 {
            return if self.flipflop != 0 { 2 } else { 6 };
        }
        let z = (code ^ if self.flipflop != 0 { 0x7f } else { 0x1f }) % 8;
        match z {
            0 | 7 => 4,
            1 | 2 => 3,
            3 => 1,
            4 => 0,
            5 => 5,
            _ => 7,
        }
    }

    fn call_b_once(&mut self, x: i32) -> i32 {
        let a = self.target_b(x);
        let b = self.target_b(a.wrapping_add(9));
        let c = self.target_b((a.wrapping_add(9) ^ 0x2222).wrapping_sub(17));
        let d = self.target_b(c ^ x);
        a.wrapping_shl(1) ^ b.wrapping_shl(2) ^ c.wrapping_shl(3) ^ d.wrapping_shl(4)
    }

    fn process_b_stream(&mut self, xs: &[i32]) -> i32 {
        let mut acc = 1i32;
        for &v in xs {
            let mut iter = 0i32;
            loop {
                iter = iter.wrapping_add(1);
                if iter > 4 {
                    break;
                }
                let t = self.target_b(v.wrapping_sub(iter));
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

    fn classify(&mut self, implementation: i32, x: i32) -> i32 {
        match implementation {
            0 => self.call_a_once(x),
            1 => self.call_b_once(x.wrapping_add(1)),
            _ => external_target(external_target(x.wrapping_add(1))),
        }
    }

    fn process_stream(&mut self, implementation: i32, buf: &[i32]) -> i32 {
        match implementation {
            0 => self.process_a_stream(buf),
            1 => self.process_b_stream(buf),
            _ => {
                let mut acc = 0i32;
                for &value in buf {
                    let t = external_target(value);
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

    fn run_engine(&mut self, implementation: i32, code: &[i32], vm: &mut Vm) -> i32 {
        let mut ip = 0usize;
        while let Some(&op) = code.get(ip) {
            ip += 1;
            vm.steps = vm.steps.wrapping_add(1);
            match op {
                0 => {
                    let Some(&immediate) = code.get(ip) else {
                        return 1;
                    };
                    ip += 1;
                    vm.stack.push(immediate);
                    vm.trace.push(0);
                }
                1 => {
                    let Some(b) = vm.stack.pop() else {
                        return 2;
                    };
                    let Some(a) = vm.stack.pop() else {
                        return 2;
                    };
                    vm.stack.push(a.wrapping_add(b));
                    vm.trace.push(1);
                }
                2 => {
                    let Some(b) = vm.stack.pop() else {
                        return 3;
                    };
                    let Some(a) = vm.stack.pop() else {
                        return 3;
                    };
                    vm.stack.push(a.wrapping_mul(b));
                    vm.trace.push(2);
                }
                3 => {
                    let a = vm.stack.last().copied().unwrap_or(0);
                    vm.stack.push(a);
                    vm.trace.push(3);
                }
                4 => {
                    if vm.stack.pop().is_none() {
                        return 4;
                    }
                    vm.trace.push(4);
                }
                5 => {
                    let x = vm.stack.last().copied().unwrap_or(0);
                    let bucket = self.classify(implementation, x);
                    vm.stack.push(bucket);
                    vm.trace.push(match bucket {
                        0 => 5,
                        1 => 6,
                        2 => 7,
                        3 | 4 => 8,
                        _ => 9,
                    });
                }
                6 => {
                    let Some(&k) = code.get(ip) else {
                        return 5;
                    };
                    ip += 1;
                    let Some(condition) = vm.stack.pop() else {
                        return 6;
                    };
                    if condition != 0 {
                        if (k as usize) > code.len() - ip {
                            return 7;
                        }
                        ip += k as usize;
                        vm.trace.push(10);
                    } else {
                        vm.trace.push(11);
                    }
                }
                7 => {
                    let Some(&times) = code.get(ip) else {
                        return 8;
                    };
                    ip += 1;
                    if ip >= code.len() {
                        return 9;
                    }
                    let saved_ip = ip;
                    for _ in 0..times {
                        let rc = self.run_engine(implementation, &code[saved_ip..saved_ip + 1], vm);
                        if rc != 0 {
                            vm.trace.push(12);
                            break;
                        }
                    }
                    ip = saved_ip + 1;
                }
                8 => {
                    let x = vm.stack.last().copied().unwrap_or(0);
                    let y = self.classify(implementation, x);
                    vm.stack.push(y);
                    vm.trace.push(13);
                }
                9 => {
                    let Some(&m) = code.get(ip) else {
                        return 10;
                    };
                    ip += 1;
                    if m < 0 || (m as usize) > vm.stack.len() {
                        return 11;
                    }
                    let m = m as usize;
                    let mut tmp = vec![0i32; m];
                    for i in (0..m).rev() {
                        tmp[i] = vm.stack.pop().unwrap();
                    }
                    for i in (0..m).rev() {
                        if let Some(value) = vm.stack.pop() {
                            tmp[i] = value;
                        }
                    }
                    let result = self.process_stream(implementation, &tmp);
                    vm.stack.push(result);
                    vm.trace.push(14);
                }
                10 => return 0,
                _ => return 99,
            }
        }
        0
    }
}

fn parse_c_long(bytes: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let negative = match bytes.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let digit_start = i;
    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value = 0u64;
    while let Some(&byte) = bytes.get(i) {
        if !byte.is_ascii_digit() {
            break;
        }
        value = value
            .saturating_mul(10)
            .saturating_add((byte - b'0') as u64)
            .min(limit);
        i += 1;
    }
    if i == digit_start || i != bytes.len() {
        return None;
    }
    let value = if negative {
        if value == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(value as i64)
        }
    } else {
        value as i64
    };
    Some(value as i32)
}

fn parse_stdin_chunk(chunk: &[u8], code: &mut Vec<i32>) {
    let end = chunk
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(chunk.len());
    let chunk = &chunk[..end];
    let mut p = 0usize;
    while p < chunk.len() {
        let mut q = p;
        while q < chunk.len() && !matches!(chunk[q], b' ' | b'\t' | b'\n' | b'\r') {
            q += 1;
        }
        if q > p {
            if let Some(value) = parse_c_long(&chunk[p..q]) {
                code.push(value);
            }
        }
        p = if q < chunk.len() { q + 1 } else { q };
    }
}

fn read_fgets<R: BufRead>(reader: &mut R, output: &mut Vec<u8>) -> bool {
    const LIMIT: usize = 4095;
    output.clear();
    while output.len() < LIMIT {
        let available = match reader.fill_buf() {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        if available.is_empty() {
            return !output.is_empty();
        }
        let available_len = available.len().min(LIMIT - output.len());
        let newline = available[..available_len]
            .iter()
            .position(|&byte| byte == b'\n');
        let take = newline.map_or(available_len, |position| position + 1);
        output.extend_from_slice(&available[..take]);
        reader.consume(take);
        if newline.is_some() {
            return true;
        }
    }
    true
}

fn read_stdin(code: &mut Vec<i32>) {
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut chunk = Vec::with_capacity(4095);
    while read_fgets(&mut reader, &mut chunk) {
        parse_stdin_chunk(&chunk, code);
    }
}

fn write_usage(stderr: &mut impl Write, program: &[u8]) {
    let _ = stderr.write_all(b"Usage: ");
    let _ = stderr.write_all(program);
    let _ = stderr.write_all(
        b" [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n",
    );
}

fn vm_output(output: &mut Vec<u8>, label: &[u8], vm: &Vm) {
    output.extend_from_slice(label);
    let top = vm.stack.last().copied().unwrap_or(-777);
    write!(output, "STACK_TOP={top} STEPS={} TRACE=", vm.steps).unwrap();
    for &entry in &vm.trace {
        output.push(b"abcdefghijklmnopqrstuvwxyz"[(entry & 25) as usize]);
    }
    output.push(b'\n');
}

fn os_bytes(value: &OsStr) -> &[u8] {
    value.as_bytes()
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    let program = args.first().map_or(&b""[..], |arg| os_bytes(arg));
    let mut code = Vec::new();
    let mut use_stdin = false;
    let stderr = io::stderr();
    let mut stderr = stderr.lock();

    for arg in args.iter().skip(1) {
        let arg = os_bytes(arg);
        if arg == b"--help" {
            write_usage(&mut stderr, program);
            return;
        } else if arg == b"--stdin" {
            use_stdin = true;
        } else if arg.is_empty() {
            code.push(0);
        } else if let Some(value) = parse_c_long(arg) {
            code.push(value);
        } else {
            let _ = stderr.write_all(b"skip '");
            let _ = stderr.write_all(arg);
            let _ = stderr.write_all(b"'\n");
        }
    }

    if use_stdin {
        read_stdin(&mut code);
    }
    if code.is_empty() {
        let _ = stderr.write_all(b"no program\n");
        std::process::exit(2);
    }

    let mut engines = Engines::default();
    let mut vm_a = Vm::default();
    let mut vm_b = Vm::default();
    let mut vm_external = Vm::default();
    let rc_a = engines.run_engine(0, &code, &mut vm_a);
    let rc_b = engines.run_engine(1, &code, &mut vm_b);
    let rc_external = engines.run_engine(2, &code, &mut vm_external);

    let mut output = Vec::new();
    writeln!(output, "RC:A={rc_a} B={rc_b} EXT={rc_external}").unwrap();
    vm_output(&mut output, b"A:", &vm_a);
    vm_output(&mut output, b"B:", &vm_b);
    vm_output(&mut output, b"EXT:", &vm_external);
    let _ = io::stdout().write_all(&output);
}
