#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/random.h>
#include <openssl/evp.h>

int main(void) {
    char pwd[512];
    memset(pwd, 0, sizeof(pwd));

    /* Read password strictly from STDIN (no command line arguments, no env var) */
    if (fgets(pwd, sizeof(pwd), stdin) == NULL) {
        fprintf(stderr, "Error reading password from STDIN\n");
        return 1;
    }

    /* Strip trailing newline */
    size_t len = strlen(pwd);
    while (len > 0 && (pwd[len - 1] == '\n' || pwd[len - 1] == '\r')) {
        pwd[len - 1] = '\0';
        len--;
    }

    if (len < 12) {
        fprintf(stderr, "Password too short (minimum 12 characters)\n");
        explicit_bzero(pwd, sizeof(pwd));
        return 1;
    }

    /* Generate 64 cryptographically secure random bytes for salt */
    unsigned char salt[64];
    if (getrandom(salt, sizeof(salt), 0) != sizeof(salt)) {
        FILE *f = fopen("/dev/urandom", "rb");
        if (!f || fread(salt, 1, sizeof(salt), f) != sizeof(salt)) {
            if (f) fclose(f);
            fprintf(stderr, "Error generating CSPRNG salt\n");
            explicit_bzero(pwd, sizeof(pwd));
            return 1;
        }
        fclose(f);
    }

    /* Compute 10,000 iterations of PBKDF2-HMAC-SHA512 */
    unsigned char out[64];
    if (!PKCS5_PBKDF2_HMAC(pwd, (int)len, salt, sizeof(salt), 10000, EVP_sha512(), sizeof(out), out)) {
        fprintf(stderr, "PKCS5_PBKDF2_HMAC computation failed\n");
        explicit_bzero(pwd, sizeof(pwd));
        explicit_bzero(salt, sizeof(salt));
        return 1;
    }

    /* Immediately wipe plaintext password from memory */
    explicit_bzero(pwd, sizeof(pwd));

    /* Format output matching GRUB password_pbkdf2 specification */
    printf("password_pbkdf2 bhoot-admin grub.pbkdf2.sha512.10000.");
    for (size_t i = 0; i < sizeof(salt); i++) {
        printf("%02X", salt[i]);
    }
    printf(".");
    for (size_t i = 0; i < sizeof(out); i++) {
        printf("%02X", out[i]);
    }
    printf("\n");

    /* Wipe sensitive digests from stack */
    explicit_bzero(out, sizeof(out));
    explicit_bzero(salt, sizeof(salt));
    return 0;
}
