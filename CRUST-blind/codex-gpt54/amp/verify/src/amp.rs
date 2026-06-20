use std::cell::RefCell;
use std::collections::HashMap;

/// Protocol version.
pub const AMP_VERSION: i16 = 1;
/// Message struct.
pub struct Amp {
    /// Protocol version.
    pub version: i16,
    /// Number of arguments.
    pub argc: i16,
    /// Encoded buffer.
    pub buf: String,
}
impl Amp {
    /// Decodes the given buffer into this message.
    ///
    /// # Arguments
    ///
    /// * `buf` - A string slice containing the encoded message.
    pub fn decode(&mut self, buf: &str) {
        let bytes = buf.as_bytes();
        let header = bytes.first().copied().unwrap_or_default();

        self.version = i16::from(header >> 4);
        self.argc = i16::from(header & 0x0f);
        self.buf.clear();

        CURSORS.with(|cursors| {
            cursors
                .borrow_mut()
                .insert(self as *const Self as usize, bytes.get(1..).unwrap_or(&[]).to_vec());
        });
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let key = self as *const Self as usize;
        let next = CURSORS.with(|cursors| {
            let mut cursors = cursors.borrow_mut();
            let Some(remaining) = cursors.get_mut(&key) else {
                return Vec::new();
            };

            if remaining.len() < 4 {
                remaining.clear();
                return Vec::new();
            }

            let len = read_u32_be(&remaining[..4]) as usize;
            let available = remaining.len().saturating_sub(4);
            let take = len.min(available);
            let arg = remaining[4..4 + take].to_vec();
            remaining.drain(..4 + take);
            arg
        });

        self.buf = String::from_utf8(next)
            .unwrap_or_else(|err| String::from_utf8_lossy(&err.into_bytes()).into_owned());
        self.buf.as_str()
    }
}
/// Encodes the given arguments into a message buffer.
///
/// # Arguments
///
/// * `argv` - A slice of string slices representing the arguments.
///
/// # Returns
///
/// A `String` containing the encoded message.
pub fn amp_encode(argv: &[&str]) -> String {
    let total_len = 1 + argv.iter().map(|arg| 4 + arg.len()).sum::<usize>();
    let mut buf = Vec::with_capacity(total_len);
    let argc = (argv.len() & 0x0f) as u8;

    buf.push(((AMP_VERSION as u8) << 4) | argc);

    for arg in argv {
        let len = arg.len() as u32;
        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(arg.as_bytes());
    }

    String::from_utf8(buf).unwrap_or_else(|err| {
        String::from_utf8_lossy(&err.into_bytes()).into_owned()
    })
}

thread_local! {
    static CURSORS: RefCell<HashMap<usize, Vec<u8>>> = RefCell::new(HashMap::new());
}

fn read_u32_be(buf: &[u8]) -> u32 {
    let mut bytes = [0_u8; 4];
    bytes.copy_from_slice(&buf[..4]);
    u32::from_be_bytes(bytes)
}
