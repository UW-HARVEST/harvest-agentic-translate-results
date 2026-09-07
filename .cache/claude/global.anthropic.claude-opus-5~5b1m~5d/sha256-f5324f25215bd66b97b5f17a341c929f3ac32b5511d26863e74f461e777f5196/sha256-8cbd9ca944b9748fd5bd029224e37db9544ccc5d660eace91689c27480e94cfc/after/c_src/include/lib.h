typedef struct os_data {
    char *os_name;
    char *os_version;
    char *os_major;
    char *os_minor;
    char *os_codename;
    char *os_platform;
    char *os_build;
    char *os_uname;
    char *os_arch;
} os_data;

void parse_uname_string (char *uname, os_data *osd);
