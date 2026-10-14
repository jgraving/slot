import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('package_system', Path(__file__).with_name('package-system.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        system = self.root / 'card/System'
        system.mkdir(parents=True)
        (system / 'launch_frontend.sh').write_text('exec slot\n')
        (self.root / 'LICENSE').write_text('test license')
        subprocess.run(['git', '-C', str(self.root), 'add', '.'], check=True)
        # Runtime content must never accidentally ship.
        (system / 'password.txt').write_text('secret')
        self.binary = self.root / 'frontend'
        self.binary.write_bytes(b'\x7fELF\x02\x01' + bytes(12) + b'\xb7\x00' + bytes(64))
        self.out = self.root / 'out'

    def build(self, **kwargs):
        options = dict(root=self.root, binary=self.binary, output=self.out,
                       commit='a' * 40, channel='development', version='dev-123-1')
        options.update(kwargs)
        module.package(**options)

    def test_manifest_matches_every_installed_file_and_no_secrets(self):
        self.build()
        with zipfile.ZipFile(self.out / 'slot-System.zip') as z:
            manifest = json.loads(z.read('manifest.json'))
            self.assertNotIn('System/password.txt', z.namelist())
            self.assertEqual({i['path'] for i in manifest['files']},
                             {p for p in z.namelist() if p.startswith('System/')})
            for record in manifest['files']:
                data = z.read(record['path'])
                self.assertEqual(record['size'], len(data))
                self.assertEqual(record['sha256'], hashlib.sha256(data).hexdigest())
            self.assertEqual(json.loads(z.read('System/build.json'))['commit'], 'a' * 40)
        digest = hashlib.sha256((self.out / 'slot-System.zip').read_bytes()).hexdigest()
        self.assertEqual((self.out / 'SHA256SUMS').read_text(), digest + '  slot-System.zip\n')

    def test_reproducible(self):
        self.build()
        before = (self.out / 'slot-System.zip').read_bytes()
        self.build()
        self.assertEqual(before, (self.out / 'slot-System.zip').read_bytes())

    def test_wrong_architecture_and_identity_rejected(self):
        for kwargs in [dict(commit='main'), dict(channel='oops'), dict(version='../foo')]:
            with self.assertRaises(ValueError):
                self.build(**kwargs)
        self.binary.write_bytes(b'host binary')
        with self.assertRaises(ValueError):
            self.build()

    def test_tracked_symlink_rejected(self):
        path = self.root / 'card/System/launch_frontend.sh'
        path.unlink()
        path.symlink_to(self.binary)
        with self.assertRaises(ValueError):
            self.build()


if __name__ == '__main__':
    unittest.main()
