/* ptrace based dumper: stops the C driver at the entry of process_strings and
 * dumps the `main` stack frame (ref_buffer, input_buffer and the locals above
 * them) so that the Rust translation can reproduce the *exact* bytes the C code
 * reads out of the uninitialised parts of that frame.
 *
 * usage: dump_frame <driver-path> <bp-addr-hex> <stdin-file> <bytes-after-refbuf>
 */
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/personality.h>
#include <sys/ptrace.h>
#include <sys/user.h>
#include <sys/wait.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc < 5) {
        fprintf(stderr, "usage: %s driver bp_hex stdin_file nbytes\n", argv[0]);
        return 2;
    }
    const char *driver = argv[1];
    unsigned long bp = strtoul(argv[2], NULL, 16);
    const char *in_file = argv[3];
    long nbytes = strtol(argv[4], NULL, 10);

    pid_t pid = fork();
    if (pid == 0) {
        if (freopen(in_file, "r", stdin) == NULL) {
            _exit(3);
        }
        ptrace(PTRACE_TRACEME, 0, NULL, NULL);
        execl(driver, driver, (char *)NULL);
        _exit(4);
    }

    int status;
    waitpid(pid, &status, 0);

    errno = 0;
    long orig = ptrace(PTRACE_PEEKTEXT, pid, (void *)bp, NULL);
    if (errno != 0) {
        fprintf(stderr, "peektext failed: %s\n", strerror(errno));
        return 1;
    }
    long trap = (orig & ~0xFFL) | 0xCC;
    if (ptrace(PTRACE_POKETEXT, pid, (void *)bp, (void *)trap) != 0) {
        fprintf(stderr, "poketext failed: %s\n", strerror(errno));
        return 1;
    }

    ptrace(PTRACE_CONT, pid, NULL, NULL);
    waitpid(pid, &status, 0);
    if (!WIFSTOPPED(status)) {
        fprintf(stderr, "child did not stop (status %d)\n", status);
        return 1;
    }

    struct user_regs_struct regs;
    if (ptrace(PTRACE_GETREGS, pid, NULL, &regs) != 0) {
        fprintf(stderr, "getregs failed: %s\n", strerror(errno));
        return 1;
    }
    unsigned long input = regs.rdi;
    unsigned long input_len = regs.rsi;
    unsigned long reference = regs.rdx;
    unsigned long ref_len = regs.rcx;
    unsigned long operation = regs.r8;
    unsigned long flags = regs.r9;
    fprintf(stderr, "rip=%lx input=%lx input_len=%lu reference=%lx ref_len=%lu op=%lu flags=%lu\n",
            (unsigned long)regs.rip, input, input_len, reference, ref_len, operation, flags);
    fprintf(stderr, "input - reference = %ld\n", (long)(input - reference));

    /* dump starting at reference (the lower of the two buffers) */
    printf("# base=reference=%lx input=%lx delta=%ld\n", reference, input, (long)(input - reference));
    for (long off = 0; off < nbytes; off += 8) {
        errno = 0;
        long word = ptrace(PTRACE_PEEKDATA, pid, (void *)(reference + off), NULL);
        if (errno != 0) {
            fprintf(stderr, "peekdata @%ld failed: %s\n", off, strerror(errno));
            break;
        }
        unsigned char b[8];
        memcpy(b, &word, 8);
        for (int i = 0; i < 8; i++) {
            printf("%ld %u\n", off + i, (unsigned)b[i]);
        }
    }
    fflush(stdout);
    kill(pid, 9);
    waitpid(pid, &status, 0);
    return 0;
}
