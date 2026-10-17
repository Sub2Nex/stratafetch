#!/usr/bin/env python3
"""Sync the canonical checkout to the visible workspace and development unpacked copies.

Never changes archived ZIPs or release payloads. Rejects unknown edits in the
visible workspace rather than overwriting them. Run before building either tree.
"""
from pathlib import Path
import hashlib
import json
import subprocess

repo = Path(__file__).resolve().parent.parent
workspace = repo.parent if repo.name == '.publish' else repo
state_path = repo/'.smoke/development-sync.json'
state = json.loads(state_path.read_text()) if state_path.exists() else {}
paths = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=repo).decode().split('\0')
count = 0
for name in dict.fromkeys(paths):
    if not name:
        continue
    source, destination = repo/name, workspace/name
    if not source.is_file() or source == destination:
        continue
    data = source.read_bytes()
    if destination.exists() and destination.read_bytes() != data:
        previous = subprocess.run(['git', 'show', 'HEAD:'+name], cwd=repo, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        normalize = lambda value: value.replace(b'\r\n', b'\n')
        known_copy = state.get(name) == hashlib.sha256(destination.read_bytes()).hexdigest()
        if not known_copy and (previous.returncode or normalize(destination.read_bytes()) not in (normalize(previous.stdout), normalize(data))):
            raise RuntimeError('Preserved unknown workspace change: '+str(destination))
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
    state[name] = hashlib.sha256(data).hexdigest()
    count += 1

unpacked = 0
for dist in [workspace/'dist', repo/'dist']:
    if not dist.exists():
        continue
    for target in dist.rglob('background.js'):
        if 'unpacked' not in target.relative_to(dist).parts or target.parent.name not in ('chromium', 'firefox'):
            continue
        source = repo/'extensions'/target.parent.name/'background.js'
        target.write_bytes(source.read_bytes())
        assert hashlib.sha256(target.read_bytes()).digest() == hashlib.sha256(source.read_bytes()).digest()
        unpacked += 1
state_path.parent.mkdir(parents=True, exist_ok=True)
state_path.write_text(json.dumps(state, indent=2), encoding='utf-8')
print(f'Synchronized {count} workspace files and {unpacked} development unpacked background scripts. Archived release assets unchanged.')
