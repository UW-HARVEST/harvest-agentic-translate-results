use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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
        self.version = i16::from(bytes[0] >> 4);
        self.argc = i16::from(bytes[0] & 0x0f);
        self.buf = buf[1..].to_string();
        cursor_map()
            .lock()
            .expect("amp cursor map poisoned")
            .insert(self as *const Self as usize, 0);
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let key = self as *const Self as usize;
        let mut cursors = cursor_map().lock().expect("amp cursor map poisoned");
        let cursor = cursors.entry(key).or_insert(0);
        let len = read_u32_be(&self.buf.as_bytes()[*cursor..*cursor + 4]) as usize;
        let start = *cursor + 4;
        let end = start + len;
        *cursor = end;
        &self.buf[start..end]
    }
}

impl Drop for Amp {
    fn drop(&mut self) {
        if let Ok(mut cursors) = cursor_map().lock() {
            cursors.remove(&(self as *const Self as usize));
        }
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
    let mut buf = Vec::with_capacity(1 + argv.iter().map(|arg| 4 + arg.len()).sum::<usize>());
    buf.push(((AMP_VERSION as u8) << 4) | (argv.len() as u8 & 0x0f));

    for arg in argv {
        let len = arg.len() as u32;
        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(arg.as_bytes());
    }

    // AMP is a byte protocol; the length prefix is not guaranteed to be UTF-8.
    unsafe { String::from_utf8_unchecked(buf) }
}

fn cursor_map() -> &'static Mutex<HashMap<usize, usize>> {
    static CURSORS: OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();
    CURSORS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn read_u32_be(buf: &[u8]) -> u32 {
    u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]])
}
