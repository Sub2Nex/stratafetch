#!/usr/bin/env python3
"""Create local source and store-upload archives. Firefox archives are unsigned."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('--output', type=Path, default=root/'dist')
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
ids = json.loads((root/'extensions/local-ids.json').read_text(encoding='utf-8-sig'))
for folder, name, store in [
    ('chromium', 'stratafetch-chromium-local.zip', False),
    ('firefox', 'stratafetch-firefox-local-source.zip', False),
    ('chromium', 'stratafetch-chrome-webstore.zip', True),
    ('firefox', 'stratafetch-firefox-amo.zip', True),
]:
    directory = root/'extensions'/folder
    manifest = json.loads((directory/'manifest.json').read_text(encoding='utf-8-sig'))
    if folder == 'firefox' and manifest['browser_specific_settings']['gecko']['id'] != ids['firefox']:
        raise ValueError('Firefox native origin mismatch')
    if folder == 'chromium' and store:
        # The Web Store generates its own key/ID on the first draft upload.
        # Keep our development key only in the unpacked/local build.
        manifest.pop('key', None)
    archive = args.output/name
    with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED) as output:
        for file in ['manifest.json', 'background.js', 'popup.html', 'popup.js', *sorted(set(manifest['icons'].values()))]:
            # Normalize source text so Windows editor BOMs cannot break manifests.
            data = (json.dumps(manifest, indent=2)+'\n').encode() if file == 'manifest.json' else (directory/file).read_bytes()
            if not file.endswith('.png'):
                data = data.decode('utf-8-sig').replace('\r\n', '\n').encode()
            entry = zipfile.ZipInfo(file, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            output.writestr(entry, data)
    with zipfile.ZipFile(archive) as check:
        if check.testzip() is not None or 'manifest.json' not in check.namelist():
            raise ValueError('Invalid extension archive')
    print(hashlib.sha256(archive.read_bytes()).hexdigest()+'  '+name)
