use std::io::{self, Read};

pub trait ByteReader {
    fn getchar(&mut self) -> Option<u8>;
    fn peek(&mut self) -> Option<u8>;

    fn fgets(&mut self, size: usize) -> Option<String> {
        if size == 0 {
            return None;
        }

        let mut buf = Vec::new();
        let limit = size.saturating_sub(1);
        while buf.len() < limit {
            match self.getchar() {
                Some(byte) => {
                    buf.push(byte);
                    if byte == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }

        if buf.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(&buf).into_owned())
        }
    }

    fn scanf_d(&mut self) -> Option<i32> {
        loop {
            match self.peek() {
                Some(byte) if byte.is_ascii_whitespace() => {
                    self.getchar();
                }
                _ => break,
            }
        }

        let mut buf = Vec::new();

        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            buf.push(self.getchar().unwrap());
        }

        while matches!(self.peek(), Some(byte) if byte.is_ascii_digit()) {
            buf.push(self.getchar().unwrap());
        }

        let digits = match buf.as_slice() {
            [b'+'] | [b'-'] | [] => return None,
            [b'+', rest @ ..] | [b'-', rest @ ..] if rest.is_empty() => return None,
            _ => &buf[..],
        };

        let text = String::from_utf8_lossy(digits);
        text.parse::<i32>().ok()
    }

    fn flush_until_newline(&mut self) {
        loop {
            if self.getchar() == Some(b'\n') {
                break;
            }
        }
    }
}

pub struct StreamReader<R: Read> {
    inner: R,
    peeked: Option<Option<u8>>,
}

impl<R: Read> StreamReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner, peeked: None }
    }

    fn read_byte(&mut self) -> Option<u8> {
        let mut buf = [0_u8; 1];
        match self.inner.read(&mut buf) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(buf[0]),
        }
    }
}

impl<R: Read> ByteReader for StreamReader<R> {
    fn getchar(&mut self) -> Option<u8> {
        match self.peeked.take() {
            Some(byte) => byte,
            None => self.read_byte(),
        }
    }

    fn peek(&mut self) -> Option<u8> {
        match self.peeked {
            Some(byte) => byte,
            None => {
                let byte = self.read_byte();
                self.peeked = Some(byte);
                byte
            }
        }
    }
}

pub fn stdin_reader() -> StreamReader<io::StdinLock<'static>> {
    let stdin = Box::leak(Box::new(io::stdin()));
    StreamReader::new(stdin.lock())
}
