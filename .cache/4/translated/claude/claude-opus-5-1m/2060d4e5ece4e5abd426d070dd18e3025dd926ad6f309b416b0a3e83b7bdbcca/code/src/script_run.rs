// Translated from pcre2_script_run.c
//
// This module contains the function for checking a script run.

use crate::internal::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_int, c_void};
use core::mem::size_of;

/*************************************************
*                Check script run                *
*************************************************/

/* A script run is conceptually a sequence of characters all in the same
Unicode script. However, it isn't quite that simple. There are special rules
for scripts that are commonly used together, and also special rules for digits.

Arguments:
  pgr       point to the first character
  endptr    point after the last character
  utf       TRUE if in UTF mode

Returns:    TRUE if this is a valid script run
*/

/* These are states in the checking process. */

const SCRIPT_UNSET: u32 = 0; /* Requirement as yet unknown */
const SCRIPT_MAP: u32 = 1; /* Bitmap contains acceptable scripts */
const SCRIPT_HANPENDING: u32 = 2; /* Have had only Han characters */
const SCRIPT_HANHIRAKATA: u32 = 3; /* Expect Han or Hirikata */
const SCRIPT_HANBOPOMOFO: u32 = 4; /* Expect Han or Bopomofo */
const SCRIPT_HANHANGUL: u32 = 5; /* Expect Han or Hangul */

const UCD_MAPSIZE: usize = (ucp_Unknown / 32 + 1) as usize;
const FULL_MAPSIZE: usize = (ucp_Script_Count / 32 + 1) as usize;

const FOUND_BOPOMOFO: u32 = 1;
const FOUND_HIRAGANA: u32 = 2;
const FOUND_KATAKANA: u32 = 4;
const FOUND_HANGUL: u32 = 8;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_script_run_8(ptr: PCRE2_SPTR, endptr: PCRE2_SPTR, utf: BOOL) -> BOOL {
    let mut ptr: PCRE2_SPTR = ptr;
    let mut require_state: u32 = SCRIPT_UNSET;
    let mut require_map: [u32; FULL_MAPSIZE] = [0; FULL_MAPSIZE];
    let mut map: [u32; FULL_MAPSIZE] = [0; FULL_MAPSIZE];
    let mut require_digitset: u32 = 0;
    let mut c: u32;

    /* Any string containing fewer than 2 characters is a valid script run. */

    if ptr >= endptr {
        return TRUE;
    }
    /* GETCHARINCTEST(c, ptr) */
    c = *ptr as u32;
    ptr = ptr.add(1);
    if utf != 0 && c >= 0xc0 {
        let r = getutf8inc(c, ptr);
        c = r.0;
        ptr = r.1;
    }
    if ptr >= endptr {
        return TRUE;
    }

    /* Initialize the require map. This is a full-size bitmap that has a bit for
    every script, as opposed to the maps in ucd_script_sets, which only have bits
    for scripts less than ucp_Unknown - those that appear in script extension
    lists. */

    {
        let mut i: c_int = 0;
        while (i as usize) < FULL_MAPSIZE {
            require_map[i as usize] = 0;
            i += 1;
        }
    }

    /* Scan strings of two or more characters, checking the Unicode characteristics
    of each code point. */

    loop {
        let ucd: &ucd_record = GET_UCD(c);
        let script: u32 = ucd.script as u32;

        /* If the script is Unknown, the string is not a valid script run. Such
        characters can only form script runs of length one (see test above). */

        if script == ucp_Unknown {
            return FALSE;
        }

        /* A character without any script extensions whose script is Inherited or
        Common is always accepted with any script. If there are extensions, the
        following processing happens for all scripts. */

        if UCD_SCRIPTX_PROP(ucd) != 0 || (script != ucp_Inherited && script != ucp_Common) {
            let mut OK: BOOL;

            /* Set up a full-sized map for this character that can include bits for all
            scripts. */

            memcpy(
                map.as_mut_ptr() as *mut c_void,
                _pcre2_ucd_script_sets_8
                    .as_ptr()
                    .add(UCD_SCRIPTX_PROP(ucd) as usize) as *const c_void,
                UCD_MAPSIZE * size_of::<u32>(),
            );
            memset(
                map.as_mut_ptr().add(UCD_MAPSIZE) as *mut c_void,
                0,
                (FULL_MAPSIZE - UCD_MAPSIZE) * size_of::<u32>(),
            );
            if script != ucp_Common && script != ucp_Inherited {
                /* MAPSET(map, script) */
                map[(script / 32) as usize] |= 1u32 << (script % 32);
            }

            /* Handle the different checking states */

            match require_state {
                /* First significant character - it might follow Common or Inherited
                characters that do not have any script extensions. */
                SCRIPT_UNSET => match script {
                    ucp_Han => {
                        require_state = SCRIPT_HANPENDING;
                    }

                    ucp_Hiragana | ucp_Katakana => {
                        require_state = SCRIPT_HANHIRAKATA;
                    }

                    ucp_Bopomofo => {
                        require_state = SCRIPT_HANBOPOMOFO;
                    }

                    ucp_Hangul => {
                        require_state = SCRIPT_HANHANGUL;
                    }

                    _ => {
                        memcpy(
                            require_map.as_mut_ptr() as *mut c_void,
                            map.as_ptr() as *const c_void,
                            FULL_MAPSIZE * size_of::<u32>(),
                        );
                        require_state = SCRIPT_MAP;
                    }
                },

                /* The first significant character was Han. */
                SCRIPT_HANPENDING => {
                    if script != ucp_Han
                    /* Another Han does nothing */
                    {
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
                        } /* Not allowed with Han */

                        if chspecial == FOUND_BOPOMOFO {
                            require_state = SCRIPT_HANBOPOMOFO;
                        } else if chspecial == (FOUND_HIRAGANA | FOUND_KATAKANA) {
                            require_state = SCRIPT_HANHIRAKATA;
                        }

                        /* Otherwise this character must be allowed with all of them, so remain
                        in the pending state. */
                    }
                }

                /* Previously encountered one of the "with Han" scripts. Check that
                this character is appropriate. */
                SCRIPT_HANHIRAKATA => {
                    if MAPBIT(&map, ucp_Han)
                        .wrapping_add(MAPBIT(&map, ucp_Hiragana))
                        .wrapping_add(MAPBIT(&map, ucp_Katakana))
                        == 0
                    {
                        return FALSE;
                    }
                }

                SCRIPT_HANBOPOMOFO => {
                    if MAPBIT(&map, ucp_Han).wrapping_add(MAPBIT(&map, ucp_Bopomofo)) == 0 {
                        return FALSE;
                    }
                }

                SCRIPT_HANHANGUL => {
                    if MAPBIT(&map, ucp_Han).wrapping_add(MAPBIT(&map, ucp_Hangul)) == 0 {
                        return FALSE;
                    }
                }

                /* Previously encountered one or more characters that are allowed with a
                list of scripts. */
                SCRIPT_MAP => {
                    OK = FALSE;

                    {
                        let mut i: c_int = 0;
                        while (i as usize) < FULL_MAPSIZE {
                            if (require_map[i as usize] & map[i as usize]) != 0 {
                                OK = TRUE;
                                break;
                            }
                            i += 1;
                        }
                    }

                    if OK == FALSE {
                        return FALSE;
                    }

                    /* The rest of the string must be in this script, but we have to
                    allow for the Han complications. */

                    match script {
                        ucp_Han => {
                            require_state = SCRIPT_HANPENDING;
                        }

                        ucp_Hiragana | ucp_Katakana => {
                            require_state = SCRIPT_HANHIRAKATA;
                        }

                        ucp_Bopomofo => {
                            require_state = SCRIPT_HANBOPOMOFO;
                        }

                        ucp_Hangul => {
                            require_state = SCRIPT_HANHANGUL;
                        }

                        /* Compute the intersection of the required list of scripts and the
                        allowed scripts for this character. */
                        _ => {
                            let mut i: c_int = 0;
                            while (i as usize) < FULL_MAPSIZE {
                                require_map[i as usize] &= map[i as usize];
                                i += 1;
                            }
                        }
                    }
                }

                _ => {}
            }
        } /* End checking character's script and extensions. */

        /* The character is in an acceptable script. We must now ensure that all
        decimal digits in the string come from the same set. */

        if ucd.chartype as u32 == ucp_Nd {
            let mut digitset: u32 = 0;

            if c <= _pcre2_ucd_digit_sets_8[1] {
                digitset = 1;
            } else {
                let mut mid: c_int;
                let mut bot: c_int = 1;
                let mut top: c_int = _pcre2_ucd_digit_sets_8[0] as c_int;
                loop {
                    if top <= bot + 1
                    /* <= rather than == is paranoia */
                    {
                        digitset = top as u32;
                        break;
                    }
                    mid = (top + bot) / 2;
                    if c <= _pcre2_ucd_digit_sets_8[mid as usize] {
                        top = mid;
                    } else {
                        bot = mid;
                    }
                }
            }

            /* A required value of 0 means "unset". */

            if require_digitset == 0 {
                require_digitset = digitset;
            } else if digitset != require_digitset {
                return FALSE;
            }
        } /* End digit handling */

        /* If we haven't yet got to the end, pick up the next character. */

        if ptr >= endptr {
            return TRUE;
        }
        /* GETCHARINCTEST(c, ptr) */
        c = *ptr as u32;
        ptr = ptr.add(1);
        if utf != 0 && c >= 0xc0 {
            let r = getutf8inc(c, ptr);
            c = r.0;
            ptr = r.1;
        }
    } /* End checking loop */
}

/* End of pcre2_script_run.c */
