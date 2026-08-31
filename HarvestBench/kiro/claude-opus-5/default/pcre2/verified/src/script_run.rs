//! Translation of `c_src/src/pcre2_script_run.c`.
//!
//! Contains the function for checking a script run. `SUPPORT_UNICODE` is
//! defined, so the real implementation is translated.

#![allow(non_snake_case, non_upper_case_globals)]

use crate::internal::*;
use crate::ucp::*;

/* These are states in the checking process. */

const SCRIPT_UNSET: u32 = 0; /* Requirement as yet unknown */
const SCRIPT_MAP: u32 = 1; /* Bitmap contains acceptable scripts */
const SCRIPT_HANPENDING: u32 = 2; /* Have had only Han characters */
const SCRIPT_HANHIRAKATA: u32 = 3; /* Expect Han or Hirikata */
const SCRIPT_HANBOPOMOFO: u32 = 4; /* Expect Han or Bopomofo */
const SCRIPT_HANHANGUL: u32 = 5; /* Expect Han or Hangul */

const UCD_MAPSIZE: usize = (ucp_Unknown as usize) / 32 + 1;
const FULL_MAPSIZE: usize = (ucp_Script_Count as usize) / 32 + 1;

const FOUND_BOPOMOFO: u32 = 1;
const FOUND_HIRAGANA: u32 = 2;
const FOUND_KATAKANA: u32 = 4;
const FOUND_HANGUL: u32 = 8;

/// `PRIV(script_run)`
///
/// Check a script run.
///
/// Arguments:
/// * `ptr`    - point to the first character
/// * `endptr` - point after the last character
/// * `utf`    - TRUE if in UTF mode
///
/// Returns: TRUE if this is a valid script run.
pub unsafe fn script_run(mut ptr: PCRE2_SPTR, endptr: PCRE2_SPTR, utf: BOOL) -> BOOL {
    unsafe {
        let mut require_state: u32 = SCRIPT_UNSET;
        let mut require_map: [u32; FULL_MAPSIZE] = [0; FULL_MAPSIZE];
        let mut map: [u32; FULL_MAPSIZE] = [0; FULL_MAPSIZE];
        let mut require_digitset: u32 = 0;
        let mut c: u32;

        /* Any string containing fewer than 2 characters is a valid script run. */

        if ptr >= endptr {
            return TRUE;
        }
        c = getcharinctest(&mut ptr, utf != FALSE);
        if ptr >= endptr {
            return TRUE;
        }

        /* Initialize the require map. This is a full-size bitmap that has a bit for
        every script, as opposed to the maps in ucd_script_sets, which only have bits
        for scripts less than ucp_Unknown - those that appear in script extension
        lists. */

        for i in 0..FULL_MAPSIZE {
            require_map[i] = 0;
        }

        /* Scan strings of two or more characters, checking the Unicode
        characteristics of each code point. */

        loop {
            let ucd: &UcdRecord = get_ucd(c);
            let script: u32 = ucd.script as u32;

            /* If the script is Unknown, the string is not a valid script run. Such
            characters can only form script runs of length one (see test above). */

            if script == ucp_Unknown {
                return FALSE;
            }

            /* A character without any script extensions whose script is Inherited or
            Common is always accepted with any script. If there are extensions, the
            following processing happens for all scripts. */

            if ucd_scriptx_prop(ucd) != 0
                || (script != ucp_Inherited && script != ucp_Common)
            {
                let mut OK: BOOL;

                /* Set up a full-sized map for this character that can include bits for
                all scripts. Copy the scriptx map for this character (which covers those
                scripts that appear in script extension lists), set the remaining values
                to zero, and then, except for Common or Inherited, add this script's bit
                to the map. */

                memcpy(
                    map.as_mut_ptr(),
                    UCD_SCRIPT_SETS.as_ptr().add(ucd_scriptx_prop(ucd) as usize),
                    UCD_MAPSIZE,
                );
                memset(
                    map.as_mut_ptr().add(UCD_MAPSIZE) as *mut u8,
                    0,
                    (FULL_MAPSIZE - UCD_MAPSIZE) * core::mem::size_of::<u32>(),
                );
                if script != ucp_Common && script != ucp_Inherited {
                    mapset(&mut map, script);
                }

                /* Handle the different checking states */

                match require_state {
                    /* First significant character - it might follow Common or Inherited
                    characters that do not have any script extensions. */
                    SCRIPT_UNSET => match script {
                        s if s == ucp_Han => {
                            require_state = SCRIPT_HANPENDING;
                        }
                        s if s == ucp_Hiragana || s == ucp_Katakana => {
                            require_state = SCRIPT_HANHIRAKATA;
                        }
                        s if s == ucp_Bopomofo => {
                            require_state = SCRIPT_HANBOPOMOFO;
                        }
                        s if s == ucp_Hangul => {
                            require_state = SCRIPT_HANHANGUL;
                        }
                        _ => {
                            memcpy(require_map.as_mut_ptr(), map.as_ptr(), FULL_MAPSIZE);
                            require_state = SCRIPT_MAP;
                        }
                    },

                    /* The first significant character was Han. */
                    SCRIPT_HANPENDING => {
                        if script != ucp_Han
                        /* Another Han does nothing */
                        {
                            let mut chspecial: u32 = 0;

                            if mapbit(&map, ucp_Bopomofo) != 0 {
                                chspecial |= FOUND_BOPOMOFO;
                            }
                            if mapbit(&map, ucp_Hiragana) != 0 {
                                chspecial |= FOUND_HIRAGANA;
                            }
                            if mapbit(&map, ucp_Katakana) != 0 {
                                chspecial |= FOUND_KATAKANA;
                            }
                            if mapbit(&map, ucp_Hangul) != 0 {
                                chspecial |= FOUND_HANGUL;
                            }

                            if chspecial == 0 {
                                return FALSE; /* Not allowed with Han */
                            }

                            if chspecial == FOUND_BOPOMOFO {
                                require_state = SCRIPT_HANBOPOMOFO;
                            } else if chspecial == (FOUND_HIRAGANA | FOUND_KATAKANA) {
                                require_state = SCRIPT_HANHIRAKATA;
                            }

                            /* Otherwise this character must be allowed with all of them,
                            so remain in the pending state. */
                        }
                    }

                    /* Previously encountered one of the "with Han" scripts. Check that
                    this character is appropriate. */
                    SCRIPT_HANHIRAKATA => {
                        if mapbit(&map, ucp_Han)
                            + mapbit(&map, ucp_Hiragana)
                            + mapbit(&map, ucp_Katakana)
                            == 0
                        {
                            return FALSE;
                        }
                    }

                    SCRIPT_HANBOPOMOFO => {
                        if mapbit(&map, ucp_Han) + mapbit(&map, ucp_Bopomofo) == 0 {
                            return FALSE;
                        }
                    }

                    SCRIPT_HANHANGUL => {
                        if mapbit(&map, ucp_Han) + mapbit(&map, ucp_Hangul) == 0 {
                            return FALSE;
                        }
                    }

                    /* Previously encountered one or more characters that are allowed
                    with a list of scripts. */
                    SCRIPT_MAP => {
                        OK = FALSE;

                        for i in 0..FULL_MAPSIZE {
                            if (require_map[i] & map[i]) != 0 {
                                OK = TRUE;
                                break;
                            }
                        }

                        if OK == FALSE {
                            return FALSE;
                        }

                        /* The rest of the string must be in this script, but we have to
                        allow for the Han complications. */

                        match script {
                            s if s == ucp_Han => {
                                require_state = SCRIPT_HANPENDING;
                            }
                            s if s == ucp_Hiragana || s == ucp_Katakana => {
                                require_state = SCRIPT_HANHIRAKATA;
                            }
                            s if s == ucp_Bopomofo => {
                                require_state = SCRIPT_HANBOPOMOFO;
                            }
                            s if s == ucp_Hangul => {
                                require_state = SCRIPT_HANHANGUL;
                            }
                            /* Compute the intersection of the required list of scripts
                            and the allowed scripts for this character. */
                            _ => {
                                for i in 0..FULL_MAPSIZE {
                                    require_map[i] &= map[i];
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
                let digitset: u32;

                if c <= UCD_DIGIT_SETS[1] {
                    digitset = 1;
                } else {
                    let mut mid: i32;
                    let mut bot: i32 = 1;
                    let mut top: i32 = UCD_DIGIT_SETS[0] as i32;
                    loop {
                        if top <= bot + 1
                        /* <= rather than == is paranoia */
                        {
                            digitset = top as u32;
                            break;
                        }
                        mid = (top + bot) / 2;
                        if c <= UCD_DIGIT_SETS[mid as usize] {
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
            c = getcharinctest(&mut ptr, utf != FALSE);
        } /* End checking loop */
    }
}

/// Exported as `_pcre2_script_run_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_script_run_8(
    ptr: PCRE2_SPTR,
    endptr: PCRE2_SPTR,
    utf: BOOL,
) -> BOOL {
    unsafe { script_run(ptr, endptr, utf) }
}
