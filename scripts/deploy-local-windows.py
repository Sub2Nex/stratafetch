#!/usr/bin/env python3
"""Deploy an already built matching package; never touches download state.

Explicit --install is required. Stop StrataFetch first. Normal distribution
uses the setup installer; this is for authorized local development upgrades.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import tomllib

repo = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('--package', type=Path, default=repo/'dist/stratafetch-windows-x64')
parser.add_argument('--install', action='store_true')
args = parser.parse_args()
if os.name != 'nt':
    parser.error('Windows only')
package = args.package.resolve()
installed = Path(os.environ['LOCALAPPDATA'])/'Programs/StrataFetch'
version = tomllib.loads((repo/'Cargo.toml').read_text(encoding='utf-8'))['workspace']['package']['version']
if f'StrataFetch {version}' not in (package/'BUILD.txt').read_text():
    raise ValueError('Package/source versions differ')
bins = ['stratafetch.exe', 'stratafetch-desktop.exe', 'stratafetch-browser-host.exe']
subprocess.run([sys.executable, str(repo/'scripts/verify-windows-manifest.py'), *[str(package/name) for name in bins]], check=True)
files = [Path(name) for name in bins + ['BUILD.txt']]
files += [path.relative_to(package) for path in (package/'extensions').rglob('*') if path.is_file()]
for name in bins:
    target = installed/name
    if target.exists():
        with target.open('r+b'):
            pass  # Windows rejects write access to a running executable.
if not args.install:
    print('Validated matching package. Use --install to deploy into '+str(installed))
    sys.exit(0)
backup = repo/'.smoke/runtime-backup'/time.strftime('%Y%m%d-%H%M%S')
changed = []
try:
    for relative in files:
        target = installed/relative
        if target.exists():
            saved = backup/relative
            saved.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(target, saved)
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_name(target.name+'.strata-update')
        shutil.copy2(package/relative, temporary)
        os.replace(temporary, target)
        changed.append(relative)
    subprocess.run([str(installed/'stratafetch-browser-host.exe'), '--register-local'], check=True, creationflags=0x08000000)
except Exception:
    for relative in reversed(changed):
        saved, target = backup/relative, installed/relative
        if saved.exists():
            shutil.copy2(saved, target)
        else:
            target.unlink(missing_ok=True)
    raise
for relative in files:
    if hashlib.sha256((installed/relative).read_bytes()).digest() != hashlib.sha256((package/relative).read_bytes()).digest():
        raise RuntimeError('Installed hash mismatch: '+str(relative))
print(json.dumps({'installed':str(installed), 'version':version, 'backup':str(backup), 'verifiedFiles':len(files)}, indent=2))
