// Translated from c_src/src/pcre2_script_run.c
use crate::internal::*;

/*************************************************
*                Check script run                *
*************************************************/

/* A script run is conceptually a sequence of characters all in the same
Unicode script. However, it isn't quite that simple. There are special rules
for scripts that are commonly used together, and also special rules for digits.
This function implements the appropriate checks, which is possible only when
PCRE2 is compiled with Unicode support. The function returns TRUE if there is
no Unicode support; however, it should never be called in that circumstance
because an error is given by pcre2_compile() if a script run is called for in a
version of PCRE2 compiled without Unicode support.

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
pub unsafe extern "C" fn _pcre2_script_run_8(
    mut ptr: PCRE2_SPTR,
    endptr: PCRE2_SPTR,
    utf: BOOL,
) -> BOOL {
    let mut require_state: u32 = SCRIPT_UNSET;
    let mut require_map: [u32; FULL_MAPSIZE] = [0; FULL_MAPSIZE];
    let mut map: [u32; FULL_MAPSIZE] = [0; FULL_MAPSIZE];
    let mut require_digitset: u32 = 0;
    let mut c: u32;

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
        let mut i: c_int = 0;
        while (i as usize) < FULL_MAPSIZE {
            require_map[i as usize] = 0;
            i += 1;
        }
    }

    let require_map_p: *mut u32 = require_map.as_mut_ptr();
    let map_p: *mut u32 = map.as_mut_ptr();

    /* Scan strings of two or more characters, checking the Unicode characteristics
    of each code point. There is special code for scripts that can be combined with
    characters from the Han Chinese script. This may be used in conjunction with
    four other scripts in these combinations:

    . Han with Hiragana and Katakana is allowed (for Japanese).
    . Han with Bopomofo is allowed (for Taiwanese Mandarin).
    . Han with Hangul is allowed (for Korean).

    If the first significant character's script is one of the four, the required
    script type is immediately known. However, if the first significant
    character's script is Han, we have to keep checking for a non-Han character.
    Hence the SCRIPT_HANPENDING state. */

    loop {
        let ucd: *const ucd_record = GET_UCD(c);
        let script: u32 = (*ucd).script as u32;

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
            scripts. Copy the scriptx map for this character (which covers those
            scripts that appear in script extension lists), set the remaining values to
            zero, and then, except for Common or Inherited, add this script's bit to
            the map. */

            memcpy(
                map_p as *mut c_void,
                _pcre2_ucd_script_sets_8
                    .as_ptr()
                    .add(UCD_SCRIPTX_PROP(ucd) as usize) as *const c_void,
                UCD_MAPSIZE * size_of::<u32>(),
            );
            memset(
                map_p.add(UCD_MAPSIZE) as *mut c_void,
                0,
                (FULL_MAPSIZE - UCD_MAPSIZE) * size_of::<u32>(),
            );
            if script != ucp_Common && script != ucp_Inherited {
                MAPSET!(map_p, script);
            }

            /* Handle the different checking states */

            if require_state == SCRIPT_UNSET {
                /* First significant character - it might follow Common or Inherited
                characters that do not have any script extensions. */

                if script == ucp_Han {
                    require_state = SCRIPT_HANPENDING;
                } else if script == ucp_Hiragana || script == ucp_Katakana {
                    require_state = SCRIPT_HANHIRAKATA;
                } else if script == ucp_Bopomofo {
                    require_state = SCRIPT_HANBOPOMOFO;
                } else if script == ucp_Hangul {
                    require_state = SCRIPT_HANHANGUL;
                } else {
                    memcpy(
                        require_map_p as *mut c_void,
                        map_p as *const c_void,
                        FULL_MAPSIZE * size_of::<u32>(),
                    );
                    require_state = SCRIPT_MAP;
                }
            }
            /* The first significant character was Han. An inspection of the Unicode
            11.0.0 files shows that there are the following types of Script Extension
            list that involve the Han, Bopomofo, Hiragana, Katakana, and Hangul
            scripts:

            . Bopomofo + Han
            . Han + Hiragana + Katakana
            . Hiragana + Katakana
            . Bopopmofo + Hangul + Han + Hiragana + Katakana

            The following code tries to make sense of this. */
            else if require_state == SCRIPT_HANPENDING {
                if script != ucp_Han
                /* Another Han does nothing */
                {
                    let mut chspecial: u32 = 0;

                    if MAPBIT!(map_p, ucp_Bopomofo) != 0 {
                        chspecial |= FOUND_BOPOMOFO;
                    }
                    if MAPBIT!(map_p, ucp_Hiragana) != 0 {
                        chspecial |= FOUND_HIRAGANA;
                    }
                    if MAPBIT!(map_p, ucp_Katakana) != 0 {
                        chspecial |= FOUND_KATAKANA;
                    }
                    if MAPBIT!(map_p, ucp_Hangul) != 0 {
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
            else if require_state == SCRIPT_HANHIRAKATA {
                if MAPBIT!(map_p, ucp_Han) + MAPBIT!(map_p, ucp_Hiragana)
                    + MAPBIT!(map_p, ucp_Katakana)
                    == 0
                {
                    return FALSE;
                }
            } else if require_state == SCRIPT_HANBOPOMOFO {
                if MAPBIT!(map_p, ucp_Han) + MAPBIT!(map_p, ucp_Bopomofo) == 0 {
                    return FALSE;
                }
            } else if require_state == SCRIPT_HANHANGUL {
                if MAPBIT!(map_p, ucp_Han) + MAPBIT!(map_p, ucp_Hangul) == 0 {
                    return FALSE;
                }
            }
            /* Previously encountered one or more characters that are allowed with a
            list of scripts. */
            else if require_state == SCRIPT_MAP {
                OK = FALSE;

                {
                    let mut i: c_int = 0;
                    while (i as usize) < FULL_MAPSIZE {
                        if (*require_map_p.add(i as usize) & *map_p.add(i as usize)) != 0 {
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

                if script == ucp_Han {
                    require_state = SCRIPT_HANPENDING;
                } else if script == ucp_Hiragana || script == ucp_Katakana {
                    require_state = SCRIPT_HANHIRAKATA;
                } else if script == ucp_Bopomofo {
                    require_state = SCRIPT_HANBOPOMOFO;
                } else if script == ucp_Hangul {
                    require_state = SCRIPT_HANHANGUL;
                } else {
                    /* Compute the intersection of the required list of scripts and the
                    allowed scripts for this character. */

                    let mut i: c_int = 0;
                    while (i as usize) < FULL_MAPSIZE {
                        *require_map_p.add(i as usize) &= *map_p.add(i as usize);
                        i += 1;
                    }
                }
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
            let digitset: u32;

            if c <= *_pcre2_ucd_digit_sets_8.as_ptr().add(1) {
                digitset = 1;
            } else {
                let mut mid: c_int;
                let mut bot: c_int = 1;
                let mut top: c_int = *_pcre2_ucd_digit_sets_8.as_ptr().add(0) as c_int;
                loop {
                    if top <= bot + 1
                    /* <= rather than == is paranoia */
                    {
                        digitset = top as u32;
                        break;
                    }
                    mid = (top + bot) / 2;
                    if c <= *_pcre2_ucd_digit_sets_8.as_ptr().add(mid as usize) {
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

/* End of pcre2_script_run.c */
