// Translation of c_src/src/main.c to Rust.
//
// Behavior is preserved exactly, including the original's quirks:
//   * `scanf("%d")` skips whitespace and so reads across newlines.
//   * The split position is read as an `int` but passed to a `size_t`
//     parameter, so a negative value becomes a huge unsigned value and is
//     reported verbatim in the error message.
//   * Validation order, error strings and exit codes are unchanged.
//
// Copyright notice from the original source is retained in c_src/.

mod buffer;
mod scan;

use std::io::Write;

use buffer::{
    buffer_copy, buffer_interleave, buffer_merge, buffer_reverse, buffer_rotate, buffer_split,
    calculate_checksum, init_buffer_array, Buffer,
};
use scan::Scanner;

const OP_COPY: i32 = 0;
const OP_REVERSE: i32 = 1;
const OP_MERGE: i32 = 2;
const OP_SPLIT: i32 = 3;
const OP_INTERLEAVE: i32 = 4;
const OP_ROTATE: i32 = 5;
const OP_CHECKSUM: i32 = 6;

// ==================== Input/Output Functions ====================

/// Read buffer from stdin.
fn read_buffer<R: std::io::Read>(sc: &mut Scanner<R>, buf: &mut Buffer) -> i32 {
    let mut length: i32 = 0;
    if sc.scan_int(&mut length) != 1 {
        eprint!("Error: Failed to read buffer length\n");
        return -1;
    }

    if length < 0 || length > 256 {
        eprint!("Error: Invalid buffer length {}\n", length);
        return -1;
    }

    buf.length = length as usize;
    for i in 0..buf.length {
        let mut byte: i32 = 0;
        if sc.scan_int(&mut byte) != 1 {
            eprint!("Error: Failed to read byte {}\n", i);
            return -1;
        }
        buf.data[i] = byte as u8;
    }

    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    0
}

/// Write buffer to stdout: `%zu` followed by ` %u` per byte, then a newline.
fn write_buffer(buf: &Buffer) {
    let mut line = String::new();
    line.push_str(&buf.length.to_string());
    for i in 0..buf.length {
        line.push(' ');
        line.push_str(&buf.data[i].to_string());
    }
    line.push('\n');
    print!("{}", line);
}

// ==================== Main Function ====================

fn run() -> i32 {
    let stdin = std::io::stdin();
    let mut sc = Scanner::new(stdin.lock());

    let mut operation: i32 = 0;
    let mut buffer_count: i32 = 0;

    // Read operation type
    if sc.scan_int(&mut operation) != 1 {
        eprint!("Error: Failed to read operation\n");
        return 1;
    }

    // Read buffer count
    if sc.scan_int(&mut buffer_count) != 1 {
        eprint!("Error: Failed to read buffer count\n");
        return 1;
    }

    if buffer_count <= 0 || buffer_count > 100 {
        eprint!("Error: Invalid buffer count {}\n", buffer_count);
        return 1;
    }

    // Allocate buffer array
    let mut buffers = match init_buffer_array(buffer_count) {
        Some(b) => b,
        None => return 1,
    };

    // Read all buffers
    for i in 0..buffer_count as usize {
        if read_buffer(&mut sc, &mut buffers.buffers[i]) != 0 {
            return 1;
        }
        buffers.count += 1;
    }

    // Execute operation based on type
    let mut result: i32 = 0;
    match operation {
        OP_COPY => {
            if buffer_count >= 2 {
                let mut temp = Buffer::new();
                let src = buffers.buffers[0];
                result = buffer_copy(&src, &mut temp);
                if result == 0 {
                    write_buffer(&temp);
                }
            } else {
                eprint!("Error: Copy needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_REVERSE => {
            for i in 0..buffer_count as usize {
                result = buffer_reverse(&mut buffers.buffers[i]);
                if result != 0 {
                    break;
                }
                write_buffer(&buffers.buffers[i]);
            }
        }

        OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged = Buffer::new();
                let (a, b) = (buffers.buffers[0], buffers.buffers[1]);
                result = buffer_merge(&a, &b, &mut merged);
                if result == 0 {
                    write_buffer(&merged);
                }
            } else {
                eprint!("Error: Merge needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_SPLIT => {
            if buffer_count >= 1 {
                let mut split_pos: i32 = 0;
                if sc.scan_int(&mut split_pos) != 1 {
                    eprint!("Error: Failed to read split position\n");
                    result = -1;
                } else {
                    let mut part1 = Buffer::new();
                    let mut part2 = Buffer::new();
                    let src = buffers.buffers[0];
                    // `int` -> `size_t`: sign-extends, matching the C call.
                    result = buffer_split(&src, split_pos as isize as usize, &mut part1, &mut part2);
                    if result == 0 {
                        write_buffer(&part1);
                        write_buffer(&part2);
                    }
                }
            }
        }

        OP_INTERLEAVE => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::new();
                let (a, b) = (buffers.buffers[0], buffers.buffers[1]);
                result = buffer_interleave(&a, &b, &mut interleaved);
                if result == 0 {
                    write_buffer(&interleaved);
                }
            } else {
                eprint!("Error: Interleave needs at least 2 buffers\n");
                result = -1;
            }
        }

        OP_ROTATE => {
            let mut positions: i32 = 0;
            if sc.scan_int(&mut positions) != 1 {
                eprint!("Error: Failed to read rotation amount\n");
                result = -1;
            } else {
                for i in 0..buffer_count as usize {
                    result = buffer_rotate(&mut buffers.buffers[i], positions);
                    if result != 0 {
                        break;
                    }
                    write_buffer(&buffers.buffers[i]);
                }
            }
        }

        OP_CHECKSUM => {
            for i in 0..buffer_count as usize {
                print!("{}\n", buffers.buffers[i].checksum);
            }
        }

        _ => {
            eprint!("Error: Unknown operation {}\n", operation);
            result = -1;
        }
    }

    if result != 0 {
        1
    } else {
        0
    }
}

/// The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main`, so a write
/// to a closed pipe returns `EPIPE` and `print!` panics. The C program keeps the
/// default disposition and is therefore killed by the signal (wait status 141).
/// Restore the default so a truncated stdout reader yields the same status.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();
    let code = run();
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code);
}
