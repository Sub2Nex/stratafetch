#!/usr/bin/env python3
"""Deploy an already built matching package; never touches download state.

Explicit --install and a matching full --installer are required. Stop
StrataFetch first. Never update only EXEs: Windows shell metadata must match.
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
parser.add_argument('--installer', type=Path, help='Matching full Windows setup executable')
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
files = [Path(name) for name in bins + ['BUILD.txt', 'stratafetch.ico']]
files += [path.relative_to(package) for path in (package/'extensions').rglob('*') if path.is_file()]
for name in bins:
    target = installed/name
    if target.exists():
        with target.open('r+b'):
            pass  # Windows rejects write access to a running executable.
if not args.install:
    print('Validated matching package. Use --install --installer SETUP.exe to deploy into '+str(installed))
    sys.exit(0)
if not args.installer or not args.installer.is_file():
    parser.error('--install requires the matching full --installer; partial EXE deployment leaves Windows shortcuts and Installed apps stale')
installer = args.installer.resolve()
checksum_lines = (installer.parent/'SHA256SUMS').read_text(encoding='ascii').splitlines()
expected = [line.split()[0] for line in checksum_lines if len(line.split()) == 2 and line.split()[1] == installer.name]
if expected != [hashlib.sha256(installer.read_bytes()).hexdigest()]:
    raise ValueError('Missing, duplicate or mismatched full setup checksum')
backup = repo/'.smoke/runtime-backup'/time.strftime('%Y%m%d-%H%M%S')
for relative in files:
    target = installed/relative
    if target.exists():
        saved = backup/relative
        saved.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(target, saved)
# NSIS /D= must be last and unquoted, including spaces. No command shell is used.
if any(char in str(installed) for char in '\"\r\n'):
    raise ValueError('Invalid installation path')
command = subprocess.list2cmdline([str(installer), '/S'])+' /D='+str(installed)
subprocess.run(command, executable=str(installer), check=True, creationflags=0x08000000)
for relative in files:
    if hashlib.sha256((installed/relative).read_bytes()).digest() != hashlib.sha256((package/relative).read_bytes()).digest():
        raise RuntimeError('Installed hash mismatch: '+str(relative))
import winreg
with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r'Software\Microsoft\Windows\CurrentVersion\Uninstall\StrataFetch', 0, winreg.KEY_READ | winreg.KEY_WOW64_64KEY) as key:
    if winreg.QueryValueEx(key, 'DisplayVersion')[0] != version or winreg.QueryValueEx(key, 'DisplayIcon')[0] != f'"{installed / "stratafetch.ico"}",0':
        raise RuntimeError('Windows shell registration differs from the installed package')
print(json.dumps({'installed':str(installed), 'version':version, 'backup':str(backup), 'verifiedFiles':len(files)}, indent=2))
