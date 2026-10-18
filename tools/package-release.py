#!/usr/bin/env python3
"""Validate an ARM64 card tree and produce the public release downloads."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import zipfile


def package(root: Path, output: Path, source: str) -> None:
    if not re.fullmatch(r'[0-9a-f]{40}', source):
        raise ValueError('source must be a full Git commit SHA')
    root = root.resolve()
    output = output.resolve()
    if output == root or root in output.parents:
        raise ValueError('output must be outside the card tree')
    system = root / 'System'
    for name in ('slot', 'mgba_libretro.so', 'gpsp_libretro.so', 'fsck.exfat'):
        binary = system / name
        header = binary.read_bytes()[:20]
        if len(header) < 20 or header[:6] != b'\x7fELF\x02\x01' or int.from_bytes(header[18:20], 'little') != 183:
            raise ValueError(f'{name} must be a little-endian ELF64 AArch64 binary')
    if not (system / 'slot').stat().st_mode & 0o111:
        raise ValueError('System/slot must be executable')
    licenses = system / 'licenses'
    if not any(licenses.glob('gpsp-*.tar.gz')) or not any(licenses.glob('gpsp-*.meta')) or not any(licenses.glob('mgba-*.meta')):
        raise ValueError('missing emulator source archive or provenance')
    if not any(licenses.glob('gpsp-*.patch')) or not any(licenses.glob('mgba-*.patch')):
        raise ValueError('missing emulator source patches')
    if not any(licenses.glob('exfatprogs-*.tar.gz')) or not (licenses / 'exfatprogs-GPL-2.0.txt').is_file():
        raise ValueError('missing SD repair tool source or license')
    metadata = {
        'repository': 'jgraving/slot',
        'source_commit': source,
        'architecture': 'aarch64',
        'run_url': f'https://github.com/jgraving/slot/actions/runs/{os.environ.get("GITHUB_RUN_ID", "local")}',
    }
    (system / 'build-info.json').write_text(json.dumps(metadata, indent=2) + '\n')
    output.mkdir(parents=True, exist_ok=True)
    (output / 'build-info.json').write_bytes((system / 'build-info.json').read_bytes())
    for filename, subtree in [('slot-System.zip', system), ('slot-SD2.zip', root)]:
        with zipfile.ZipFile(output / filename, 'w', zipfile.ZIP_DEFLATED) as archive:
            for path in sorted(subtree.rglob('*')):
                if path.is_symlink():
                    raise ValueError(f'unexpected symlink: {path}')
                archive.write(path, path.relative_to(root))
        with zipfile.ZipFile(output / filename) as archive:
            if archive.testzip() is not None:
                raise ValueError(f'corrupt archive: {filename}')
            if 'System/slot' not in archive.namelist():
                raise ValueError(f'incorrect archive layout: {filename}')
    files = ['slot-System.zip', 'slot-SD2.zip', 'build-info.json']
    (output / 'SHA256SUMS').write_text(''.join(
        f'{hashlib.sha256((output / name).read_bytes()).hexdigest()}  {name}\n' for name in files
    ))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('source')
    args = parser.parse_args()
    package(args.root, args.output, args.source)
