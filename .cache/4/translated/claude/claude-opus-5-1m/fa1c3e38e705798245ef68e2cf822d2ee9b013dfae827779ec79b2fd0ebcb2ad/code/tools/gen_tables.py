import os, struct, sys

D = sys.argv[1]  # dump dir

def raw(name):
    return open(os.path.join(D, name + '.bin'), 'rb').read()

def arr(name, ty):
    b = raw(name)
    if ty == 'u8':
        vals = list(b)
    elif ty == 'u16':
        vals = list(struct.unpack('<%dH' % (len(b)//2), b))
    elif ty == 'u32':
        vals = list(struct.unpack('<%dI' % (len(b)//4), b))
    elif ty == 'i32':
        vals = list(struct.unpack('<%di' % (len(b)//4), b))
    elif ty == 'usize':
        vals = list(struct.unpack('<%dQ' % (len(b)//8), b))
    else:
        raise Exception(ty)
    return vals

def fmt_array(name, ty, vals, per_line=16):
    out = ['#[unsafe(no_mangle)]', 'pub static %s: [%s; %d] = [' % (name, ty, len(vals))]
    for i in range(0, len(vals), per_line):
        out.append('  ' + ', '.join(str(v) for v in vals[i:i+per_line]) + ',')
    out.append('];')
    return '\n'.join(out)

def fmt_scalar(name, ty, val):
    return '#[unsafe(no_mangle)]\npub static %s: %s = %d;' % (name, ty, val)

HDR = '''// %s
// Data tables mechanically transcribed from the C build (byte-for-byte identical).
#![allow(dead_code, non_upper_case_globals)]

use crate::internal::*;

'''

# ---------------------------------------------------------------- chartables
out = [HDR % 'Translated from c_src/src/pcre2_chartables.c']
out.append(fmt_array('_pcre2_default_tables_8', 'u8', arr('_pcre2_default_tables_8', 'u8')))
open('src/pcre2_chartables.rs', 'w').write('\n'.join(out) + '\n')

# ---------------------------------------------------------------- pcre2_tables
out = [HDR % 'Translated from c_src/src/pcre2_tables.c and pcre2_ucptables_inc.h']
out.append(fmt_array('_pcre2_OP_lengths_8', 'u8', arr('_pcre2_OP_lengths_8', 'u8')))
out.append('')
out.append(fmt_array('_pcre2_hspace_list_8', 'u32', arr('_pcre2_hspace_list_8', 'u32'), 8))
out.append(fmt_array('_pcre2_vspace_list_8', 'u32', arr('_pcre2_vspace_list_8', 'u32'), 8))
out.append(fmt_array('_pcre2_callout_start_delims_8', 'u32',
                     arr('_pcre2_callout_start_delims_8', 'u32'), 8))
out.append(fmt_array('_pcre2_callout_end_delims_8', 'u32',
                     arr('_pcre2_callout_end_delims_8', 'u32'), 8))
out.append(fmt_array('_pcre2_utf8_table1', 'i32', arr('_pcre2_utf8_table1', 'i32'), 8))
out.append(fmt_scalar('_pcre2_utf8_table1_size', 'c_uint', arr('_pcre2_utf8_table1_size', 'u32')[0]))
out.append(fmt_array('_pcre2_utf8_table2', 'i32', arr('_pcre2_utf8_table2', 'i32'), 8))
out.append(fmt_array('_pcre2_utf8_table3', 'i32', arr('_pcre2_utf8_table3', 'i32'), 8))
out.append(fmt_array('_pcre2_utf8_table4', 'u8', arr('_pcre2_utf8_table4', 'u8')))
out.append(fmt_array('_pcre2_ucp_gentype_8', 'u32', arr('_pcre2_ucp_gentype_8', 'u32'), 8))
out.append(fmt_array('_pcre2_ucp_gbtable_8', 'u32', arr('_pcre2_ucp_gbtable_8', 'u32'), 4))

# utt table: struct { u16 name_offset; u16 type; u16 value; }
b = raw('_pcre2_utt_8')
n = len(b) // 6
lines = ['#[unsafe(no_mangle)]', 'pub static _pcre2_utt_8: [ucp_type_table; %d] = [' % n]
for i in range(n):
    no, ty, va = struct.unpack_from('<HHH', b, i*6)
    lines.append('  ucp_type_table { name_offset: %d, type_: %d, value: %d },' % (no, ty, va))
lines.append('];')
out.append('\n'.join(lines))

# utt_names: char array
b = raw('_pcre2_utt_names_8')
lines = ['#[unsafe(no_mangle)]', 'pub static _pcre2_utt_names_8: [u8; %d] = [' % len(b)]
for i in range(0, len(b), 20):
    lines.append('  ' + ', '.join(str(v) for v in b[i:i+20]) + ',')
lines.append('];')
out.append('\n'.join(lines))
out.append(fmt_scalar('_pcre2_utt_size_8', 'usize', arr('_pcre2_utt_size_8', 'usize')[0]))
open('src/pcre2_tables.rs', 'w').write('\n'.join(out) + '\n')

# ---------------------------------------------------------------- pcre2_ucd
out = [HDR % 'Translated from c_src/src/pcre2_ucd.c']
uv = raw('_pcre2_unicode_version_8')
assert uv[-1] == 0
out.append('static UNICODE_VERSION_STR: [u8; %d] = %s;' % (len(uv), '[' + ', '.join(str(v) for v in uv) + ']'))
out.append('#[unsafe(no_mangle)]')
out.append('pub static mut _pcre2_unicode_version_8: *const c_char =')
out.append('    UNICODE_VERSION_STR.as_ptr() as *const c_char;')
out.append('')
out.append(fmt_array('_pcre2_ucd_caseless_sets_8', 'u32', arr('_pcre2_ucd_caseless_sets_8', 'u32'), 10))
out.append(fmt_scalar('_pcre2_ucd_turkish_dotted_i_caseset_8', 'u32',
                      arr('_pcre2_ucd_turkish_dotted_i_caseset_8', 'u32')[0]))
out.append(fmt_array('_pcre2_ucd_nocase_ranges_8', 'u32', arr('_pcre2_ucd_nocase_ranges_8', 'u32'), 6))
out.append(fmt_scalar('_pcre2_ucd_nocase_ranges_size_8', 'u32',
                      arr('_pcre2_ucd_nocase_ranges_size_8', 'u32')[0]))
out.append(fmt_array('_pcre2_ucd_digit_sets_8', 'u32', arr('_pcre2_ucd_digit_sets_8', 'u32'), 10))
out.append(fmt_array('_pcre2_ucd_script_sets_8', 'u32', arr('_pcre2_ucd_script_sets_8', 'u32'), 6))
out.append(fmt_array('_pcre2_ucd_boolprop_sets_8', 'u32', arr('_pcre2_ucd_boolprop_sets_8', 'u32'), 6))

b = raw('_pcre2_ucd_records_8')
n = len(b) // 12
lines = ['#[unsafe(no_mangle)]', 'pub static _pcre2_ucd_records_8: [ucd_record; %d] = [' % n]
for i in range(n):
    sc, ct, gb, cs, oc, sx, bp = struct.unpack_from('<BBBBiHH', b, i*12)
    lines.append('  ucd_record { script: %d, chartype: %d, gbprop: %d, caseset: %d, other_case: %d, scriptx_bidiclass: %d, bprops: %d },'
                 % (sc, ct, gb, cs, oc, sx, bp))
lines.append('];')
out.append('\n'.join(lines))
out.append(fmt_array('_pcre2_ucd_stage1_8', 'u16', arr('_pcre2_ucd_stage1_8', 'u16'), 20))
out.append(fmt_array('_pcre2_ucd_stage2_8', 'u16', arr('_pcre2_ucd_stage2_8', 'u16'), 20))
open('src/pcre2_ucd.rs', 'w').write('\n'.join(out) + '\n')

# ---------------------------------------------------------------- posix maps
vals = arr('_pcre2_posix_class_maps8', 'i32')
print('posix_class_maps count =', len(vals), vals)
open('/tmp/posix_maps.txt', 'w') if False else None
with open('src/_posix_class_maps.rs.part', 'w') as f:
    f.write(fmt_array('_pcre2_posix_class_maps8', 'i32', vals, 6) + '\n')

print('done')
