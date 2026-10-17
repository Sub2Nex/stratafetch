"""Exercise real upload archives and identity configuration in disposable copies."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile

SOURCE = Path(__file__).resolve().parent.parent

class PackagingTests(unittest.TestCase):
    def setUp(self):
        testing = SOURCE/'.smoke'
        testing.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix='extension-packaging-', dir=testing)
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        assert self.root.resolve().is_relative_to(testing.resolve())
        for directory in ['extensions', 'packaging', 'docs']:
            shutil.copytree(SOURCE/directory, self.root/directory)
        shutil.copyfile(SOURCE/'PRIVACY.md', self.root/'PRIVACY.md')

    def run_tool(self, name, *args, ok=True):
        result = subprocess.run([sys.executable, str(self.root/'packaging'/name), *args], capture_output=True, text=True)
        self.assertEqual(result.returncode == 0, ok, result.stderr)
        return result

    def test_initial_submission_archives_and_checksums(self):
        self.run_tool('package-extensions.py', '--output', str(self.root/'output'))
        output = self.root/'output'
        for name in ['stratafetch-chrome-webstore.zip', 'stratafetch-edge-addons.zip', 'stratafetch-firefox-amo.zip', 'stratafetch-firefox-unsigned-dev.xpi']:
            with zipfile.ZipFile(output/name) as archive:
                self.assertIsNone(archive.testzip())
                manifest = json.loads(archive.read('manifest.json'))
                self.assertNotIn('key', manifest)
                self.assertTrue(all(name in archive.namelist() for name in manifest['icons'].values()))
                self.assertEqual(len(archive.namelist()), 11)
                self.assertEqual(manifest['default_locale'], 'en')
                self.assertIn('locale.js', archive.namelist())
                self.assertIn('_locales/sv/messages.json', archive.namelist())
        self.assertTrue((output/'unpacked/chromium/manifest.json').exists())
        self.assertTrue((output/'unpacked/firefox/manifest.json').exists())
        for line in (output/'SHA256SUMS').read_text().splitlines():
            digest, name = line.split('  ', 1)
            self.assertEqual(hashlib.sha256((output/name).read_bytes()).hexdigest(), digest)

    def test_recording_store_ids_never_changes_local_development(self):
        originals = {name: (self.root/name).read_bytes() for name in ['extensions/local-ids.json', 'extensions/chromium/manifest.json']}
        for browser, letter in [('chrome', 'b'), ('edge', 'c')]:
            self.run_tool('configure-store-id.py', browser, '--id', letter*32)
        stores = json.loads((self.root/'extensions/store-ids.json').read_text())
        self.assertEqual(stores['chrome']['id'], 'b'*32)
        self.assertEqual(stores['edge']['id'], 'c'*32)
        for name, data in originals.items():
            self.assertEqual((self.root/name).read_bytes(), data)
        self.run_tool('package-extensions.py', '--output', str(self.root/'output'))

    def test_invalid_private_or_mismatched_keys_do_not_change_identity(self):
        path = self.root/'extensions/store-ids.json'
        original = path.read_bytes()
        self.run_tool('configure-store-id.py', 'chrome', '--id', 'invalid', ok=False)
        development = json.loads((self.root/'extensions/local-ids.json').read_text())
        self.run_tool('configure-store-id.py', 'chrome', '--id', development['chromium'], ok=False)
        key = self.root/'key.txt'
        key.write_text('-----BEGIN PRIVATE KEY-----\nnot-a-key')
        self.run_tool('configure-store-id.py', 'chrome', '--id', 'b'*32, '--public-key-file', str(key), ok=False)
        key.write_text(json.loads((self.root/'extensions/chromium/manifest.json').read_text())['key'])
        self.run_tool('configure-store-id.py', 'chrome', '--id', 'b'*32, '--public-key-file', str(key), ok=False)
        self.assertEqual(path.read_bytes(), original)

    def test_upload_archives_are_deterministic(self):
        self.run_tool('package-extensions.py', '--output', str(self.root/'first'))
        self.run_tool('package-extensions.py', '--output', str(self.root/'second'))
        for path in (self.root/'first').glob('*.zip'):
            self.assertEqual(path.read_bytes(), (self.root/'second'/path.name).read_bytes())

if __name__ == '__main__':
    unittest.main()
