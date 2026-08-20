/* Run the C driver with a *controlled* `main` stack frame.
 *
 * The C code reads the uninitialised part of its own stack frame (it calls
 * strcmp/strlen on buffers that need not be NUL terminated).  Those bytes are
 * left over from the dynamic loader, so they depend on ASLR, on the size of the
 * environment and on the length of the path the program was started with -
 * i.e. they are not reproducible across environments.
 *
 * To make the executable level differential test independent of that, this
 * helper stops the driver at the entry of `process_strings` and overwrites every
 * byte of the frame that `main` did *not* initialise with the snapshot the Rust
 * translation uses (`src/frame_junk.rs`).  Afterwards both programs read exactly
 * the same bytes and any difference in their output is a real difference in the
 * translated logic.
 *
 * usage: inject_frame <driver> <process_strings-addr-hex> <junk-file> <stdin-file>
 *
 * The child's stdout/stderr are inherited, the helper itself prints nothing and
 * exits with the child's exit code (or 128+signal if the child was killed).
 */
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/resource.h>
#include <sys/user.h>
#include <sys/wait.h>
#include <unistd.h>

/* modelled frame offsets, see src/mem.rs */
#define BUF_SIZE 1024
#define REF_OFF 0
#define INPUT_OFF 1024
#define LOCALS_OFF 2048
/* everything from `saved rbp` upwards must stay untouched or the child cannot
 * return from main any more */
#define MODEL_END 2096

static int peek_poke_range(pid_t pid, unsigned long base, const unsigned char *junk,
                           long from, long to) {
    for (long off = from & ~7L; off < to; off += 8) {
        errno = 0;
        long word = ptrace(PTRACE_PEEKDATA, pid, (void *)(base + off), NULL);
        if (errno != 0) {
            return -1;
        }
        unsigned char b[8];
        memcpy(b, &word, 8);
        for (int i = 0; i < 8; i++) {
            long o = off + i;
            if (o >= from && o < to) {
                b[i] = junk[o];
            }
        }
        memcpy(&word, b, 8);
        if (ptrace(PTRACE_POKEDATA, pid, (void *)(base + off), (void *)word) != 0) {
            return -1;
        }
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 5) {
        fprintf(stderr, "usage: %s driver bp_hex junk_file stdin_file\n", argv[0]);
        return 2;
    }
    const char *driver = argv[1];
    unsigned long bp = strtoul(argv[2], NULL, 16);
    const char *junk_file = argv[3];
    const char *in_file = argv[4];

    unsigned char junk[MODEL_END];
    FILE *jf = fopen(junk_file, "rb");
    if (jf == NULL || fread(junk, 1, MODEL_END, jf) != MODEL_END) {
        fprintf(stderr, "cannot read %d junk bytes from %s\n", MODEL_END, junk_file);
        return 2;
    }
    fclose(jf);

    pid_t pid = fork();
    if (pid == 0) {
        struct rlimit no_core = {0, 0};
        setrlimit(RLIMIT_CORE, &no_core);
        if (freopen(in_file, "r", stdin) == NULL) {
            _exit(3);
        }
        ptrace(PTRACE_TRACEME, 0, NULL, NULL);
        execl(driver, driver, (char *)NULL);
        _exit(4);
    }

    int status;
    waitpid(pid, &status, 0);
    if (!WIFSTOPPED(status)) {
        return 5;
    }

    errno = 0;
    long orig = ptrace(PTRACE_PEEKTEXT, pid, (void *)bp, NULL);
    if (errno != 0) {
        return 6;
    }
    long trap = (orig & ~0xFFL) | 0xCC;
    if (ptrace(PTRACE_POKETEXT, pid, (void *)bp, (void *)trap) != 0) {
        return 7;
    }

    int hits = 0;
    for (;;) {
        if (ptrace(PTRACE_CONT, pid, NULL, NULL) != 0) {
            return 8;
        }
        waitpid(pid, &status, 0);
        if (WIFEXITED(status)) {
            int code = WEXITSTATUS(status);
            /* the frame was never controlled: only legitimate when `main`
             * rejected its input before calling process_strings */
            if (hits == 0 && code == 0) {
                return 90;
            }
            return code;
        }
        if (WIFSIGNALED(status)) {
            if (hits == 0) {
                return 91;
            }
            return 128 + WTERMSIG(status);
        }
        if (!WIFSTOPPED(status)) {
            return 9;
        }
        int sig = WSTOPSIG(status);
        if (sig != SIGTRAP) {
            /* forward anything else (e.g. SIGSEGV from the unbounded loop) */
            ptrace(PTRACE_CONT, pid, NULL, (void *)(long)sig);
            waitpid(pid, &status, 0);
            if (WIFEXITED(status)) {
                return WEXITSTATUS(status);
            }
            if (WIFSIGNALED(status)) {
                return 128 + WTERMSIG(status);
            }
            return 10;
        }

        struct user_regs_struct regs;
        if (ptrace(PTRACE_GETREGS, pid, NULL, &regs) != 0) {
            return 11;
        }
        unsigned long input = regs.rdi;
        unsigned long input_len = regs.rsi;
        unsigned long reference = regs.rdx;
        unsigned long ref_len = regs.rcx;
        if (input != reference + BUF_SIZE) {
            fprintf(stderr, "unexpected frame layout: input-reference = %ld\n",
                    (long)(input - reference));
            kill(pid, SIGKILL);
            return 12;
        }
        unsigned long base = reference; /* modelled offset 0 */

        /* the bytes of both arrays that `main` never wrote ... */
        long ref_from = (long)(ref_len > BUF_SIZE ? BUF_SIZE : ref_len);
        long in_from = INPUT_OFF + (long)(input_len > BUF_SIZE ? BUF_SIZE : input_len);
        if (peek_poke_range(pid, base, junk, ref_from, BUF_SIZE) != 0 ||
            peek_poke_range(pid, base, junk, in_from, LOCALS_OFF) != 0 ||
            /* ... the 4 padding bytes at rbp-0x20 ... */
            peek_poke_range(pid, base, junk, LOCALS_OFF + 16, LOCALS_OFF + 20) != 0 ||
            /* ... and `int result` at rbp-0x14, which is only written later */
            peek_poke_range(pid, base, junk, LOCALS_OFF + 28, LOCALS_OFF + 32) != 0) {
            kill(pid, SIGKILL);
            return 13;
        }

        hits++;

        /* remove the breakpoint and resume at the original instruction */
        if (ptrace(PTRACE_POKETEXT, pid, (void *)bp, (void *)orig) != 0) {
            return 14;
        }
        regs.rip = bp;
        if (ptrace(PTRACE_SETREGS, pid, NULL, &regs) != 0) {
            return 15;
        }
    }
}
