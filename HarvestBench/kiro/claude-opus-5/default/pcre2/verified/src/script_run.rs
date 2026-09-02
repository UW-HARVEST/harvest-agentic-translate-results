//! Translation of `pcre2_script_run.c`.
//!
//! This module contains the function for checking a script run.

use crate::internal::*;
use core::ffi::c_int;

// These are states in the checking process.
const SCRIPT_UNSET: u32 = 0; // Requirement as yet unknown
const SCRIPT_MAP: u32 = 1; // Bitmap contains acceptable scripts
const SCRIPT_HANPENDING: u32 = 2; // Have had only Han characters
const SCRIPT_HANHIRAKATA: u32 = 3; // Expect Han or Hirikata
const SCRIPT_HANBOPOMOFO: u32 = 4; // Expect Han or Bopomofo
const SCRIPT_HANHANGUL: u32 = 5; // Expect Han or Hangul

const UCD_MAPSIZE: usize = (ucp_Unknown as usize) / 32 + 1;
const FULL_MAPSIZE: usize = (ucp_Script_Count as usize) / 32 + 1;

// These mirror the local #defines used inside the SCRIPT_HANPENDING case.
const FOUND_BOPOMOFO: u32 = 1;
const FOUND_HIRAGANA: u32 = 2;
const FOUND_KATAKANA: u32 = 4;
const FOUND_HANGUL: u32 = 8;

/// `MAPSET(map, x)` — set bit `x` in a 32-bit-word bitmap.
#[inline(always)]
unsafe fn MAPSET(map: *mut u32, x: u32) {
    unsafe {
        *map.add((x as usize) / 32) |= 1u32 << ((x as usize) % 32);
    }
}

/// `PRIV(script_run)` — return TRUE if this is a valid script run.
///
/// Arguments:
///   ptr       point to the first character
///   endptr    point after the last character
///   utf       TRUE if in UTF mode
///
/// Returns:    TRUE if this is a valid script run
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_script_run_8(
    ptr: PCRE2_SPTR,
    endptr: PCRE2_SPTR,
    utf: BOOL,
) -> BOOL {
    unsafe {
        // SUPPORT_UNICODE is defined in this configuration.
        let mut require_state: u32 = SCRIPT_UNSET;
        let mut require_map = [0u32; FULL_MAPSIZE];
        let mut map = [0u32; FULL_MAPSIZE];
        let mut require_digitset: u32 = 0;
        let mut c: u32;

        let utf = utf != FALSE;

        let mut ptr = ptr;

        // Any string containing fewer than 2 characters is a valid script run.
        if ptr >= endptr {
            return TRUE;
        }
        c = GETCHARINCTEST(&mut ptr, utf);
        if ptr >= endptr {
            return TRUE;
        }

        // Initialize the require map. This is a full-size bitmap that has a bit
        // for every script.
        for i in 0..FULL_MAPSIZE {
            require_map[i] = 0;
        }

        // Scan strings of two or more characters.
        loop {
            let ucd: &UcdRecord = GET_UCD(c);
            let script = ucd.script as u32;

            // If the script is Unknown, the string is not a valid script run.
            if script == ucp_Unknown {
                return FALSE;
            }

            // A character without any script extensions whose script is
            // Inherited or Common is always accepted with any script.
            if UCD_SCRIPTX_PROP(ucd) != 0
                || (script != ucp_Inherited && script != ucp_Common)
            {
                let mut ok: BOOL;

                // Set up a full-sized map for this character.
                let src = (crate::tables::_pcre2_ucd_script_sets)
                    .as_ptr()
                    .add(UCD_SCRIPTX_PROP(ucd) as usize);
                core::ptr::copy_nonoverlapping(src, map.as_mut_ptr(), UCD_MAPSIZE);
                for i in UCD_MAPSIZE..FULL_MAPSIZE {
                    map[i] = 0;
                }
                if script != ucp_Common && script != ucp_Inherited {
                    MAPSET(map.as_mut_ptr(), script);
                }

                // Handle the different checking states.
                match require_state {
                    // First significant character.
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
                            require_map.copy_from_slice(&map);
                            require_state = SCRIPT_MAP;
                        }
                    },

                    // The first significant character was Han.
                    SCRIPT_HANPENDING => {
                        if script != ucp_Han {
                            // Another Han does nothing
                            let mut chspecial: u32 = 0;

                            if MAPBIT(map.as_ptr(), ucp_Bopomofo) != 0 {
                                chspecial |= FOUND_BOPOMOFO;
                            }
                            if MAPBIT(map.as_ptr(), ucp_Hiragana) != 0 {
                                chspecial |= FOUND_HIRAGANA;
                            }
                            if MAPBIT(map.as_ptr(), ucp_Katakana) != 0 {
                                chspecial |= FOUND_KATAKANA;
                            }
                            if MAPBIT(map.as_ptr(), ucp_Hangul) != 0 {
                                chspecial |= FOUND_HANGUL;
                            }

                            if chspecial == 0 {
                                return FALSE; // Not allowed with Han
                            }

                            if chspecial == FOUND_BOPOMOFO {
                                require_state = SCRIPT_HANBOPOMOFO;
                            } else if chspecial == (FOUND_HIRAGANA | FOUND_KATAKANA) {
                                require_state = SCRIPT_HANHIRAKATA;
                            }

                            // Otherwise remain in the pending state.
                        }
                    }

                    // Previously encountered one of the "with Han" scripts.
                    SCRIPT_HANHIRAKATA => {
                        if MAPBIT(map.as_ptr(), ucp_Han)
                            + MAPBIT(map.as_ptr(), ucp_Hiragana)
                            + MAPBIT(map.as_ptr(), ucp_Katakana)
                            == 0
                        {
                            return FALSE;
                        }
                    }

                    SCRIPT_HANBOPOMOFO => {
                        if MAPBIT(map.as_ptr(), ucp_Han) + MAPBIT(map.as_ptr(), ucp_Bopomofo)
                            == 0
                        {
                            return FALSE;
                        }
                    }

                    SCRIPT_HANHANGUL => {
                        if MAPBIT(map.as_ptr(), ucp_Han) + MAPBIT(map.as_ptr(), ucp_Hangul)
                            == 0
                        {
                            return FALSE;
                        }
                    }

                    // Previously encountered one or more characters that are
                    // allowed with a list of scripts.
                    SCRIPT_MAP => {
                        ok = FALSE;

                        for i in 0..FULL_MAPSIZE {
                            if (require_map[i] & map[i]) != 0 {
                                ok = TRUE;
                                break;
                            }
                        }

                        if ok == FALSE {
                            return FALSE;
                        }

                        // The rest of the string must be in this script, but we
                        // have to allow for the Han complications.
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
                            // Compute the intersection of the required list of
                            // scripts and the allowed scripts for this character.
                            _ => {
                                for i in 0..FULL_MAPSIZE {
                                    require_map[i] &= map[i];
                                }
                            }
                        }
                    }

                    _ => {}
                }
            } // End checking character's script and extensions.

            // The character is in an acceptable script. Ensure all decimal
            // digits in the string come from the same set.
            if ucd.chartype as u32 == ucp_Nd {
                let digitset: u32;

                let digit_sets = &crate::tables::_pcre2_ucd_digit_sets;

                if c <= digit_sets[1] {
                    digitset = 1;
                } else {
                    let mut mid: i32;
                    let mut bot: i32 = 1;
                    let mut top: i32 = digit_sets[0] as i32;
                    loop {
                        if top <= bot + 1 {
                            // <= rather than == is paranoia
                            digitset = top as u32;
                            break;
                        }
                        mid = (top + bot) / 2;
                        if c <= digit_sets[mid as usize] {
                            top = mid;
                        } else {
                            bot = mid;
                        }
                    }
                }

                // A required value of 0 means "unset".
                if require_digitset == 0 {
                    require_digitset = digitset;
                } else if digitset != require_digitset {
                    return FALSE;
                }
            } // End digit handling

            // If we haven't yet got to the end, pick up the next character.
            if ptr >= endptr {
                return TRUE;
            }
            c = GETCHARINCTEST(&mut ptr, utf);
        } // End checking loop
    }
}

// Silence unused-import lints for c_int if not otherwise referenced.
#[allow(unused_imports)]
use c_int as _;
