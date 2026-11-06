#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <sys/random.h>
#include <openssl/evp.h>

#define SALT_LEN 64
#define HASH_LEN 64
#define PBKDF2_ROUNDS 10000
#define WORDLIST_SIZE 7776
#define EXPECTED_EFF_SHA256 "addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e"

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

static int get_csprng_bytes(void *buf, size_t len) {
    if (getrandom(buf, len, 0) == (ssize_t)len) return 0;
    FILE *f = fopen("/dev/urandom", "rb");
    if (!f) return -1;
    size_t r = fread(buf, 1, len, f);
    fclose(f);
    return (r == len) ? 0 : -1;
}

/* Rejection sampling: draw uniform uint16_t < 62208 (8 * 7776) to eliminate modulo bias */
static int draw_uniform_word_index(uint16_t *out_idx) {
    uint16_t val;
    for (int attempts = 0; attempts < 100; attempts++) {
        if (get_csprng_bytes(&val, sizeof(val)) != 0) return -1;
        if (val < 62208) {
            *out_idx = val % WORDLIST_SIZE;
            return 0;
        }
    }
    return -1;
}

static int verify_file_sha256(const char *filepath, const char *expected_hex) {
    FILE *f = fopen(filepath, "rb");
    if (!f) return -1;

    EVP_MD_CTX *ctx = EVP_MD_CTX_new();
    if (!ctx) { fclose(f); return -1; }
    if (!EVP_DigestInit_ex(ctx, EVP_sha256(), NULL)) { EVP_MD_CTX_free(ctx); fclose(f); return -1; }

    unsigned char buf[8192];
    size_t n;
    while ((n = fread(buf, 1, sizeof(buf), f)) > 0) {
        if (!EVP_DigestUpdate(ctx, buf, n)) { EVP_MD_CTX_free(ctx); fclose(f); return -1; }
    }
    fclose(f);

    unsigned char digest[32];
    unsigned int dlen = 0;
    if (!EVP_DigestFinal_ex(ctx, digest, &dlen)) { EVP_MD_CTX_free(ctx); return -1; }
    EVP_MD_CTX_free(ctx);

    char hex[65];
    for (int i = 0; i < 32; i++) {
        sprintf(hex + i * 2, "%02x", digest[i]);
    }
    hex[64] = '\0';

    return (strcmp(hex, expected_hex) == 0) ? 0 : -2;
}

static int word_cmp(const void *a, const void *b) {
    return strcmp((const char *)a, (const char *)b);
}

static int load_and_verify_wordlist(const char *filepath, char words[WORDLIST_SIZE][32]) {
    if (verify_file_sha256(filepath, EXPECTED_EFF_SHA256) != 0) {
        fprintf(stderr, "Wordlist SHA-256 verification failed (corrupted or tampered list)\n");
        return -1;
    }

    FILE *f = fopen(filepath, "r");
    if (!f) return -1;

    char line[128];
    int count = 0;
    while (fgets(line, sizeof(line), f)) {
        char *tab = strchr(line, '\t');
        if (!tab) tab = strchr(line, ' ');
        const char *w = tab ? tab + 1 : line;
        char clean_w[32];
        size_t l = 0;
        while (w[l] && w[l] != '\r' && w[l] != '\n' && w[l] != ' ' && l < 31) {
            clean_w[l] = w[l];
            l++;
        }
        clean_w[l] = '\0';
        if (l == 0) continue;
        if (count >= WORDLIST_SIZE) { fclose(f); return -2; }
        strncpy(words[count], clean_w, 32);
        words[count][31] = '\0';
        count++;
    }
    fclose(f);

    if (count != WORDLIST_SIZE) {
        fprintf(stderr, "Wordlist entry count mismatch: expected %d, got %d\n", WORDLIST_SIZE, count);
        return -3;
    }

    /* Verify uniqueness of all 7,776 words */
    char sorted[WORDLIST_SIZE][32];
    memcpy(sorted, words, sizeof(sorted));
    qsort(sorted, WORDLIST_SIZE, 32, word_cmp);
    for (int i = 0; i < WORDLIST_SIZE - 1; i++) {
        if (strcmp(sorted[i], sorted[i + 1]) == 0) {
            fprintf(stderr, "Duplicate word found in list: %s\n", sorted[i]);
            explicit_bzero(sorted, sizeof(sorted));
            return -4;
        }
    }
    explicit_bzero(sorted, sizeof(sorted));
    return 0;
}

static int validate_diceware_passphrase(const char *pwd, const char *filepath) {
    static char words[WORDLIST_SIZE][32];
    if (load_and_verify_wordlist(filepath, words) != 0) return -1;

    char sorted[WORDLIST_SIZE][32];
    memcpy(sorted, words, sizeof(sorted));
    qsort(sorted, WORDLIST_SIZE, 32, word_cmp);

    char copy[512];
    strncpy(copy, pwd, sizeof(copy) - 1);
    copy[sizeof(copy) - 1] = '\0';

    int word_count = 0;
    char *saveptr = NULL;
    char *token = strtok_r(copy, " ", &saveptr);
    int all_valid = 1;

    while (token) {
        word_count++;
        if (word_count > 5) {
            all_valid = 0;
            break;
        }
        if (bsearch(token, sorted, WORDLIST_SIZE, 32, word_cmp) == NULL) {
            all_valid = 0;
            break;
        }
        token = strtok_r(NULL, " ", &saveptr);
    }

    explicit_bzero(copy, sizeof(copy));
    explicit_bzero(sorted, sizeof(sorted));

    return (word_count == 5 && all_valid) ? 0 : -2;
}

int main(int argc, char *argv[]) {
    if (argc >= 2 && strcmp(argv[1], "--verify-wordlist") == 0) {
        if (argc != 3) {
            fprintf(stderr, "Usage: %s --verify-wordlist <wordlist_path>\n", argv[0]);
            return 1;
        }
        static char words[WORDLIST_SIZE][32];
        if (load_and_verify_wordlist(argv[2], words) != 0) {
            return 1;
        }
        printf("WORDLIST_INTEGRITY_VERIFIED_7776_WORDS\n");
        return 0;
    } else if (argc >= 2 && strcmp(argv[1], "--generate-diceware") == 0) {
        if (argc != 3) {
            fprintf(stderr, "Usage: %s --generate-diceware <wordlist_path>\n", argv[0]);
            return 1;
        }
        static char words[WORDLIST_SIZE][32];
        if (load_and_verify_wordlist(argv[2], words) != 0) {
            return 1;
        }

        char passphrase[256];
        passphrase[0] = '\0';
        for (int i = 0; i < 5; i++) {
            uint16_t idx;
            if (draw_uniform_word_index(&idx) != 0) {
                fprintf(stderr, "CSPRNG error generating word index\n");
                explicit_bzero(passphrase, sizeof(passphrase));
                return 1;
            }
            if (i > 0) strcat(passphrase, " ");
            strcat(passphrase, words[idx]);
        }

        printf("%s\n", passphrase);
        explicit_bzero(passphrase, sizeof(passphrase));
        return 0;
    } else if (argc >= 2 && strcmp(argv[1], "--validate-diceware") == 0) {
        if (argc != 3) {
            fprintf(stderr, "Usage: %s --validate-diceware <wordlist_path>\n", argv[0]);
            return 1;
        }
        char pwd[512];
        int len = read_password_stdin(pwd, sizeof(pwd));
        if (len < 0) {
            fprintf(stderr, "Error reading passphrase from STDIN\n");
            explicit_bzero(pwd, sizeof(pwd));
            return 1;
        }
        int res = validate_diceware_passphrase(pwd, argv[2]);
        explicit_bzero(pwd, sizeof(pwd));
        if (res == 0) {
            printf("DICEWARE_PASSPHRASE_VALID\n");
            return 0;
        } else {
            fprintf(stderr, "DICEWARE_PASSPHRASE_INVALID\n");
            return 1;
        }
    } else if (argc >= 2 && strcmp(argv[1], "--verify") == 0) {
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
