"""Offline, dependency-free checks for the one-time Rust split tool."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("split-rs.py")
spec = importlib.util.spec_from_file_location("split_rs", SCRIPT)
tool = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tool)

FIXTURE = '''//! Crate docs stay in the parent.
use std::fmt::{
    self, Debug,
};

// Keep this comment with the type.
#[derive(Clone)]
struct Palette {
    value: u8,
}

impl Palette {
    fn value(&self) -> u8 {
        self.value
    }
}

impl Debug for Palette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// Uneven braces and item-like lines in literals must stay opaque.
const fn rgb() -> &'static str {
    /* nested /* { } */ comment */
    r###"}
fn fake() { }
#[cfg(test)]
"###
}

pub fn kept() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check() {
        assert!(rgb().contains("fake"));
    }
}
'''


class SplitTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source = self.root / "shell.rs"
        self.source.write_text(FIXTURE)
        self.manifest = self.root / "split.toml"

    def test_moves_attributes_comments_inherent_and_trait_impls_losslessly(self):
        self.manifest.write_text('[kit]\nitems = ["Palette", "rgb"]\n')
        tool.split(self.source, self.manifest)
        parent = self.source.read_text()
        moved = (self.root / "shell/kit.rs").read_text()
        self.assertIn("//! Crate docs stay in the parent.", parent)
        self.assertIn("pub fn kept() {}", parent)
        self.assertIn("mod kit;\nuse kit::*;", parent)
        self.assertTrue(moved.startswith("use super::*;\nuse std::fmt::{"))
        self.assertIn("// Keep this comment with the type.\n#[derive(Clone)]\npub(super) struct Palette", moved)
        self.assertIn("impl Palette {", moved)
        self.assertIn("impl Debug for Palette {", moved)
        self.assertIn("pub(super) const fn rgb()", moved)
        self.assertNotIn("pub(super) impl", moved)
        self.assertNotIn("fn fake", parent)
        _, before = tool.items(FIXTURE)
        _, after = tool.items(moved)
        expected = [item[2] for item in before if item[0] in ("Palette", "rgb")]
        actual = [item[2].replace("pub(super) ", "", 1) for item in after if item[0] in ("Palette", "rgb")]
        tool.check_conservation(expected, [], actual)

    def test_inline_test_module_keeps_attributes_and_lexical_level(self):
        self.source = self.root / "lib.rs"
        self.source.write_text(FIXTURE)
        self.manifest.write_text('[tests]\nitems = ["tests"]\n')
        tool.split(self.source, self.manifest)
        self.assertIn("#[cfg(test)]\nmod tests;", self.source.read_text())
        body = (self.root / "tests.rs").read_text()
        self.assertFalse(body.startswith("\n"))
        self.assertIn("    use super::*;", body)
        self.assertIn("    fn check() {", body)
        self.assertNotIn("mod tests", body)

    def test_rejections_write_nothing(self):
        for manifest in ('[kit]\nitems = ["missing"]', '[kit]\nitems = ["Palette", "Palette"]',
                         '["../outside"]\nitems = ["Palette"]', '[kit]\nitems = []'):
            with self.subTest(manifest=manifest):
                self.manifest.write_text(manifest)
                with self.assertRaises(ValueError):
                    tool.split(self.source, self.manifest)
                self.assertEqual(self.source.read_text(), FIXTURE)
                self.assertFalse((self.root / "shell").exists())

    def test_duplicate_declarations_and_existing_outputs_are_rejected(self):
        self.manifest.write_text('[kit]\nitems = ["Palette"]')
        self.source.write_text(FIXTURE + '\nstruct Palette;\n')
        with self.assertRaisesRegex(ValueError, "found 2"):
            tool.split(self.source, self.manifest)
        self.source.write_text(FIXTURE)
        (self.root / "shell").mkdir()
        output = self.root / "shell/kit.rs"
        output.write_text("preserve me")
        with self.assertRaisesRegex(ValueError, "destination exists"):
            tool.split(self.source, self.manifest)
        self.assertEqual(output.read_text(), "preserve me")
        self.assertEqual(self.source.read_text(), FIXTURE)

    def test_conservation_rejects_missing_duplicate_or_changed_bytes(self):
        for moved in ([], ["original", "original"], ["changed"]):
            with self.subTest(moved=moved), self.assertRaisesRegex(ValueError, "conservation"):
                tool.check_conservation(["original"], [], moved)

    def test_generic_impl_follows_its_declared_type(self):
        _, parsed = tool.items("struct Item<T>(T);\nimpl<T: Copy> Clone for Item<T> {\n}\n")
        self.assertEqual([(item[0], item[1]) for item in parsed], [("Item", "struct"), ("Item", "impl")])

    def test_destination_module_can_share_a_function_name(self):
        self.manifest.write_text('[rgb]\nitems = ["rgb"]')
        tool.split(self.source, self.manifest)
        self.assertIn("mod rgb;\nuse rgb::*;", self.source.read_text())
        self.assertIn("pub(super) const fn rgb()", (self.root / "shell/rgb.rs").read_text())

    def test_cli_reports_missing_item_without_writing(self):
        self.manifest.write_text('[kit]\nitems = ["missing"]')
        result = subprocess.run([sys.executable, str(SCRIPT), str(self.source), str(self.manifest)],
                                capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("expected exactly one item, found 0", result.stderr)
        self.assertEqual(self.source.read_text(), FIXTURE)


if __name__ == "__main__":
    unittest.main()
