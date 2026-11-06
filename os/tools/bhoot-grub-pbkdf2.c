#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/random.h>
#include <openssl/evp.h>

#define SALT_LEN 64
#define HASH_LEN 64
#define PBKDF2_ROUNDS 10000

static int hex_char_to_val(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

static int hex_to_bin(const char *hex, unsigned char *bin, size_t bin_len) {
    if (strlen(hex) != bin_len * 2) return -1;
    for (size_t i = 0; i < bin_len; i++) {
        int hi = hex_char_to_val(hex[i * 2]);
        int lo = hex_char_to_val(hex[i * 2 + 1]);
        if (hi < 0 || lo < 0) return -1;
        bin[i] = (unsigned char)((hi << 4) | lo);
    }
    return 0;
}

static int read_password_stdin(char *pwd, size_t max_len) {
    memset(pwd, 0, max_len);
    if (fgets(pwd, (int)max_len, stdin) == NULL) {
        return -1;
    }
    size_t len = strlen(pwd);
    while (len > 0 && (pwd[len - 1] == '\n' || pwd[len - 1] == '\r')) {
        pwd[len - 1] = '\0';
        len--;
    }
    return (int)len;
}

/* Constant-time memory comparison to mitigate timing attacks */
static int constant_time_compare(const unsigned char *a, const unsigned char *b, size_t len) {
    int diff = 0;
    for (size_t i = 0; i < len; i++) {
        diff |= a[i] ^ b[i];
    }
    return diff == 0 ? 0 : -1;
}

int main(int argc, char *argv[]) {
    if (argc >= 2 && strcmp(argv[1], "--verify") == 0) {
        if (argc != 4) {
            fprintf(stderr, "Usage: %s --verify <salt_hex> <expected_hash_hex>\n", argv[0]);
            return 1;
        }
        unsigned char salt[SALT_LEN];
        unsigned char expected[HASH_LEN];
        if (hex_to_bin(argv[2], salt, SALT_LEN) != 0) {
            fprintf(stderr, "Invalid salt hex (expected 128 hex chars)\n");
            return 1;
        }
        if (hex_to_bin(argv[3], expected, HASH_LEN) != 0) {
            fprintf(stderr, "Invalid hash hex (expected 128 hex chars)\n");
            return 1;
        }

        char pwd[512];
        int len = read_password_stdin(pwd, sizeof(pwd));
        if (len < 0) {
            fprintf(stderr, "Error reading password from STDIN\n");
            explicit_bzero(pwd, sizeof(pwd));
            return 1;
        }

        unsigned char out[HASH_LEN];
        if (!PKCS5_PBKDF2_HMAC(pwd, len, salt, sizeof(salt), PBKDF2_ROUNDS, EVP_sha512(), sizeof(out), out)) {
            fprintf(stderr, "PBKDF2 computation failed\n");
            explicit_bzero(pwd, sizeof(pwd));
            explicit_bzero(salt, sizeof(salt));
            explicit_bzero(expected, sizeof(expected));
            return 1;
        }

        explicit_bzero(pwd, sizeof(pwd));

        int res = constant_time_compare(out, expected, HASH_LEN);
        explicit_bzero(out, sizeof(out));
        explicit_bzero(salt, sizeof(salt));
        explicit_bzero(expected, sizeof(expected));

        if (res == 0) {
            printf("VERIFICATION_SUCCESS\n");
            return 0;
        } else {
            fprintf(stderr, "VERIFICATION_FAILED\n");
            return 1;
        }
    } else if (argc >= 2 && strcmp(argv[1], "--verify-file") == 0) {
        if (argc != 3) {
            fprintf(stderr, "Usage: %s --verify-file <grub-auth.cfg>\n", argv[0]);
            return 1;
        }
        FILE *f = fopen(argv[2], "r");
        if (!f) {
            fprintf(stderr, "Cannot open file: %s\n", argv[2]);
            return 1;
        }
        char line[1024];
        char salt_hex[256];
        char hash_hex[256];
        memset(salt_hex, 0, sizeof(salt_hex));
        memset(hash_hex, 0, sizeof(hash_hex));

        const char *prefix = "password_pbkdf2 bhoot-admin grub.pbkdf2.sha512.10000.";
        size_t prefix_len = strlen(prefix);
        int found = 0;

        while (fgets(line, sizeof(line), f)) {
            if (strncmp(line, prefix, prefix_len) == 0) {
                const char *p = line + prefix_len;
                const char *dot = strchr(p, '.');
                if (dot && (dot - p) == 128) {
                    memcpy(salt_hex, p, 128);
                    salt_hex[128] = '\0';
                    const char *h = dot + 1;
                    size_t h_len = 0;
                    while (h[h_len] && h[h_len] != '\r' && h[h_len] != '\n' && h[h_len] != ' ') {
                        h_len++;
                    }
                    if (h_len == 128) {
                        memcpy(hash_hex, h, 128);
                        hash_hex[128] = '\0';
                        found = 1;
                        break;
                    }
                }
            }
        }
        fclose(f);

        if (!found) {
            fprintf(stderr, "No valid enrolled bhoot-admin PBKDF2 line found in %s\n", argv[2]);
            return 1;
        }

        unsigned char salt[SALT_LEN];
        unsigned char expected[HASH_LEN];
        if (hex_to_bin(salt_hex, salt, SALT_LEN) != 0 || hex_to_bin(hash_hex, expected, HASH_LEN) != 0) {
            fprintf(stderr, "Corrupted salt/hash in auth file\n");
            return 1;
        }

        char pwd[512];
        int len = read_password_stdin(pwd, sizeof(pwd));
        if (len < 0) {
            fprintf(stderr, "Error reading password from STDIN\n");
            explicit_bzero(pwd, sizeof(pwd));
            return 1;
        }

        unsigned char out[HASH_LEN];
        if (!PKCS5_PBKDF2_HMAC(pwd, len, salt, sizeof(salt), PBKDF2_ROUNDS, EVP_sha512(), sizeof(out), out)) {
            fprintf(stderr, "PBKDF2 computation failed\n");
            explicit_bzero(pwd, sizeof(pwd));
            explicit_bzero(salt, sizeof(salt));
            explicit_bzero(expected, sizeof(expected));
            return 1;
        }

        explicit_bzero(pwd, sizeof(pwd));

        int res = constant_time_compare(out, expected, HASH_LEN);
        explicit_bzero(out, sizeof(out));
        explicit_bzero(salt, sizeof(salt));
        explicit_bzero(expected, sizeof(expected));

        if (res == 0) {
            printf("VERIFICATION_SUCCESS\n");
            return 0;
        } else {
            fprintf(stderr, "VERIFICATION_FAILED\n");
            return 1;
        }
    } else {
        /* Default mode: Generate new salt and PBKDF2 hash from STDIN */
        char pwd[512];
        int len = read_password_stdin(pwd, sizeof(pwd));
        if (len < 0) {
            fprintf(stderr, "Error reading password from STDIN\n");
            return 1;
        }
        if (len < 12) {
            fprintf(stderr, "Password too short (minimum 12 characters)\n");
            explicit_bzero(pwd, sizeof(pwd));
            return 1;
        }

        /* Generate 64 cryptographically secure random bytes for salt */
        unsigned char salt[SALT_LEN];
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
        unsigned char out[HASH_LEN];
        if (!PKCS5_PBKDF2_HMAC(pwd, len, salt, sizeof(salt), PBKDF2_ROUNDS, EVP_sha512(), sizeof(out), out)) {
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
}
