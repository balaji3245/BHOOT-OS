#!/usr/bin/env python3
"""
Unit and Integration Tests for BHOOT Safe System Update & Rollback Engine.
"""

import hashlib
import os
import shutil
import tempfile
import unittest
from pathlib import Path

# Import bhoot_sysupdate functions
from bhoot_sysupdate import (
    SQUASHFS_MAGIC,
    cmd_apply,
    cmd_rollback,
    cmd_status,
    cmd_verify_dry_run,
    compute_sha256,
    verify_squashfs_magic,
)


class TestBhootSysupdate(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp(prefix="bhoot_sysupdate_test_")
        self.target_dir = Path(self.test_dir) / "live"
        self.target_dir.mkdir(parents=True)

        # Create dummy simulated SquashFS (6 MiB with hsqs header)
        self.orig_img = self.target_dir / "filesystem.squashfs"
        with open(self.orig_img, "wb") as f:
            f.write(SQUASHFS_MAGIC)
            f.write(b"ORIGINAL_OS_PAYLOAD" * 350000)

        # Create update image (6 MiB with hsqs header)
        self.update_img = Path(self.test_dir) / "update.squashfs"
        with open(self.update_img, "wb") as f:
            f.write(SQUASHFS_MAGIC)
            f.write(b"NEW_UPDATED_OS_PAYLOAD" * 350000)

    def tearDown(self):
        shutil.rmtree(self.test_dir)

    def test_01_magic_verification(self):
        self.assertTrue(verify_squashfs_magic(self.orig_img))
        self.assertTrue(verify_squashfs_magic(self.update_img))

        bad_img = Path(self.test_dir) / "corrupt.img"
        with open(bad_img, "wb") as f:
            f.write(b"RANDOM_HEADER_BYTES")
        self.assertFalse(verify_squashfs_magic(bad_img))

    def test_02_preflight_dry_run(self):
        actual_sha = compute_sha256(self.update_img)
        # Should pass with matching SHA-256
        rc = cmd_verify_dry_run(self.update_img, actual_sha, skip_content_check=True)
        self.assertEqual(rc, 0)

        # Should fail with mismatched SHA-256
        rc = cmd_verify_dry_run(self.update_img, "0" * 64, skip_content_check=True)
        self.assertEqual(rc, 1)

    def test_03_atomic_apply_and_rollback(self):
        orig_sha = compute_sha256(self.orig_img)
        update_sha = compute_sha256(self.update_img)
        self.assertNotEqual(orig_sha, update_sha)

        # 1. Apply update
        rc = cmd_apply(self.update_img, self.target_dir, update_sha, skip_content_check=True)
        self.assertEqual(rc, 0)

        # Active image should now be the new update
        current_active_sha = compute_sha256(self.target_dir / "filesystem.squashfs")
        self.assertEqual(current_active_sha, update_sha)

        # Rollback snapshot should exist and match original
        rollback_path = self.target_dir / "filesystem.squashfs.rollback"
        self.assertTrue(rollback_path.exists())
        self.assertEqual(compute_sha256(rollback_path), orig_sha)

        # Status command should return 0
        self.assertEqual(cmd_status(self.target_dir), 0)

        # 2. Rollback
        rc = cmd_rollback(self.target_dir)
        self.assertEqual(rc, 0)

        # Active image should be reverted to original SHA-256
        reverted_sha = compute_sha256(self.target_dir / "filesystem.squashfs")
        self.assertEqual(reverted_sha, orig_sha)

    def test_04_real_image_verification(self):
        repo_root = Path(__file__).resolve().parent.parent
        real_squashfs = repo_root / "build_real" / "filesystem.squashfs"
        if real_squashfs.exists():
            print(f"\n[TEST] Testing genuine OS image at: {real_squashfs}")
            rc = cmd_verify_dry_run(real_squashfs)
            self.assertEqual(rc, 0)


if __name__ == "__main__":
    unittest.main()
