#include "lib.h"

#include <stdlib.h>
#include <string.h>
#include <regex.h>
#include <stdio.h>

/**
 * @brief Looks for the OS architecture in a string. Possibles architectures
 *        are x86_64, i386, i686, sparc, amd64, ia64, AIX, armv6, armv7.
 *        The function will return a pointer to allocated memory that must
 *        be de-allocated by the caller.
 *
 * @param[in] os_header String that contains the architecture. Usually uname.
 * @retval A string pointer to the architecture. NULL if not found.
 */
char * get_os_arch(char * os_header) {
    const char * ARCHS[] = { "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7", "aarch64", "arm64", NULL };
    char * os_arch = NULL;
    int i;

    for (i = 0; ARCHS[i]; i++) {
        if (strstr(os_header, ARCHS[i])) {
            os_arch = strdup(ARCHS[i]);
            break;
        }
    }

    return os_arch;
}

int w_regexec(const char * pattern, const char * string, size_t nmatch, regmatch_t * pmatch) {
    regex_t regex;
    int result;

    if (!(pattern && string)) {
        return 0;
    }

    if (regcomp(&regex, pattern, REG_EXTENDED)) {
        fprintf(stderr, "Couldn't compile regular expression '%s'\n", pattern);
        return 0;
    }

    result = regexec(&regex, string, nmatch, pmatch, 0);
    regfree(&regex);
    return !result;
}

/**
 * @brief Parses an OS uname string. All the OUT parameters are pointers
 *        to allocated memory that must be de-allocated by the caller.
 *
 * @param[in] msg The agent update message string to be parsed.
 * @param[in] osd An os_data structure to be filled with the os's data.
 */
void parse_uname_string (char *uname,
                         os_data *osd)
{
    char *str_tmp = NULL;
    regmatch_t match[2] = {{.rm_so = 0}};
    int match_size = 0;
    
    if (!osd)
        return;

    // [Ver: os_major.os_minor.os_build]
    if (str_tmp = strstr(uname, " [Ver: "), str_tmp) {
        *str_tmp = '\0';
        str_tmp += 7;
        osd->os_name = strdup(uname);
        *(str_tmp + strlen(str_tmp) - 1) = '\0';

        // Get os_major
        if (w_regexec("^([0-9]+)\\.*", str_tmp, 2, match)) {
            match_size = match[1].rm_eo - match[1].rm_so;
            osd->os_major = malloc(match_size +1);
            snprintf (osd->os_major, match_size + 1, "%.*s", match_size, str_tmp + match[1].rm_so);
        }

        // Get os_minor
        if (w_regexec("^[0-9]+\\.([0-9]+)\\.*", str_tmp, 2, match)) {
            match_size = match[1].rm_eo - match[1].rm_so;
            osd->os_minor = malloc(match_size +1);
            snprintf(osd->os_minor, match_size + 1, "%.*s", match_size, str_tmp + match[1].rm_so);
        }

        // Get os_build
        if (w_regexec("^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", str_tmp, 2, match)) {
            match_size = match[1].rm_eo - match[1].rm_so;
            osd->os_build = malloc(match_size +1);
            snprintf(osd->os_build, match_size + 1, "%.*s", match_size, str_tmp + match[1].rm_so);
        }

        osd->os_version = strdup(str_tmp);
        osd->os_platform = strdup("windows");
    } else {
        if (str_tmp = strstr(uname, " ["), str_tmp) {
            *str_tmp = '\0';
            str_tmp += 2;
            osd->os_name = strdup(str_tmp);
            if (str_tmp = strstr(osd->os_name, ": "), str_tmp) {
                *str_tmp = '\0';
                str_tmp += 2;
                osd->os_version = strdup(str_tmp);
                *(osd->os_version + strlen(osd->os_version) - 1) = '\0';

                // os_major.os_minor (os_codename)
                if (str_tmp = strstr(osd->os_version, " ("), str_tmp) {
                    *str_tmp = '\0';
                    str_tmp += 2;
                    osd->os_codename = strdup(str_tmp);
                    *(osd->os_codename + strlen(osd->os_codename) - 1) = '\0';
                }

                // Get os_major
                if (w_regexec("^([0-9]+)\\.*", osd->os_version, 2, match)) {
                    match_size = match[1].rm_eo - match[1].rm_so;
                    osd->os_major = malloc(match_size +1);
                    snprintf(osd->os_major, match_size + 1, "%.*s", match_size, osd->os_version + match[1].rm_so);
                }

                // Get os_minor
                if (w_regexec("^[0-9]+\\.([0-9]+)\\.*", osd->os_version, 2, match)) {
                    match_size = match[1].rm_eo - match[1].rm_so;
                    osd->os_minor = malloc(match_size +1);
                    snprintf(osd->os_minor, match_size + 1, "%.*s", match_size, osd->os_version + match[1].rm_so);
                }

            } else {
                *(osd->os_name + strlen(osd->os_name) - 1) = '\0';
            }

            // os_name|os_platform
            if (str_tmp = strstr(osd->os_name, "|"), str_tmp) {
                *str_tmp = '\0';
                str_tmp++;
                osd->os_platform = strdup(str_tmp);
            }
        }

        if (str_tmp = get_os_arch(uname), str_tmp) {
            osd->os_arch = strdup(str_tmp);
            free(str_tmp);
        }
    }
}
