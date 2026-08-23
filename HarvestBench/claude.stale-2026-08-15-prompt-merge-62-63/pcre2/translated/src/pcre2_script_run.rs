use crate::pcre2_internal::*;

const SCRIPT_UNSET: u32 = 0;
const SCRIPT_MAP: u32 = 1;
const SCRIPT_HANPENDING: u32 = 2;
const SCRIPT_HANHIRAKATA: u32 = 3;
const SCRIPT_HANBOPOMOFO: u32 = 4;
const SCRIPT_HANHANGUL: u32 = 5;

const UCD_MAPSIZE: usize = (ucp_Unknown as usize) / 32 + 1;
const FULL_MAPSIZE: usize = (ucp_Script_Count as usize) / 32 + 1;

const FOUND_BOPOMOFO: u32 = 1;
const FOUND_HIRAGANA: u32 = 2;
const FOUND_KATAKANA: u32 = 4;
const FOUND_HANGUL: u32 = 8;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_script_run_8(
    mut ptr: PCRE2_SPTR,
    endptr: PCRE2_SPTR,
    utf: BOOL,
) -> BOOL {
    let mut require_state: u32 = SCRIPT_UNSET;
    let mut require_map = [0u32; FULL_MAPSIZE];
    let mut map = [0u32; FULL_MAPSIZE];
    let mut require_digitset: u32 = 0;
    let mut c: u32;

    if ptr >= endptr {
        return TRUE;
    }
    // GETCHARINCTEST
    {
        let (cc, consumed) = getcharinctest(ptr, utf);
        c = cc;
        ptr = ptr.add(consumed);
    }
    if ptr >= endptr {
        return TRUE;
    }

    // require_map already zeroed.

    loop {
        let ucd = GET_UCD(c);
        let script = ucd.script as u32;

        if script == ucp_Unknown {
            return FALSE;
        }

        if UCD_SCRIPTX_PROP(ucd) != 0 || (script != ucp_Inherited && script != ucp_Common) {
            // Build full-sized map.
            let sx = UCD_SCRIPTX_PROP(ucd) as usize;
            for i in 0..UCD_MAPSIZE {
                map[i] = _pcre2_ucd_script_sets_8[sx + i];
            }
            for i in UCD_MAPSIZE..FULL_MAPSIZE {
                map[i] = 0;
            }
            if script != ucp_Common && script != ucp_Inherited {
                MAPSET(&mut map, script);
            }

            match require_state {
                SCRIPT_UNSET => match script {
                    x if x == ucp_Han => require_state = SCRIPT_HANPENDING,
                    x if x == ucp_Hiragana || x == ucp_Katakana => {
                        require_state = SCRIPT_HANHIRAKATA
                    }
                    x if x == ucp_Bopomofo => require_state = SCRIPT_HANBOPOMOFO,
                    x if x == ucp_Hangul => require_state = SCRIPT_HANHANGUL,
                    _ => {
                        require_map.copy_from_slice(&map);
                        require_state = SCRIPT_MAP;
                    }
                },

                SCRIPT_HANPENDING => {
                    if script != ucp_Han {
                        let mut chspecial: u32 = 0;
                        if MAPBIT(&map, ucp_Bopomofo) != 0 {
                            chspecial |= FOUND_BOPOMOFO;
                        }
                        if MAPBIT(&map, ucp_Hiragana) != 0 {
                            chspecial |= FOUND_HIRAGANA;
                        }
                        if MAPBIT(&map, ucp_Katakana) != 0 {
                            chspecial |= FOUND_KATAKANA;
                        }
                        if MAPBIT(&map, ucp_Hangul) != 0 {
                            chspecial |= FOUND_HANGUL;
                        }

                        if chspecial == 0 {
                            return FALSE;
                        }

                        if chspecial == FOUND_BOPOMOFO {
                            require_state = SCRIPT_HANBOPOMOFO;
                        } else if chspecial == (FOUND_HIRAGANA | FOUND_KATAKANA) {
                            require_state = SCRIPT_HANHIRAKATA;
                        }
                    }
                }

                SCRIPT_HANHIRAKATA => {
                    if MAPBIT(&map, ucp_Han) + MAPBIT(&map, ucp_Hiragana) + MAPBIT(&map, ucp_Katakana)
                        == 0
                    {
                        return FALSE;
                    }
                }

                SCRIPT_HANBOPOMOFO => {
                    if MAPBIT(&map, ucp_Han) + MAPBIT(&map, ucp_Bopomofo) == 0 {
                        return FALSE;
                    }
                }

                SCRIPT_HANHANGUL => {
                    if MAPBIT(&map, ucp_Han) + MAPBIT(&map, ucp_Hangul) == 0 {
                        return FALSE;
                    }
                }

                SCRIPT_MAP => {
                    let mut ok = false;
                    for i in 0..FULL_MAPSIZE {
                        if (require_map[i] & map[i]) != 0 {
                            ok = true;
                            break;
                        }
                    }
                    if !ok {
                        return FALSE;
                    }

                    match script {
                        x if x == ucp_Han => require_state = SCRIPT_HANPENDING,
                        x if x == ucp_Hiragana || x == ucp_Katakana => {
                            require_state = SCRIPT_HANHIRAKATA
                        }
                        x if x == ucp_Bopomofo => require_state = SCRIPT_HANBOPOMOFO,
                        x if x == ucp_Hangul => require_state = SCRIPT_HANHANGUL,
                        _ => {
                            for i in 0..FULL_MAPSIZE {
                                require_map[i] &= map[i];
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        if ucd.chartype as u32 == ucp_Nd {
            let digitset: u32;
            if c <= _pcre2_ucd_digit_sets_8[1] {
                digitset = 1;
            } else {
                let mut bot: i32 = 1;
                let mut top: i32 = _pcre2_ucd_digit_sets_8[0] as i32;
                loop {
                    if top <= bot + 1 {
                        digitset = top as u32;
                        break;
                    }
                    let mid = (top + bot) / 2;
                    if c <= _pcre2_ucd_digit_sets_8[mid as usize] {
                        top = mid;
                    } else {
                        bot = mid;
                    }
                }
            }

            if require_digitset == 0 {
                require_digitset = digitset;
            } else if digitset != require_digitset {
                return FALSE;
            }
        }

        if ptr >= endptr {
            return TRUE;
        }
        let (cc, consumed) = getcharinctest(ptr, utf);
        c = cc;
        ptr = ptr.add(consumed);
    }
}

#[inline]
unsafe fn getcharinctest(ptr: PCRE2_SPTR, utf: BOOL) -> (u32, usize) {
    if utf == 0 {
        (*ptr as u32, 1)
    } else {
        GETCHARINC(ptr)
    }
}
