#!/usr/bin/env python3
"""Create local source and store-upload archives. Firefox archives are unsigned."""
import argparse
import hashlib
import json
from pathlib import Path
import base64
import re
import shutil
import zipfile

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('--output', type=Path, default=root/'dist')
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
ids = json.loads((root/'extensions/local-ids.json').read_text(encoding='utf-8-sig'))
stores = json.loads((root/'extensions/store-ids.json').read_text(encoding='utf-8-sig'))
for browser in ['chrome', 'edge']:
    identity = stores.get(browser)
    if identity is not None:
        if not re.fullmatch('[a-p]{32}', identity['id']) or identity['id'] == ids['chromium']:
            raise ValueError('Invalid store identity; do not replace the development ID')
        if identity.get('public_key'):
            digest = hashlib.sha256(base64.b64decode(identity['public_key'], validate=True)).digest()[:16]
            derived = ''.join(chr(97+n) for byte in digest for n in (byte >> 4, byte & 15))
            if derived != identity['id']:
                raise ValueError('Store public key/ID mismatch')
if stores['firefox']['id'] != ids['firefox']:
    raise ValueError('Firefox store/development identity mismatch')
for folder, name, store in [
    ('chromium', 'stratafetch-chromium-local.zip', False),
    ('firefox', 'stratafetch-firefox-local-source.zip', False),
    ('chromium', 'stratafetch-chrome-webstore.zip', 'chrome'),
    ('chromium', 'stratafetch-edge-addons.zip', 'edge'),
    ('firefox', 'stratafetch-firefox-amo.zip', 'firefox'),
    ('firefox', 'stratafetch-firefox-unsigned-dev.xpi', 'firefox'),
]:
    directory = root/'extensions'/folder
    manifest = json.loads((directory/'manifest.json').read_text(encoding='utf-8-sig'))
    if folder == 'firefox' and manifest['browser_specific_settings']['gecko']['id'] != ids['firefox']:
        raise ValueError('Firefox native origin mismatch')
    if folder == 'chromium' and store:
        # The Web Store generates its own key/ID on the first draft upload.
        # Keep our development key only in the unpacked/local build.
        manifest.pop('key', None)
        if stores.get(store) and stores[store].get('public_key'):
            manifest['key'] = stores[store]['public_key']
    archive = args.output/name
    with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED) as output:
        for file in ['manifest.json', 'locale.js', '_locales/en/messages.json', '_locales/sv/messages.json', 'background.js', 'popup.html', 'popup.js', *sorted(set(manifest['icons'].values()))]:
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

for folder in ['chromium', 'firefox']:
    destination = args.output/'unpacked'/folder
    destination.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.output/('stratafetch-chromium-local.zip' if folder == 'chromium' else 'stratafetch-firefox-local-source.zip')) as archive:
        for entry in archive.infolist():
            # All names come from the explicit source allowlist above.
            path = destination/entry.filename
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(archive.read(entry))
shutil.copyfile(root/'docs/EXTENSION-PUBLISHING.md', args.output/'SUBMISSION-GUIDE.md')
shutil.copyfile(root/'PRIVACY.md', args.output/'PRIVACY.md')
shutil.copyfile(root/'docs/RELEASE-NOTES.md', args.output/'RELEASE-NOTES.md')
files = sorted(path for path in args.output.rglob('*') if path.is_file() and path.name != 'SHA256SUMS')
(args.output/'SHA256SUMS').write_text(''.join(hashlib.sha256(path.read_bytes()).hexdigest()+'  '+path.relative_to(args.output).as_posix()+'\n' for path in files), encoding='ascii')
