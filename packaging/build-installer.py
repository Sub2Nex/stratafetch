#!/usr/bin/env python3
"""Build the same per-user NSIS installer on Windows or a local Linux host."""
import argparse
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib


def main():
    root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', default=shutil.which('makensis'))
    parser.add_argument('--package', type=Path, default=root/'dist/stratafetch-windows-x64')
    parser.add_argument('--output', type=Path, default=root/'dist/stratafetch-windows-x64-setup.exe')
    parser.add_argument('--test-id', help='Isolated registration/shortcut destinations for native tests only')
    args = parser.parse_args()
    if not args.compiler:
        parser.error('Install NSIS or pass --compiler with the makensis path')
    package = args.package.resolve()
    output = args.output.resolve()
    version = tomllib.loads((root/'Cargo.toml').read_text(encoding='utf-8'))['workspace']['package']['version']
    core = ['stratafetch.exe', 'stratafetch-desktop.exe', 'stratafetch-browser-host.exe']
    subprocess.run([sys.executable, str(root/'scripts/verify-windows-manifest.py'),
                    *[str(package/name) for name in core]], check=True)
    if f'StrataFetch {version}' not in (package/'BUILD.txt').read_text(encoding='utf-8'):
        raise ValueError('Payload version differs from Cargo.toml')
    version_parts = [int(part) for part in version.split('-', 1)[0].split('.')]
    revision = re.search(r'(\d+)$', version.split('-', 1)[1]) if '-' in version else None
    version_parts.append(int(revision[1]) if revision else 0)
    if len(version_parts) != 4 or any(not 0 <= part <= 65535 for part in version_parts):
        raise ValueError('Version cannot be represented in Windows VERSIONINFO')
    instructions = []
    paths = sorted(package.rglob('*'))
    for path in paths:
        relative = str(path.relative_to(package)).replace('/', '\\')
        if path.is_symlink() or any(char in relative for char in '"$\r\n'):
            raise ValueError(f'Unsafe installer payload path: {relative}')
        if path.is_file() and relative not in core:
            instructions.append(f'Delete "$INSTDIR\\{relative}"')
    for path in sorted((p for p in paths if p.is_dir()), key=lambda p: len(p.parts), reverse=True):
        relative = str(path.relative_to(package)).replace('/', '\\')
        instructions.append(f'RMDir "$INSTDIR\\{relative}"')
    flag = '/' if os.name == 'nt' else '-'
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='strata-installer-') as temporary:
        include = Path(temporary)/'installer-files.nsh'
        include.write_text('\n'.join(instructions)+'\n', encoding='utf-8')
        defines = {
            'PACKAGE_DIR': str(package), 'OUTPUT_FILE': str(output),
            'APP_VERSION': version, 'PRODUCT_VERSION': '.'.join(map(str, version_parts)),
            'INSTALLER_FILES': str(include),
        }
        if args.test_id:
            if not re.fullmatch(r'[a-zA-Z0-9-]+', args.test_id):
                raise ValueError('Invalid isolated installer test identifier')
            product = 'StrataFetch-InstallerTest-'+args.test_id
            defines['UNINSTALL_KEY'] = 'Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\'+product
            defines['START_MENU_DIR'] = product
        command = [args.compiler, flag+'V3',
                   *[flag+'D'+key+'='+value for key, value in defines.items()],
                   str(root/'packaging/installer.nsi')]
        subprocess.run(command, cwd=root/'packaging', check=True)
    if not args.test_id:
        checksum_path = output.parent/'SHA256SUMS'
        lines = checksum_path.read_text(encoding='ascii').splitlines() if checksum_path.exists() else []
        lines = [line for line in lines if line.split(maxsplit=1)[-1].lstrip('*') != output.name]
        digest = hashlib.sha256(output.read_bytes()).hexdigest()
        lines.append(f'{digest}  {output.name}')
        checksum_path.write_text('\n'.join(lines)+'\n', encoding='ascii')
        print(f'{digest}  {output.name}')
    print(output)


if __name__ == '__main__':
    main()
