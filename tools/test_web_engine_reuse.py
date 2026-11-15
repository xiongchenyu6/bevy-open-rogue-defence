import functools
import hashlib
import http.server
import json
import tempfile
import threading
import unittest
from pathlib import Path

from reuse_web_engine import ENGINE_FILES, reuse
from web_engine_inputs import fingerprint


class FingerprintTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        for name, content in {
            'Cargo.toml': '[package]\nname="fixture"\n', 'Cargo.lock': 'locked-dependencies',
            'src/main.rs': 'fn main() {}', 'vendor/lib/src/lib.rs': 'pub fn dependency() {}',
            '.cargo/config.toml': '[build]', 'build.rs': 'fn main() {}',
            'rust-toolchain.toml': '[toolchain]\nchannel="stable"',
            'scripts/build-wasm.sh': 'cargo build --profile wasm-release',
            'assets/data/campaign.json': '{"damage":1}', 'web/index.html': '<html>v1</html>',
            'docs/design.md': 'notes', 'assets/fonts/font.ttf': 'font-bytes',
        }.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)

    def tearDown(self):
        self.temporary.cleanup()

    def test_runtime_content_changes_reuse_engine_but_compile_inputs_do_not(self):
        original = fingerprint(self.root)[0]
        for name in ['assets/data/campaign.json', 'web/index.html', 'docs/design.md']:
            (self.root / name).write_text('changed')
            self.assertEqual(fingerprint(self.root)[0], original, name)
        for name in ['src/main.rs', 'vendor/lib/src/lib.rs', '.cargo/config.toml',
                     'Cargo.toml', 'Cargo.lock', 'build.rs', 'rust-toolchain.toml', 'scripts/build-wasm.sh']:
            path = self.root / name
            before = path.read_bytes()
            path.write_bytes(before + b'changed')
            self.assertNotEqual(fingerprint(self.root)[0], original, name)
            path.write_bytes(before)
            self.assertEqual(fingerprint(self.root)[0], original)

    def test_runtime_literal_includes_after_test_module_are_tracked_but_test_only_are_not(self):
        (self.root / 'src/main.rs').write_text('''
#[cfg(test)]
mod tests {
    // } quoted and commented braces must not end the test item.
    /* nested /* } */ comment */
    const S: &str = r#" { } "#;
    const W: &str = include_str!("../web/index.html");
    const C: &str = include_str!("../assets/data/campaign.json");
}
const RUNTIME: &[u8] = include_bytes!(r#"../assets/fonts/font.ttf"#);
fn main() {}
''')
        original, files = fingerprint(self.root)
        self.assertIn('assets/fonts/font.ttf', [file['path'] for file in files])
        self.assertNotIn('web/index.html', [file['path'] for file in files])
        for name in ['web/index.html', 'assets/data/campaign.json']:
            (self.root / name).write_text('changed')
            self.assertEqual(fingerprint(self.root)[0], original)
        (self.root / 'assets/fonts/font.ttf').write_text('new font')
        self.assertNotEqual(fingerprint(self.root)[0], original)
        (self.root / 'src/main.rs').write_text('const C: &str = include_str!("../assets/data/campaign.json");')
        embedded = fingerprint(self.root)[0]
        (self.root / 'assets/data/campaign.json').write_text('actually embedded change')
        self.assertNotEqual(fingerprint(self.root)[0], embedded)

    def test_nonliteral_include_requires_explicit_inputs_and_clone_path_is_irrelevant(self):
        original = fingerprint(self.root)[0]
        import shutil
        with tempfile.TemporaryDirectory() as copy:
            shutil.copytree(self.root, Path(copy) / 'clone')
            self.assertEqual(fingerprint(Path(copy) / 'clone')[0], original)
        (self.root / 'src/main.rs').write_text('const C: &str = include_str!(concat!("../assets/", "data/campaign.json"));')
        with self.assertRaisesRegex(ValueError, 'Nonliteral include'):
            fingerprint(self.root)


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    requests = []
    def do_GET(self):
        self.requests.append(self.path)
        super().do_GET()
    def log_message(self, *_):
        pass


class ReuseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.site = self.root / 'site'
        self.site.mkdir()
        self.output = self.root / 'dist'
        self.digest = hashlib.sha256(b'inputs').hexdigest()
        self.content = {name: (name + '-binary-content').encode() for name in ENGINE_FILES}
        self.manifest = {'revision': 'published-revision', 'files': []}
        for name, data in self.content.items():
            digest = hashlib.sha256(data).hexdigest()
            relative = f'_resources/{digest}/{name}'
            path = self.site / relative
            path.parent.mkdir(parents=True)
            path.write_bytes(data)
            self.manifest['files'].append({'path': name, 'url': relative, 'sha256': digest, 'bytes': len(data)})
        wasm = self.manifest['files'][1]
        self.receipt = {'revision': 'published-revision', 'engine_inputs_sha256': self.digest,
                        'wasm_sha256': wasm['sha256'], 'wasm_bytes': wasm['bytes']}
        self.publish_metadata()
        QuietHandler.requests = []
        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(QuietHandler, directory=str(self.site)))
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.url = f'http://127.0.0.1:{self.server.server_port}/'

    def publish_metadata(self):
        (self.site / 'build.json').write_text(json.dumps(self.receipt))
        (self.site / 'asset-manifest.json').write_text(json.dumps(self.manifest))

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()
        self.temporary.cleanup()

    def test_matching_inputs_reuse_exact_verified_engine_files(self):
        result = reuse(self.url, self.output, self.digest)
        self.assertEqual(result['revision'], self.receipt['revision'])
        self.assertEqual(sorted(path.name for path in self.output.iterdir()), sorted(ENGINE_FILES))
        for name, data in self.content.items():
            self.assertEqual((self.output / name).read_bytes(), data)
        self.assertEqual(QuietHandler.requests[:2], ['/build.json', '/asset-manifest.json'])
        self.assertTrue(all(path.startswith('/_resources/') for path in QuietHandler.requests[2:]))
        self.assertFalse(list(self.root.glob('.engine-reuse-*')))

    def test_changed_inputs_or_old_receipt_do_not_download_engine(self):
        for published in [None, hashlib.sha256(b'other-inputs').hexdigest()]:
            with self.subTest(published=published):
                QuietHandler.requests.clear()
                self.receipt['engine_inputs_sha256'] = published
                self.publish_metadata()
                with self.assertRaisesRegex(ValueError, 'inputs differ'):
                    reuse(self.url, self.output, self.digest)
                self.assertEqual(QuietHandler.requests, ['/build.json'])
                self.assertFalse(self.output.exists())

    def test_cli_unavailable_release_requests_rebuild_without_partial_files(self):
        import subprocess
        import sys
        (self.site / 'build.json').unlink()
        result = subprocess.run([sys.executable, str(Path(__file__).with_name('reuse_web_engine.py')),
                                 '--base-url', self.url, '--output', str(self.output),
                                 '--fingerprint', self.digest], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn('rebuilding', result.stderr)
        self.assertNotIn('Traceback', result.stderr)
        self.assertFalse(self.output.exists())
        self.assertFalse(list(self.root.glob('.engine-reuse-*')))

    def test_same_size_tamper_discards_all_staged_files_and_preserves_existing_output(self):
        self.output.mkdir()
        (self.output / 'keep.txt').write_text('unchanged')
        bad = self.manifest['files'][-1]
        (self.site / bad['url']).write_bytes(b'X' * bad['bytes'])
        with self.assertRaisesRegex(ValueError, 'integrity check failed'):
            reuse(self.url, self.output, self.digest)
        self.assertEqual(list(path.name for path in self.output.iterdir()), ['keep.txt'])
        self.assertFalse(list(self.root.glob('.engine-reuse-*')))

    def test_mixed_release_nonimmutable_url_and_truncated_download_fail_closed(self):
        self.receipt['revision'] = 'different-revision'
        self.publish_metadata()
        with self.assertRaisesRegex(ValueError, 'revisions differ'):
            reuse(self.url, self.output, self.digest)
        self.receipt['revision'] = self.manifest['revision']
        entry = self.manifest['files'][0]
        immutable = entry['url']
        entry['url'] = entry['path']
        self.publish_metadata()
        with self.assertRaisesRegex(ValueError, 'content-addressed'):
            reuse(self.url, self.output, self.digest)
        entry['url'] = immutable
        self.publish_metadata()
        (self.site / immutable).write_bytes(b'partial')
        with self.assertRaisesRegex(ValueError, 'integrity check failed'):
            reuse(self.url, self.output, self.digest)
        self.assertFalse(self.output.exists())
        self.assertFalse(list(self.root.glob('.engine-reuse-*')))


class BuildWrapperTests(unittest.TestCase):
    def test_actual_wrapper_build_receipt_and_content_only_reuse_without_compiler(self):
        import os
        import shutil
        import subprocess
        repo = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'project'
            for directory in ['scripts', 'tools', 'src', 'web', 'assets/fonts', 'assets/data', 'THIRD_PARTY_NOTICES', 'fakebin']:
                (root / directory).mkdir(parents=True, exist_ok=True)
            for name in ['scripts/build-wasm.sh', 'tools/web_engine_inputs.py', 'tools/reuse_web_engine.py', 'tools/generate_web_manifest.py']:
                shutil.copyfile(repo / name, root / name)
            (root / 'Cargo.toml').write_text('[package]\nname="fixture"')
            (root / 'Cargo.lock').write_text('[[package]]\nname = "wasm-bindgen"\nversion = "0.2.126"\n')
            (root / 'src/main.rs').write_text('fn main() {}')
            (root / 'web/index.html').write_text('<html>original</html>')
            (root / 'web/sw.js').write_text('const MANIFEST = /* BUILD_MANIFEST */ null;')
            (root / 'assets/fonts/rogue_commander_ui.ttf').write_bytes(b'font')
            (root / 'assets/fonts/OFL.txt').write_text('license')
            (root / 'assets/data/campaign.json').write_text('{"damage":1}')
            commands = {
                'cargo': '#!/usr/bin/env bash\nprintf "cargo\\n" >> "$FIXTURE_CALL_LOG"\nmkdir -p target/wasm32-unknown-unknown/wasm-release\nprintf "compiled wasm fixture" > target/wasm32-unknown-unknown/wasm-release/rogue_commander.wasm\n',
                'wasm-bindgen': '#!/usr/bin/env bash\nprintf "bindgen\\n" >> "$FIXTURE_CALL_LOG"\nif [ "$1" = "--version" ]; then echo "wasm-bindgen 0.2.126"; exit 0; fi\ncp target/wasm32-unknown-unknown/wasm-release/rogue_commander.wasm dist/rogue_commander_bg.wasm\nprintf "export default function init() {}" > dist/rogue_commander.js\n',
                'git': '#!/usr/bin/env bash\necho fixture-revision\n',
            }
            def install_commands():
                for name, content in commands.items():
                    script = root / 'fakebin' / name
                    script.write_text(content)
                    script.chmod(0o755)
            install_commands()
            env = os.environ.copy()
            env.pop('WASM_REUSE_BASE_URL', None)
            env['PATH'] = str(root / 'fakebin') + os.pathsep + env['PATH']
            env['FIXTURE_CALL_LOG'] = str(root / 'calls.log')
            def build():
                return subprocess.run(['bash', 'scripts/build-wasm.sh'], cwd=root, env=env, capture_output=True, text=True)
            first = build()
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            receipt = json.loads((root / 'dist/build.json').read_text())
            self.assertEqual(receipt['engine_inputs_sha256'], fingerprint(root)[0])
            self.assertIn('cargo', (root / 'calls.log').read_text())
            engine = {name: (root / 'dist' / name).read_bytes() for name in ENGINE_FILES}
            published = Path(temporary) / 'published'
            shutil.copytree(root / 'dist', published)
            server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(QuietHandler, directory=str(published)))
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                env['WASM_REUSE_BASE_URL'] = f'http://127.0.0.1:{server.server_port}/'
                (root / 'calls.log').write_text('')
                for name in ['cargo', 'wasm-bindgen']:
                    (root / 'fakebin' / name).write_text('#!/usr/bin/env bash\nprintf "UNEXPECTED compiler call\\n" >> "$FIXTURE_CALL_LOG"\nexit 99\n')
                (root / 'web/index.html').write_text('<html>content-only update</html>')
                (root / 'assets/data/campaign.json').write_text('{"damage":2}')
                reused = build()
                self.assertEqual(reused.returncode, 0, reused.stdout + reused.stderr)
                self.assertIn('using the verified published engine', reused.stdout)
                self.assertEqual((root / 'calls.log').read_text(), '')
                self.assertEqual(json.loads((root / 'dist/build.json').read_text())['engine_inputs_sha256'], receipt['engine_inputs_sha256'])
                for name, content in engine.items():
                    self.assertEqual((root / 'dist' / name).read_bytes(), content)
                self.assertIn('content-only update', (root / 'dist/index.html').read_text())
                self.assertEqual(json.loads((root / 'dist/assets/data/campaign.json').read_text())['damage'], 2)
                install_commands()
                (root / 'src/main.rs').write_text('fn main() { /* changed logic */ }')
                rebuilt = build()
                self.assertEqual(rebuilt.returncode, 0, rebuilt.stdout + rebuilt.stderr)
                self.assertIn('cargo', (root / 'calls.log').read_text())
                new_receipt = json.loads((root / 'dist/build.json').read_text())
                self.assertNotEqual(new_receipt['engine_inputs_sha256'], receipt['engine_inputs_sha256'])
                self.assertEqual(new_receipt['engine_inputs_sha256'], fingerprint(root)[0])
            finally:
                server.shutdown()
                server.server_close()
                thread.join()


if __name__ == '__main__':
    unittest.main()
