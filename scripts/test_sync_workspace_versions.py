#!/usr/bin/env python3
"""Regression tests for workspace version propagation in release.sh."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("sync_workspace_versions.py")


class VersionSyncTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "crates" / "meridian-platform-winit").mkdir(parents=True)
        (self.root / "crates" / "meridian-platform-winit" / "Cargo.toml").write_text(
            '[package]\nname = "meridian-platform-winit"\nversion = "0.13.0"\n',
            encoding="utf-8",
        )
        (self.root / "Cargo.toml").write_text(
            '[workspace]\n'
            '[workspace.dependencies]\n'
            'meridian-platform-winit = { path = "crates/meridian-platform-winit", version = "0.12.1" }\n'
            '\n[profile.release]\nlto = true\n',
            encoding="utf-8",
        )

    def run_sync(self, *args):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.root), *args],
            check=False,
            capture_output=True,
            text=True,
        )

    def test_patch_qualified_old_requirement_is_replaced(self):
        self.assertNotEqual(self.run_sync("--check").returncode, 0)
        self.assertEqual(self.run_sync("--write").returncode, 0)
        self.assertIn('version = "0.13.0"', (self.root / "Cargo.toml").read_text())
        self.assertEqual(self.run_sync("--check").returncode, 0)

    def test_patch_bump_keeps_minimum_version_in_sync(self):
        member = self.root / "crates" / "meridian-platform-winit" / "Cargo.toml"
        member.write_text(
            '[package]\nname = "meridian-platform-winit"\nversion = "0.13.1"\n',
            encoding="utf-8",
        )
        self.assertEqual(self.run_sync("--write").returncode, 0)
        self.assertIn('version = "0.13.1"', (self.root / "Cargo.toml").read_text())

    def test_missing_member_fails_without_mutating(self):
        original = (self.root / "Cargo.toml").read_text()
        (self.root / "crates" / "meridian-platform-winit" / "Cargo.toml").unlink()
        self.assertNotEqual(self.run_sync("--write").returncode, 0)
        self.assertEqual((self.root / "Cargo.toml").read_text(), original)


if __name__ == "__main__":
    unittest.main()
