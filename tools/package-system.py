#!/usr/bin/env python3
"""Build the same bounded System overlay for manual installs."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import stat
import subprocess
import zipfile

MAX_FILE = 32 * 1024 * 1024
MAX_TOTAL = 64 * 1024 * 1024


def package(root, binary, output, commit, channel, version):
    if not re.fullmatch(r'[0-9a-f]{40}', commit):
        raise ValueError('Expected full source commit')
    if channel not in ('preview', 'development', 'stable'):
        raise ValueError('Unknown channel')
    if not re.fullmatch(r'[A-Za-z0-9._-]{1,80}', version):
        raise ValueError('Invalid version')
    elf = binary.read_bytes()
    if elf[:6] != b'\x7fELF\x02\x01' or elf[18:20] != b'\xb7\x00':
        raise ValueError('Expected little-endian ELF64 AArch64 frontend')
    files = {'System/slot': (elf, 0o755)}
    tracked = subprocess.check_output(
        ['git', 'ls-files', '-z', 'card/System'], cwd=root).decode().split('\0')
    for name in filter(None, tracked):
        path = root / name
        if path.is_symlink() or not path.is_file():
            raise ValueError('Only regular System files may ship')
        relative = name.removeprefix('card/')
        if relative == 'System/slot' or relative.endswith('.so'):
            raise ValueError('Frontend overlay must not contain emulator cores')
        files[relative] = (path.read_bytes(), 0o755 if path.stat().st_mode & 0o111 else 0o644)
    files['System/build.json'] = ((json.dumps(dict(schema=1, commit=commit,
        channel=channel, version=version), sort_keys=True) + '\n').encode(), 0o644)
    if any(len(data) > MAX_FILE for data, _ in files.values()) or sum(len(data) for data, _ in files.values()) > MAX_TOTAL:
        raise ValueError('System overlay exceeds update limits')
    manifest = dict(schema=1, target='h700-aarch64', repository='jgraving/slot',
                    commit=commit, channel=channel, version=version,
                    files=[dict(path=name, size=len(data), sha256=hashlib.sha256(data).hexdigest(), mode=mode)
                           for name, (data, mode) in sorted(files.items())])
    files['manifest.json'] = ((json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode(), 0o644)
    files['LICENSE'] = ((root / 'LICENSE').read_bytes(), 0o644)
    files['SOURCE.txt'] = (f'https://github.com/jgraving/slot/archive/{commit}.tar.gz\n'.encode(), 0o644)
    files['INSTALL.txt'] = (b'Power off and back up your existing System folder.\nMerge System into System on SD2; replace matching files.\nKeep installed emulator cores and other existing files.\nGames, labels, settings and saves are unchanged.\nThis is a frontend overlay, not a BaseOS or fresh-card image.\n', 0o644)
    output.mkdir(parents=True, exist_ok=True)
    archive = output / 'slot-System.zip'
    with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED) as z:
        for name, (data, mode) in sorted(files.items()):
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (stat.S_IFREG | mode) << 16
            info.create_system = 3
            z.writestr(info, data)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (output / 'SHA256SUMS').write_text(f'{digest}  slot-System.zip\n')
    (output / 'manifest.json').write_bytes(files['manifest.json'][0])


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=Path('dist-system'))
    parser.add_argument('--commit', required=True)
    parser.add_argument('--channel', required=True)
    parser.add_argument('--version', required=True)
    args = parser.parse_args()
    package(Path.cwd(), args.binary, args.output, args.commit, args.channel, args.version)
