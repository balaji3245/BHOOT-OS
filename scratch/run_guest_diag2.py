#!/usr/bin/env python3
import os, sys, subprocess, time, secrets, pexpect

ROOT_DIR = "/home/bhoot/Desktop/BHOOT"
BUILD_REAL = os.path.join(ROOT_DIR, "os/build_real")
CONFIG_DIR = os.path.join(ROOT_DIR, "os/config")
DISPOSABLE_DIR = os.path.join(BUILD_REAL, "disposable_diag_test2")
OVMF_PATH = "/usr/share/OVMF/OVMF_CODE_4M.fd"
TOTAL_SECTORS = 121241600
SECTOR_SIZE = 512

def main():
    luks_pass = secrets.token_urlsafe(24)
    user_pass = "AuditMasterPassword2026!"
    
    os.makedirs(DISPOSABLE_DIR, exist_ok=True)
    disk_img = os.path.join(DISPOSABLE_DIR, "bhoot_diag2.img")
    p3_img = os.path.join(DISPOSABLE_DIR, "diag2_p3.img")
    header_backup = os.path.join(DISPOSABLE_DIR, "diag2_header.backup")

    p = subprocess.Popen([
        "python3", os.path.join(ROOT_DIR, "os/scripts_v2/provision_luks2_volume.py"),
        "--container", p3_img,
        "--size-gib", "48",
        "--header-backup", header_backup,
        "--passphrase-fd", "0"
    ], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    p.communicate(input=f"{luks_pass}\n".encode())

    with open(disk_img, "wb") as f:
        f.truncate(TOTAL_SECTORS * SECTOR_SIZE)

    sfdisk_layout = os.path.join(CONFIG_DIR, "storage/gpt_layout.sfdisk")
    subprocess.run(["sfdisk", "--quiet", disk_img], input=open(sfdisk_layout, "rb").read(), check=True)

    esp_img = os.path.join(BUILD_REAL, "bhoot_esp.img")
    sys_img = os.path.join(BUILD_REAL, "bhoot_system_ext4.img")
    rec_img = os.path.join(BUILD_REAL, "bhoot_recovery.img")

    subprocess.run(["dd", f"if={esp_img}", f"of={disk_img}", "bs=1M", "seek=1", "conv=notrunc,sparse", "status=none"], check=True)
    subprocess.run(["dd", f"if={sys_img}", f"of={disk_img}", "bs=1M", "seek=1025", "count=4096", "conv=notrunc,sparse", "status=none"], check=True)
    subprocess.run(["dd", f"if={p3_img}", f"of={disk_img}", "bs=1M", "seek=5121", "count=49152", "conv=notrunc,sparse", "status=none"], check=True)
    subprocess.run(["dd", f"if={rec_img}", f"of={disk_img}", "bs=1M", "seek=54273", "count=4096", "conv=notrunc,sparse", "status=none"], check=True)

    qemu_cmd = [
        "qemu-system-x86_64",
        "-machine", "q35,accel=kvm",
        "-cpu", "host",
        "-m", "3072",
        "-smp", "2",
        "-drive", f"if=pflash,format=raw,readonly=on,file={OVMF_PATH}",
        "-drive", f"file={disk_img},format=raw,if=virtio",
        "-vga", "virtio",
        "-display", "none",
        "-serial", "stdio",
        "-net", "none"
    ]

    child = pexpect.spawn(qemu_cmd[0], qemu_cmd[1:], timeout=120, encoding="utf-8")
    
    # 1. LUKS prompt
    child.expect("Please unlock disk", timeout=45)
    time.sleep(0.5)
    child.sendline(luks_pass)

    # 2. First boot prompt
    child.expect("Enter new master password for 'bhoot'", timeout=45)
    time.sleep(0.5)
    child.sendline(user_pass)
    child.expect("Confirm master password:", timeout=15)
    time.sleep(0.5)
    child.sendline(user_pass)

    # GRUB Diceware
    child.expect(r">>> ([a-z]+ [a-z]+ [a-z]+ [a-z]+ [a-z]+) <<<", timeout=25)
    diceware_pass = child.match.group(1).strip()
    child.expect("Confirm by entering the exact 5-word passphrase:", timeout=15)
    time.sleep(0.5)
    child.sendline(diceware_pass)

    # 3. Login
    child.expect("bhoot-secure login:", timeout=35)
    child.sendline("bhoot")
    child.expect("Password:", timeout=10)
    child.sendline(user_pass)
    child.expect(r"\$", timeout=15)
    print(">>> LOGGED IN TO GUEST SHELL <<<")

    # Now monitor processes over 20 seconds
    for i in range(10):
        time.sleep(2)
        child.sendline("ps aux | grep -v '\[.*\]'")
        child.expect(r"\$", timeout=10)
        print(f"\n--- Check {i+1} ---")
        print(child.before)

    # Now manually execute bhoot-desktop to see stdout/stderr
    print("\n--- Executing /usr/local/bin/bhoot-desktop manually ---")
    child.sendline("/usr/local/bin/bhoot-desktop --port 1420 --db /tmp/diag.db &")
    child.expect(r"\$", timeout=10)
    time.sleep(2)
    child.sendline("ps aux | grep bhoot")
    child.expect(r"\$", timeout=10)
    print(child.before)

    # Try cage command manually
    print("\n--- Executing cage --version ---")
    child.sendline("/usr/bin/cage --version")
    child.expect(r"\$", timeout=10)
    print(child.before)

    child.sendline("/usr/bin/cog --version")
    child.expect(r"\$", timeout=10)
    print(child.before)

    child.terminate(force=True)

if __name__ == "__main__":
    main()

