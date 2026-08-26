pub struct Input {
    data: Vec<u8>,
    position: usize,
}

impl Input {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, position: 0 }
    }

    pub fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if self.position >= self.data.len() || size <= 1 {
            return None;
        }

        let start = self.position;
        let limit = (start + size - 1).min(self.data.len());
        while self.position < limit {
            let byte = self.data[self.position];
            self.position += 1;
            if byte == b'\n' {
                break;
            }
        }
        Some(self.data[start..self.position].to_vec())
    }

    pub fn scanf_i32(&mut self) -> Option<i32> {
        while self.position < self.data.len() && is_space(self.data[self.position]) {
            self.position += 1;
        }

        let mut negative = false;
        if self.position < self.data.len()
            && (self.data[self.position] == b'+' || self.data[self.position] == b'-')
        {
            negative = self.data[self.position] == b'-';
            self.position += 1;
        }

        let digit_start = self.position;
        let value = parse_digits(&self.data, &mut self.position, negative);
        if self.position == digit_start {
            return None;
        }
        Some(value)
    }

    pub fn discard_through_newline(&mut self) {
        while self.position < self.data.len() {
            let byte = self.data[self.position];
            self.position += 1;
            if byte == b'\n' {
                return;
            }
        }
        loop {
            std::hint::spin_loop();
        }
    }
}

pub fn sscanf_i32(data: &[u8]) -> Option<i32> {
    parse_i32(data, 0).map(|(value, _)| value)
}

pub fn fscanf_i32(data: &[u8], position: &mut usize) -> Option<i32> {
    let (value, end) = parse_i32(data, *position)?;
    *position = end;
    while *position < data.len() && is_space(data[*position]) {
        *position += 1;
    }
    Some(value)
}

pub fn c_line_value(data: &[u8]) -> Vec<u8> {
    let end = data
        .iter()
        .position(|&byte| byte == 0 || byte == b'\n')
        .unwrap_or(data.len());
    data[..end].to_vec()
}

fn parse_i32(data: &[u8], mut position: usize) -> Option<(i32, usize)> {
    while position < data.len() && is_space(data[position]) {
        position += 1;
    }

    let mut negative = false;
    if position < data.len() && (data[position] == b'+' || data[position] == b'-') {
        negative = data[position] == b'-';
        position += 1;
    }

    let digit_start = position;
    let value = parse_digits(data, &mut position, negative);
    if position == digit_start {
        return None;
    }

    Some((value, position))
}

fn parse_digits(data: &[u8], position: &mut usize, negative: bool) -> i32 {
    let mut magnitude = 0u64;
    while *position < data.len() && data[*position].is_ascii_digit() {
        magnitude = magnitude
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(data[*position] - b'0')))
            .unwrap_or(u64::MAX);
        *position += 1;
    }

    let long_value = if negative {
        const LONG_MIN_MAGNITUDE: u64 = 1u64 << 63;
        if magnitude >= LONG_MIN_MAGNITUDE {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else if magnitude > i64::MAX as u64 {
        i64::MAX
    } else {
        magnitude as i64
    };
    long_value as i32
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}
