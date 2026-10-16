/* Parity harness: runs the CLI through the C ABI and prints stdout.
 * usage: parity ARG...   (exit code = cadetta_run's) */
#include <stdio.h>
#include "cadetta.h"

int main(int argc, char **argv) {
    char *out = NULL, *err = NULL;
    int code = cadetta_run(argc - 1, (const char *const *)(argv + 1), &out, &err);
    if (out) fputs(out, stdout);
    if (err) fputs(err, stderr);
    cadetta_free(out);
    cadetta_free(err);
    return code < 0 ? 100 - code : code;
}
