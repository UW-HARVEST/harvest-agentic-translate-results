//! Translated from pcre2_script_run.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

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

const UCD_MAPSIZE: usize = (ucp_Unknown as usize) / 32 + 1;
const FULL_MAPSIZE: usize = (ucp_Script_Count as usize) / 32 + 1;

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
    let mut c: u32 = 0;

    /* Any string containing fewer than 2 characters is a valid script run. */

    if ptr >= endptr {
        return TRUE;
    }
    GETCHARINCTEST!(c, ptr, utf);
    if ptr >= endptr {
        return TRUE;
    }

    /* Initialize the require map. This is a full-size bitmap that has a bit for
    every script, as opposed to the maps in ucd_script_sets, which only have bits
    for scripts less than ucp_Unknown - those that appear in script extension
    lists. */

    {
        let mut i: i32 = 0;
        while i < FULL_MAPSIZE as i32 {
            require_map[i as usize] = 0;
            i += 1;
        }
    }

    /* Scan strings of two or more characters, checking the Unicode characteristics
    of each code point. There is special code for scripts that can be combined with
    characters from the Han Chinese script. */

    loop {
        let ucd: *const ucd_record = GET_UCD!(c);
        let script: u32 = (*ucd).script as u32;

        /* If the script is Unknown, the string is not a valid script run. Such
        characters can only form script runs of length one (see test above). */

        if script == ucp_Unknown {
            return FALSE;
        }

        /* A character without any script extensions whose script is Inherited or
        Common is always accepted with any script. If there are extensions, the
        following processing happens for all scripts. */

        if UCD_SCRIPTX_PROP!(ucd) != 0 || (script != ucp_Inherited && script != ucp_Common) {
            let mut OK: BOOL;

            /* Set up a full-sized map for this character that can include bits for all
            scripts. Copy the scriptx map for this character (which covers those
            scripts that appear in script extension lists), set the remaining values to
            zero, and then, except for Common or Inherited, add this script's bit to
            the map. */

            core::ptr::copy_nonoverlapping(
                crate::ucd::_pcre2_ucd_script_sets_8
                    .as_ptr()
                    .add(UCD_SCRIPTX_PROP!(ucd) as usize) as *const u8,
                map.as_mut_ptr() as *mut u8,
                UCD_MAPSIZE * core::mem::size_of::<u32>(),
            );
            core::ptr::write_bytes(
                map.as_mut_ptr().add(UCD_MAPSIZE) as *mut u8,
                0u8,
                (FULL_MAPSIZE - UCD_MAPSIZE) * core::mem::size_of::<u32>(),
            );
            if script != ucp_Common && script != ucp_Inherited {
                MAPSET!(map.as_mut_ptr(), script);
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
                        core::ptr::copy_nonoverlapping(
                            map.as_ptr() as *const u8,
                            require_map.as_mut_ptr() as *mut u8,
                            FULL_MAPSIZE * core::mem::size_of::<u32>(),
                        );
                        require_state = SCRIPT_MAP;
                    }
                },

                /* The first significant character was Han. An inspection of the Unicode
                11.0.0 files shows that there are the following types of Script Extension
                list that involve the Han, Bopomofo, Hiragana, Katakana, and Hangul
                scripts:

                . Bopomofo + Han
                . Han + Hiragana + Katakana
                . Hiragana + Katakana
                . Bopopmofo + Hangul + Han + Hiragana + Katakana

                The following code tries to make sense of this. */
                SCRIPT_HANPENDING => {
                    if script != ucp_Han
                    /* Another Han does nothing */
                    {
                        let mut chspecial: u32 = 0;

                        if MAPBIT!(map.as_ptr(), ucp_Bopomofo) != 0 {
                            chspecial |= FOUND_BOPOMOFO;
                        }
                        if MAPBIT!(map.as_ptr(), ucp_Hiragana) != 0 {
                            chspecial |= FOUND_HIRAGANA;
                        }
                        if MAPBIT!(map.as_ptr(), ucp_Katakana) != 0 {
                            chspecial |= FOUND_KATAKANA;
                        }
                        if MAPBIT!(map.as_ptr(), ucp_Hangul) != 0 {
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

                        /* Otherwise this character must be allowed with all of them, so remain
                        in the pending state. */
                    }
                }

                /* Previously encountered one of the "with Han" scripts. Check that
                this character is appropriate. */
                SCRIPT_HANHIRAKATA => {
                    if MAPBIT!(map.as_ptr(), ucp_Han)
                        + MAPBIT!(map.as_ptr(), ucp_Hiragana)
                        + MAPBIT!(map.as_ptr(), ucp_Katakana)
                        == 0
                    {
                        return FALSE;
                    }
                }

                SCRIPT_HANBOPOMOFO => {
                    if MAPBIT!(map.as_ptr(), ucp_Han) + MAPBIT!(map.as_ptr(), ucp_Bopomofo) == 0 {
                        return FALSE;
                    }
                }

                SCRIPT_HANHANGUL => {
                    if MAPBIT!(map.as_ptr(), ucp_Han) + MAPBIT!(map.as_ptr(), ucp_Hangul) == 0 {
                        return FALSE;
                    }
                }

                /* Previously encountered one or more characters that are allowed with a
                list of scripts. */
                SCRIPT_MAP => {
                    OK = FALSE;

                    let mut i: i32 = 0;
                    while i < FULL_MAPSIZE as i32 {
                        if (require_map[i as usize] & map[i as usize]) != 0 {
                            OK = TRUE;
                            break;
                        }
                        i += 1;
                    }

                    if OK == 0 {
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
                            let mut i: i32 = 0;
                            while i < FULL_MAPSIZE as i32 {
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
        decimal digits in the string come from the same set. Some scripts (e.g.
        Common, Arabic) have more than one set of decimal digits. This code does
        not allow mixing sets, even within the same script. The vector called
        PRIV(ucd_digit_sets)[] contains, in its first element, the number of
        following elements, and then, in ascending order, the code points of the
        '9' characters in every set of 10 digits. Each set is identified by the
        offset in the vector of its '9' character. An initial check of the first
        value picks up ASCII digits quickly. Otherwise, a binary chop is used. */

        if (*ucd).chartype as u32 == ucp_Nd {
            let mut digitset: u32;

            if c <= crate::ucd::_pcre2_ucd_digit_sets_8[1] {
                digitset = 1;
            } else {
                let mut mid: i32;
                let mut bot: i32 = 1;
                let mut top: i32 = crate::ucd::_pcre2_ucd_digit_sets_8[0] as i32;
                loop {
                    if top <= bot + 1
                    /* <= rather than == is paranoia */
                    {
                        digitset = top as u32;
                        break;
                    }
                    mid = (top + bot) / 2;
                    if c <= crate::ucd::_pcre2_ucd_digit_sets_8[mid as usize] {
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
        GETCHARINCTEST!(c, ptr, utf);
    } /* End checking loop */
}
