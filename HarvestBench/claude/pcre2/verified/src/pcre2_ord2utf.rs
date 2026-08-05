use crate::pcre2_internal::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ord2utf_8(mut cvalue: u32, buffer: *mut PCRE2_UCHAR) -> u32 {
    let mut i: u32 = 0;
    while (i as usize) < _pcre2_utf8_table1_size as usize {
        if (cvalue as i32) <= _pcre2_utf8_table1[i as usize] {
            break;
        }
        i += 1;
    }
    let mut b = buffer.add(i as usize);
    let mut j = i;
    while j != 0 {
        *b = (0x80 | (cvalue & 0x3f)) as u8;
        b = b.sub(1);
        cvalue >>= 6;
        j -= 1;
    }
    *b = (_pcre2_utf8_table2[i as usize] | cvalue as i32) as u8;
    i + 1
}
