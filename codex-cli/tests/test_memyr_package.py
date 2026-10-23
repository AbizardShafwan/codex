import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


BUILDER = Path(__file__).resolve().parents[1] / "scripts/build_npm_package.py"


class MemyrPackageTest(unittest.TestCase):
    def stage(self, root: Path, *extra: str) -> Path:
        staging = root / "staging"
        subprocess.run(
            [
                sys.executable, str(BUILDER), "--package", "codex",
                "--version", "1.2.3", "--staging-dir", str(staging), *extra,
            ],
            check=True, capture_output=True, text=True, timeout=15,
        )
        return staging

    def test_private_package_exposes_only_memyr_without_upstream_dependencies(self):
        with tempfile.TemporaryDirectory() as temp:
            staging = self.stage(Path(temp))
            manifest = json.loads((staging / "package.json").read_text())
            self.assertTrue(manifest["private"])
            self.assertEqual(manifest["bin"], {"memyr": "bin/memyr.js"})
            self.assertNotIn("optionalDependencies", manifest)
            self.assertTrue((staging / "bin/memyr.js").is_file())
            self.assertFalse((staging / "bin/codex.js").exists())

    def test_explicit_vendor_tree_is_preserved_in_private_package(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            vendor = root / "vendor"
            relative = Path("x86_64-unknown-linux-musl/bin/memyr")
            binary = vendor / relative
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b"local Memyr binary fixture")
            binary.chmod(0o755)
            staging = self.stage(root, "--vendor-src", str(vendor))
            installed = staging / "vendor" / relative
            self.assertEqual(installed.read_bytes(), binary.read_bytes())
            self.assertEqual(installed.stat().st_mode & 0o777, 0o755)


if __name__ == "__main__":
    unittest.main()
