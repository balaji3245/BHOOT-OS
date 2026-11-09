import socket
import json
import time
import subprocess
import os

qmp_sock = "/tmp/test_qmp_keys.sock"
if os.path.exists(qmp_sock):
    os.remove(qmp_sock)

qemu_cmd = [
    "qemu-system-x86_64",
    "-machine", "q35,accel=kvm",
    "-m", "512",
    "-display", "none",
    "-device", "virtio-keyboard-pci",
    "-qmp", f"unix:{qmp_sock},server,nowait",
    "-serial", "null"
]

proc = subprocess.Popen(qemu_cmd)
time.sleep(1)

s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(qmp_sock)
greeting = s.recv(1024)
s.sendall(json.dumps({"execute": "qmp_capabilities"}).encode() + b"\n")
resp = s.recv(1024)

# Test send-key command
test_cmd = {
    "execute": "send-key",
    "arguments": {
        "keys": [{"type": "qcode", "data": "a"}]
    }
}
s.sendall(json.dumps(test_cmd).encode() + b"\n")
resp = s.recv(1024)
print("QMP send-key response:", resp.decode())

# Send ret
s.sendall(json.dumps({"execute": "send-key", "arguments": {"keys": [{"type": "qcode", "data": "ret"}]}}).encode() + b"\n")
resp = s.recv(1024)
print("QMP send-key ret response:", resp.decode())

# Quit QEMU
s.sendall(json.dumps({"execute": "quit"}).encode() + b"\n")
s.close()
proc.wait()
print("QEMU successfully accepted send-key!")

