"""Runtime model scanning must ignore real test modules, not missing production assets."""

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import verify_models


class RuntimeModelReferenceTests(unittest.TestCase):
    def test_actual_enemy_flash_fixture_is_not_a_runtime_asset(self):
        source = (verify_models.ROOT / "src/game/hit_feedback/enemy_flash.rs").read_text()
        self.assertIn('"models/flash-test.glb', source)
        self.assertNotIn("flash-test.glb", verify_models.runtime_model_names(source))

    def test_missing_production_after_test_module_is_still_rejected(self):
        source = '''
#[cfg(test)]
mod tests { fn fixture() { load("models/test-only.glb#Scene0"); } }
fn production() { load("models/flash-test.glb#Animation3"); }
'''
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "source.rs"
            rust.write_text(source)
            with self.assertRaisesRegex(AssertionError, "Missing runtime model: flash-test.glb"):
                verify_models.check_runtime_model_files([rust], root)
            (root / "flash-test.glb").touch()
            self.assertEqual(
                verify_models.check_runtime_model_files([rust], root),
                {"flash-test.glb"},
            )

    def test_nested_braces_raw_strings_characters_and_comments_do_not_end_module(self):
        source = r'''
// #[cfg(test)] mod counterfeit { "models/comment-only.glb" }
/* outer { /* nested } */ "models/block-comment.glb" } */
const PRETEND: &str = r##"#[cfg(test)] mod pretend { }"##;
fn before() { load(r#"models/before.glb#Scene7"#); }
#[cfg /* legal whitespace comment */ (test)]
#[allow(dead_code)]
#[doc = r###" stray } and " quoted {"###]
pub(crate) mod tests {
    const RAW: &str = r##" } "# "models/test-raw.glb" { "##;
    fn fixture<'a>() {
        let chars = ('}', '{', '"', '\u{7b}');
        let escaped = "\" } { \\";
        /* } /* { } */ } */
        if true { load("models/nested-test.glb#Animation1"); }
        mod nested { fn fixture() { load("models/deeper-test.glb"); } }
    }
}
pub fn after() { load("models/after.glb#Animation2"); }
'''
        self.assertEqual(
            verify_models.runtime_model_names(source),
            {"before.glb", "after.glb"},
        )

    def test_production_modules_and_runtime_enabled_cfg_remain_checked(self):
        source = '''
mod production { fn model() { load("models/normal.glb#Scene0"); } }
#[cfg(any(test, feature = "runtime"))]
mod shared { fn model() { load("models/shared.glb#Animation2"); } }
#[cfg(test)]
mod tests { fn model() { load("models/not-shipped.glb"); } }
fn after() { load("models/tail.glb"); }
'''
        self.assertEqual(
            verify_models.runtime_model_names(source),
            {"normal.glb", "shared.glb", "tail.glb"},
        )

    def test_multiple_test_modules_preserve_interleaved_production(self):
        source = '''
#[cfg(test)] mod first { mod inner { const X: &str = "models/fixture1.glb"; } }
const A: &str = "models/middle.glb";
#[cfg(test)] #[allow(unused)] mod second { const X: &str = "models/fixture2.glb"; }
const B: &str = r##"models/final.glb#Scene23"##;
'''
        self.assertEqual(
            verify_models.runtime_model_names(source),
            {"middle.glb", "final.glb"},
        )


if __name__ == "__main__":
    unittest.main()
