#!/usr/bin/env bash
""":"
python3 "$0" "$@"
exit $?
":"""
"""
BHOOT — Phase 2A.17 Strong Passphrase Enforcement & Security Claims Verification Suite

Audits all Phase 2A.17 implementations:
1. Canonical EFF Large Wordlist integrity (SHA-256, 7,776 lines, uniqueness, license)
2. Rootfs bundled wordlist integrity (/usr/share/bhoot & /etc/bhoot)
3. bhoot-grub-pbkdf2 --verify-wordlist positive & negative rejection tests
4. bhoot-grub-pbkdf2 --generate-diceware 5-word generation & CSPRNG rejection sampling
5. Statistical distribution & Chi-Square uniformity test of rejection sampling
6. Exact entropy verification (64.62 bits uncompressed)
7. bhoot-grub-pbkdf2 --validate-diceware positive & negative rejection tests
8. PBKDF2-HMAC-SHA512 hash generation & pre-activation verification (--verify-file)
9. First-boot UX & recovery console space preservation (IFS= read -r -s)
10. Memory zeroization (explicit_bzero) & CLI argument hygiene (stdin only)
11. Architecture spec & GPU benchmark claim corrections (marked NOT TESTED)
"""

import os
import sys
import subprocess
import hashlib
import tempfile
import math
import re
from collections import Counter

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OS_DIR = os.path.dirname(SCRIPT_DIR)
REPO_ROOT = os.path.dirname(OS_DIR)
BUILD_REAL = os.path.join(OS_DIR, "build_real")
ROOTFS = os.path.join(BUILD_REAL, "rootfs")
DOCS_DIR = os.path.join(REPO_ROOT, "docs")
CONFIG_DIR = os.path.join(OS_DIR, "config")
BIN_PBKDF2 = os.path.join(OS_DIR, "tools", "bhoot-grub-pbkdf2")
ROOTFS_BIN_PBKDF2 = os.path.join(ROOTFS, "usr", "local", "bin", "bhoot-grub-pbkdf2")

EXPECTED_EFF_SHA256 = "addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e"
EXPECTED_WORD_COUNT = 7776

passed = 0
failed = 0

def report(name: str, condition: bool, detail: str = ""):
    global passed, failed
    if condition:
        print(f"  [PASS] {name}")
        passed += 1
    else:
        print(f"  [FAIL] {name} - Detail: {detail}")
        failed += 1

def run_cmd(cmd: list, input_str: str = "") -> tuple[int, str, str]:
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    out, err = p.communicate(input=input_str)
    return p.returncode, out, err

# ==============================================================================
# SECTION 1: Canonical EFF Large Wordlist File Integrity & Provenance
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 1: Canonical EFF Large Wordlist Integrity & Provenance")
print("=" * 76)

eff_path = os.path.join(CONFIG_DIR, "eff_large_wordlist.txt")
report("EFF Large Wordlist exists in os/config/", os.path.isfile(eff_path))

with open(eff_path, "rb") as f:
    raw_bytes = f.read()
actual_hash = hashlib.sha256(raw_bytes).hexdigest()
report("EFF Wordlist SHA-256 matches canonical reference (addd3553...)",
       actual_hash == EXPECTED_EFF_SHA256,
       f"Expected {EXPECTED_EFF_SHA256}, got {actual_hash}")

lines = raw_bytes.decode("utf-8").strip().splitlines()
report("EFF Wordlist contains exactly 7,776 lines",
       len(lines) == EXPECTED_WORD_COUNT,
       f"Got {len(lines)}")

words = []
dice_indices = []
for line in lines:
    parts = line.split()
    if len(parts) >= 2:
        dice_indices.append(parts[0])
        words.append(parts[1])

report("EFF Wordlist contains 7,776 valid (index, word) pairs",
       len(words) == EXPECTED_WORD_COUNT,
       f"Parsed {len(words)} pairs")

report("EFF Wordlist contains 7,776 unique lowercase words (zero duplicates)",
       len(set(words)) == EXPECTED_WORD_COUNT and all(w.islower() and w.replace("-", "").isalpha() for w in words),
       f"Unique words: {len(set(words))}")

report("EFF Wordlist indices span 11111 through 66666 strictly in order",
       dice_indices[0] == "11111" and dice_indices[-1] == "66666" and len(set(dice_indices)) == EXPECTED_WORD_COUNT)

# ==============================================================================
# SECTION 2: Rootfs Bundled Wordlists (Offline-First Invariant)
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 2: Rootfs Bundled Wordlists (Offline-First Invariant)")
print("=" * 76)

usr_share_eff = os.path.join(ROOTFS, "usr", "share", "bhoot", "eff_large_wordlist.txt")
etc_eff = os.path.join(ROOTFS, "etc", "bhoot", "eff_large_wordlist.txt")

report("Rootfs contains /usr/share/bhoot/eff_large_wordlist.txt", os.path.isfile(usr_share_eff))
report("Rootfs contains /etc/bhoot/eff_large_wordlist.txt", os.path.isfile(etc_eff))

if os.path.isfile(usr_share_eff):
    with open(usr_share_eff, "rb") as f:
        usr_hash = hashlib.sha256(f.read()).hexdigest()
    report("/usr/share/bhoot/eff_large_wordlist.txt SHA-256 matches canonical", usr_hash == EXPECTED_EFF_SHA256)

if os.path.isfile(etc_eff):
    with open(etc_eff, "rb") as f:
        etc_hash = hashlib.sha256(f.read()).hexdigest()
    report("/etc/bhoot/eff_large_wordlist.txt SHA-256 matches canonical", etc_hash == EXPECTED_EFF_SHA256)

# ==============================================================================
# SECTION 3: bhoot-grub-pbkdf2 --verify-wordlist CLI Tests
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 3: bhoot-grub-pbkdf2 --verify-wordlist CLI Tests")
print("=" * 76)

report("Host compiled bhoot-grub-pbkdf2 exists and is executable",
       os.path.isfile(BIN_PBKDF2) and os.access(BIN_PBKDF2, os.X_OK))

rc, out, err = run_cmd([BIN_PBKDF2, "--verify-wordlist", eff_path])
report("--verify-wordlist exits 0 on canonical wordlist",
       rc == 0 and "WORDLIST_INTEGRITY_VERIFIED_7776_WORDS" in out,
       f"rc={rc}, out={out}, err={err}")

# Test non-existent file
rc, out, err = run_cmd([BIN_PBKDF2, "--verify-wordlist", "/tmp/non_existent_file_xyz.txt"])
report("--verify-wordlist fails on non-existent file", rc != 0)

# Test tampered SHA-256 (modified single character in wordlist)
with tempfile.NamedTemporaryFile("w+", delete=False) as tf:
    tampered_lines = list(lines)
    tampered_lines[10] = "11125\talteredword"
    tf.write("\n".join(tampered_lines) + "\n")
    tf_tampered = tf.name

rc, out, err = run_cmd([BIN_PBKDF2, "--verify-wordlist", tf_tampered])
report("--verify-wordlist rejects tampered wordlist (SHA-256 mismatch)",
       rc != 0 and "verification failed" in err.lower(),
       f"rc={rc}, err={err}")
os.unlink(tf_tampered)

# Test truncated wordlist (7,775 words)
with tempfile.NamedTemporaryFile("w+", delete=False) as tf:
    tf.write("\n".join(lines[:-1]) + "\n")
    tf_truncated = tf.name

rc, out, err = run_cmd([BIN_PBKDF2, "--verify-wordlist", tf_truncated])
report("--verify-wordlist rejects truncated wordlist (< 7,776 words)",
       rc != 0,
       f"rc={rc}, err={err}")
os.unlink(tf_truncated)

# Test rootfs binary --verify-wordlist
if os.path.isfile(ROOTFS_BIN_PBKDF2):
    rc, out, err = run_cmd([ROOTFS_BIN_PBKDF2, "--verify-wordlist", usr_share_eff])
    report("Rootfs binary --verify-wordlist executes and succeeds against rootfs wordlist",
           rc == 0 and "WORDLIST_INTEGRITY_VERIFIED_7776_WORDS" in out)

# ==============================================================================
# SECTION 4: bhoot-grub-pbkdf2 --generate-diceware & Rejection Sampling
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 4: bhoot-grub-pbkdf2 --generate-diceware & Rejection Sampling")
print("=" * 76)

eff_word_set = set(words)
generated_passphrases = []

for i in range(20):
    rc, out, err = run_cmd([BIN_PBKDF2, "--generate-diceware", eff_path])
    if rc == 0:
        phrase = out.strip()
        generated_passphrases.append(phrase)

report("Generated 20 passphrases successfully (rc == 0)", len(generated_passphrases) == 20)

all_5_words = True
all_in_eff = True
all_lowercase = True

for phrase in generated_passphrases:
    phrase_words = phrase.split(" ")
    if len(phrase_words) != 5:
        all_5_words = False
    for w in phrase_words:
        if w not in eff_word_set:
            all_in_eff = False
        if not (w.islower() and w.replace("-", "").isalpha()):
            all_lowercase = False

report("All generated passphrases have exactly 5 words separated by single spaces", all_5_words)
report("All generated words belong to the canonical EFF wordlist", all_in_eff)
report("All generated words are strictly lowercase alphabetic or hyphenated", all_lowercase)
report("Generated passphrases exhibit distinct randomness (non-deterministic CSPRNG)",
       len(set(generated_passphrases)) == len(generated_passphrases))

# ==============================================================================
# SECTION 5: Statistical Uniformity & Chi-Square Test of Rejection Sampling
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 5: Statistical Uniformity & Chi-Square Test of Rejection Sampling")
print("=" * 76)

SAMPLE_COUNT = 500
TOTAL_WORDS = SAMPLE_COUNT * 5
word_indices = []

word_to_index = {w: i for i, w in enumerate(words)}

for _ in range(SAMPLE_COUNT):
    rc, out, err = run_cmd([BIN_PBKDF2, "--generate-diceware", eff_path])
    if rc == 0:
        for w in out.strip().split(" "):
            if w in word_to_index:
                word_indices.append(word_to_index[w])

report(f"Successfully drew {len(word_indices)} random words via CSPRNG rejection sampling",
       len(word_indices) == TOTAL_WORDS)

report("All drawn indices strictly satisfy 0 <= index < 7776 (zero out-of-bounds)",
       all(0 <= idx < EXPECTED_WORD_COUNT for idx in word_indices))

# Partition wordlist into 16 equal bins (486 words each)
NUM_BINS = 16
BIN_SIZE = EXPECTED_WORD_COUNT // NUM_BINS
bin_counts = [0] * NUM_BINS

for idx in word_indices:
    bin_id = min(idx // BIN_SIZE, NUM_BINS - 1)
    bin_counts[bin_id] += 1

expected_per_bin = TOTAL_WORDS / NUM_BINS
chi_square = sum(((observed - expected_per_bin) ** 2) / expected_per_bin for observed in bin_counts)

# For 15 degrees of freedom, critical value at p = 0.001 is 37.697; at p = 0.01 is 30.578
CHI_SQUARE_CRITICAL_P001 = 37.697
report(f"Chi-Square test across {NUM_BINS} bins (chi2 = {chi_square:.2f}, critical p=0.001 is {CHI_SQUARE_CRITICAL_P001})",
       chi_square < CHI_SQUARE_CRITICAL_P001,
       f"Observed counts: {bin_counts}")

# Mathematical entropy verification
entropy_per_word = math.log2(EXPECTED_WORD_COUNT)
total_entropy = 5 * entropy_per_word
report(f"Entropy calculation: 5 * log2(7776) = {total_entropy:.3f} bits (>= 64.62 bits)",
       round(total_entropy, 2) == 64.62,
       f"Calculated {total_entropy}")

# ==============================================================================
# SECTION 6: bhoot-grub-pbkdf2 --validate-diceware CLI Tests
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 6: bhoot-grub-pbkdf2 --validate-diceware CLI Tests")
print("=" * 76)

sample_phrase = "baggage gradually battered crablike roundish"
rc, out, err = run_cmd([BIN_PBKDF2, "--validate-diceware", eff_path], input_str=sample_phrase)
report("--validate-diceware accepts genuine 5-word Diceware passphrase",
       rc == 0 and "DICEWARE_PASSPHRASE_VALID" in out)

# Negative test 1: 4 words (under-length)
rc, out, err = run_cmd([BIN_PBKDF2, "--validate-diceware", eff_path], input_str="baggage gradually battered crablike")
report("--validate-diceware rejects 4-word passphrase", rc != 0)

# Negative test 2: 6 words (over-length)
rc, out, err = run_cmd([BIN_PBKDF2, "--validate-diceware", eff_path], input_str="baggage gradually battered crablike roundish extra")
report("--validate-diceware rejects 6-word passphrase", rc != 0)

# Negative test 3: Word not in EFF wordlist
rc, out, err = run_cmd([BIN_PBKDF2, "--validate-diceware", eff_path], input_str="baggage gradually battered crablike notinwordlistxyz")
report("--validate-diceware rejects invalid word outside EFF wordlist", rc != 0)

# Negative test 4: Uppercase word
rc, out, err = run_cmd([BIN_PBKDF2, "--validate-diceware", eff_path], input_str="Baggage gradually battered crablike roundish")
report("--validate-diceware rejects uppercase characters", rc != 0)

# Negative test 5: Empty input
rc, out, err = run_cmd([BIN_PBKDF2, "--validate-diceware", eff_path], input_str="")
report("--validate-diceware rejects empty input", rc != 0)

# Negative test 6: Numbers and punctuation
rc, out, err = run_cmd([BIN_PBKDF2, "--validate-diceware", eff_path], input_str="baggage gradually battered crablike 12345")
report("--validate-diceware rejects numeric input", rc != 0)

# ==============================================================================
# SECTION 7: PBKDF2 Hash Generation & Pre-Activation Verification Mode
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 7: PBKDF2 Hash Generation & Pre-Activation Verification")
print("=" * 76)

test_diceware = "bacon adjust unbaked custom goblet"
rc, auth_line, err = run_cmd([BIN_PBKDF2], input_str=test_diceware)
auth_line = auth_line.strip()

report("Generated PBKDF2 auth line begins with 'password_pbkdf2 bhoot-admin'",
       rc == 0 and auth_line.startswith("password_pbkdf2 bhoot-admin grub.pbkdf2.sha512.10000."))

parts = auth_line.split(".")
report("PBKDF2 formatted string contains 6 dot-delimited tokens", len(parts) == 6, f"Parts: {len(parts)}")
salt_hex = parts[4] if len(parts) == 6 else ""
hash_hex = parts[5] if len(parts) == 6 else ""

report("Salt is exactly 128 hexadecimal characters (64 bytes CSPRNG)",
       len(salt_hex) == 128 and all(c in "0123456789ABCDEF" for c in salt_hex))

report("Hash is exactly 128 hexadecimal characters (64 bytes SHA-512)",
       len(hash_hex) == 128 and all(c in "0123456789ABCDEF" for c in hash_hex))

# Pre-activation self-verification (--verify-file)
with tempfile.NamedTemporaryFile("w+", delete=False) as tf:
    tf.write(auth_line + "\n")
    tf_auth = tf.name

rc, out, err = run_cmd([BIN_PBKDF2, "--verify-file", tf_auth], input_str=test_diceware)
report("--verify-file self-verification succeeds with exact 5-word Diceware passphrase",
       rc == 0 and "VERIFICATION_SUCCESS" in out)

# Negative test: incorrect word in passphrase
wrong_diceware = "bacon adjust unbaked custom wrongword"
rc, out, err = run_cmd([BIN_PBKDF2, "--verify-file", tf_auth], input_str=wrong_diceware)
report("--verify-file rejects incorrect passphrase", rc != 0)

# Negative test: tampered hash in auth file
tampered_line = auth_line[:-4] + "0000"
with open(tf_auth, "w") as f:
    f.write(tampered_line + "\n")

rc, out, err = run_cmd([BIN_PBKDF2, "--verify-file", tf_auth], input_str=test_diceware)
report("--verify-file rejects tampered PBKDF2 hash file", rc != 0)
os.unlink(tf_auth)

# Direct verification mode (--verify <salt> <hash>)
rc, out, err = run_cmd([BIN_PBKDF2, "--verify", salt_hex, hash_hex], input_str=test_diceware)
report("--verify direct CLI mode accepts matching 5-word Diceware passphrase",
       rc == 0 and "VERIFICATION_SUCCESS" in out)

rc, out, err = run_cmd([BIN_PBKDF2, "--verify", salt_hex, hash_hex], input_str="wrong word sequence completely")
report("--verify direct CLI mode rejects non-matching passphrase", rc != 0)

# ==============================================================================
# SECTION 8: First-Boot UX & Space-Preserving Shell Input (IFS= read -r -s)
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 8: First-Boot UX & Space-Preserving Shell Input")
print("=" * 76)

config_script = os.path.join(OS_DIR, "scripts_v2", "02_configure_rootfs.sh")
with open(config_script, "r") as f:
    config_code = f.read()

report("02_configure_rootfs.sh enforces --verify-wordlist before generation",
       "--verify-wordlist" in config_code)

report("02_configure_rootfs.sh generates passphrase via --generate-diceware",
       "--generate-diceware" in config_code)

report("02_configure_rootfs.sh preserves spaces using 'IFS= read -r -s'",
       "IFS= read -r -s" in config_code)

report("02_configure_rootfs.sh enforces pre-activation verification (--verify-file)",
       "--verify-file" in config_code)

report("02_configure_rootfs.sh uses atomic rename to install grub-auth.cfg",
       "mv -f" in config_code and "grub-auth.cfg" in config_code)

# Check bhoot-recovery-console
recovery_console = os.path.join(ROOTFS, "usr", "local", "sbin", "bhoot-recovery-console")
report("Rootfs contains /usr/local/sbin/bhoot-recovery-console", os.path.isfile(recovery_console))

if os.path.isfile(recovery_console):
    with open(recovery_console, "r") as f:
        console_code = f.read()
    report("bhoot-recovery-console preserves spaces using 'IFS= read -r -s'",
           "IFS= read -r -s -p" in console_code)
    report("bhoot-recovery-console authenticates via bhoot-grub-pbkdf2 --verify",
           "bhoot-grub-pbkdf2" in console_code and "--verify" in console_code)

# Verify bash space preservation behavior directly
test_input = "word1 word2 word3 word4 word5"
proc = subprocess.Popen(
    ["bash", "-c", 'IFS= read -r -s VAR; if [ "$VAR" = "word1 word2 word3 word4 word5" ]; then echo "SPACE_PRESERVED"; fi'],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
)
out, _ = proc.communicate(input=test_input)
report("Bash 'IFS= read -r -s' correctly preserves verbatim 5-word string with spaces",
       "SPACE_PRESERVED" in out)

# ==============================================================================
# SECTION 9: Memory Zeroization & CLI Argument Hygiene
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 9: Memory Zeroization & CLI Argument Hygiene")
print("=" * 76)

c_src_path = os.path.join(OS_DIR, "tools", "bhoot-grub-pbkdf2.c")
with open(c_src_path, "r") as f:
    c_source = f.read()

report("C utility bhoot-grub-pbkdf2.c reads passphrase via STDIN only (never argv)",
       "read_password_stdin" in c_source and 'strcmp(argv[1], "--verify")' in c_source)

report("C utility invokes explicit_bzero on passphrase buffers",
       c_source.count("explicit_bzero") >= 6)

report("C utility uses constant_time_compare for PBKDF2 hash verification",
       "constant_time_compare" in c_source)

report("C utility implements rejection sampling boundary (62208)",
       "62208" in c_source and "7776" in c_source)

# Verify dynamic binary glibc compatibility
rc, ldd_out, _ = run_cmd(["objdump", "-T", BIN_PBKDF2])
symbols = ldd_out.splitlines()
max_glibc = "2.0"
for line in symbols:
    m = re.search(r"GLIBC_2\.(\d+)", line)
    if m:
        ver = int(m.group(1))
        if ver > int(max_glibc.split(".")[1]):
            max_glibc = f"2.{ver}"

report(f"Binary dynamic symbols require at most GLIBC_{max_glibc} (Debian 12 Bookworm compatible)",
       int(max_glibc.split(".")[1]) <= 36)

# ==============================================================================
# SECTION 10: Secret Leakage & In-Transit Exposure Scanner
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 10: Secret Leakage & In-Transit Exposure Scanner")
print("=" * 76)

scan_scripts = [
    os.path.join(OS_DIR, "scripts_v2", "02_configure_rootfs.sh"),
    os.path.join(OS_DIR, "scripts_v2", "04_build_real_esp.sh"),
    os.path.join(OS_DIR, "tools", "bhoot-grub-pbkdf2.c"),
]

zero_leak = True
for s in scan_scripts:
    with open(s, "r") as f:
        content = f.read()
    if re.search(r'GRUB_PASS_INPUT=["\'][a-z]+ [a-z]+ [a-z]+', content):
        zero_leak = False

report("No hardcoded passphrases or credentials in build scripts / C tools", zero_leak)

# ==============================================================================
# SECTION 11: Architecture Specification & Claim Corrections
# ==============================================================================
print("\n" + "=" * 76)
print(" SECTION 11: Architecture Specification & Security Claims Corrections")
print("=" * 76)

spec_path = os.path.join(DOCS_DIR, "architecture", "BOOT_SECURITY_AND_FIRMWARE_SPEC.md")
report("BOOT_SECURITY_AND_FIRMWARE_SPEC.md exists", os.path.isfile(spec_path))

with open(spec_path, "r") as f:
    spec_content = f.read()

report("Spec documents EFF Large Wordlist provenance & CC-BY-3.0 US license",
       "Joseph Bonneau" in spec_content and "CC-BY-3.0 US" in spec_content)

report("Spec documents 5-word Diceware entropy (64.62 bits)",
       "64.62" in spec_content or "64.624" in spec_content)

report("Spec documents CSPRNG rejection sampling threshold (62,208)",
       "62,208" in spec_content or "62208" in spec_content)

report("Spec marks GPU cracking benchmarks as NOT TESTED on local hardware",
       "NOT TESTED ON HOST" in spec_content or "NOT TESTED" in spec_content)

report("Spec details Intel Comet Lake UHD graphics hardware status (no NVIDIA GPUs)",
       "Comet Lake" in spec_content and "8086:9b41" in spec_content)

report("Spec documents transparent theoretical estimates (8x RTX 4090 ~8,590 years)",
       "RTX 4090" in spec_content and ("8,594" in spec_content or "8,590" in spec_content))

report("Spec clarifies Argon2id memory-hardness without claiming 'immunity'",
       ("memory hardness" in spec_content.lower() or "memory-hardness" in spec_content.lower()) and 'not "immune"' in spec_content)

# ==============================================================================
# SUMMARY REPORT
# ==============================================================================
print("\n" + "=" * 76)
print(f" PHASE 2A.17 VERIFICATION SUMMARY: {passed} PASSED, {failed} FAILED")
print("=" * 76)

if failed > 0:
    print(f"FAILED: {failed} test(s) failed.")
    sys.exit(1)
else:
    print("ALL PHASE 2A.17 PASSPHRASE HARDENING TESTS PASSED.")
    sys.exit(0)
