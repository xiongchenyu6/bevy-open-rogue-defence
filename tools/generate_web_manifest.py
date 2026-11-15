#!/usr/bin/env python3
"""Seal the deployed runtime files into a reproducible service-worker manifest."""
import argparse
import hashlib
import json
from pathlib import Path

TOKEN = '/* BUILD_MANIFEST */ null'
EXCLUDED = {'sw.js', 'asset-manifest.json', 'build.json', '.nojekyll'}


def generate(root: Path, revision: str, template: Path):
    files = []
    for path in sorted(root.rglob('*')):
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        if relative in EXCLUDED:
            continue
        data = path.read_bytes()
        entry = {'path': relative, 'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)}
        if relative.endswith('.wasm'):
            entry['encoding'] = 'raw'
        elif relative.endswith('.wasm.gz'):
            entry['encoding'] = 'gzip'
        files.append(entry)
    if not any(f['path'] == 'index.html' for f in files):
        raise ValueError('index.html is required')
    canonical = json.dumps(files, sort_keys=True, separators=(',', ':'))
    manifest = {'version': hashlib.sha256(canonical.encode()).hexdigest(),
                'revision': revision, 'files': files}
    source = template.read_text()
    if source.count(TOKEN) != 1:
        raise ValueError('SW template must contain exactly one manifest placeholder')
    serialized = json.dumps(manifest, separators=(',', ':'))
    (root / 'asset-manifest.json').write_text(serialized + '\n')
    (root / 'sw.js').write_text(source.replace(TOKEN, serialized))
    return manifest


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--template', type=Path, default=Path('web/sw.js'))
    args = parser.parse_args()
    generate(args.root, args.revision, args.template)
