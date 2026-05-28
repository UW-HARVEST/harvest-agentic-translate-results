// util.rs - utility functions

// Format a pointer the way glibc's printf("%p", ptr) does.
// For non-null: "0x" followed by lowercase hex with no leading zeros.
// For null: "(nil)".
pub fn format_ptr(p: *const u8) -> String {
    if p.is_null() {
        "(nil)".to_string()
    } else {
        format!("0x{:x}", p as usize)
    }
}
