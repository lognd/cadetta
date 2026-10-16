/* cadetta C ABI (bindings/c, docs/spec/bindings.md 4). ASCII only.
 *
 * One call runs the cadetta command line in-process and returns its exit
 * code with captured stdout and stderr, so `cadetta_run` with
 * `--format json` returns exactly the document the CLI prints.
 */
#ifndef CADETTA_H
#define CADETTA_H

#ifdef __cplusplus
extern "C" {
#endif

/* ABI version; bumped only on an incompatible change to this header. */
#define CADETTA_ABI_VERSION 1

/* Returned by cadetta_run when its arguments are unusable (NULL or
 * non-UTF-8 argv entries). */
#define CADETTA_E_ARGS (-1)
/* Returned by cadetta_run when the core panicked (a cadetta bug). */
#define CADETTA_E_PANIC (-2)

/* The core version, e.g. "cadetta 0.1.0". Static; do not free. */
const char *cadetta_version(void);

/* Run the CLI with `argc` arguments in `argv` (the arguments after the
 * program name, as in `cadetta --format json check`).
 *
 * Returns the CLI exit code (0 ok, 1 error diagnostics, 2 usage or I/O),
 * or CADETTA_E_ARGS / CADETTA_E_PANIC. If `out` / `err` are non-NULL, each
 * receives a NUL-terminated heap string with the captured stdout / stderr
 * (set even on a negative return, to "" or a message); release each with
 * cadetta_free. Pass NULL to discard a stream. */
int cadetta_run(int argc, const char *const *argv, char **out, char **err);

/* Release a string returned through cadetta_run. NULL is a no-op. */
void cadetta_free(char *s);

#ifdef __cplusplus
}
#endif
#endif /* CADETTA_H */
