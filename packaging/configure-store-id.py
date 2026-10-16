#!/usr/bin/env python3
"""Record a store-assigned identity without changing the local development ID."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import tempfile
from urllib.parse import urlparse

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('browser', choices=['chrome', 'edge'])
parser.add_argument('--id', required=True, help='Actual 32-letter extension ID assigned by the store')
parser.add_argument('--public-key-file', type=Path, help='Optional store public key, base64 or PUBLIC KEY PEM (never a private key)')
parser.add_argument('--listing-url', help='Optional approved HTTPS store listing URL')
args = parser.parse_args()
if not re.fullmatch('[a-p]{32}', args.id):
    parser.error('The assigned extension ID must contain exactly 32 letters a-p')
development = json.loads((root/'extensions/local-ids.json').read_text(encoding='utf-8-sig'))
if args.id == development['chromium']:
    parser.error('Do not claim the local development ID as a store-assigned identity')
identity = {'id': args.id, 'public_key': None, 'listing_url': None}
if args.public_key_file:
    text = args.public_key_file.read_text(encoding='utf-8-sig')
    if 'PRIVATE' in text:
        parser.error('Private keys must never be provided or packaged')
    key = ''.join(text.replace('-----BEGIN PUBLIC KEY-----', '').replace('-----END PUBLIC KEY-----', '').split())
    try:
        digest = hashlib.sha256(base64.b64decode(key, validate=True)).digest()[:16]
    except ValueError:
        parser.error('Invalid public key')
    derived = ''.join(chr(97+n) for byte in digest for n in (byte >> 4, byte & 15))
    if derived != args.id:
        parser.error('The public key does not derive the supplied extension ID')
    identity['public_key'] = key
if args.listing_url:
    url = urlparse(args.listing_url)
    domain = 'chromewebstore.google.com' if args.browser == 'chrome' else 'microsoftedge.microsoft.com'
    if url.scheme != 'https' or url.netloc != domain:
        parser.error('Use the exact HTTPS listing URL on the official store domain')
    identity['listing_url'] = args.listing_url
path = root/'extensions/store-ids.json'
stores = json.loads(path.read_text(encoding='utf-8-sig'))
stores[args.browser] = identity
with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', dir=path.parent, delete=False) as output:
    json.dump(stores, output, indent=2)
    output.write('\n')
    temporary = Path(output.name)
temporary.replace(path)
print(f'Recorded {args.browser} store ID {args.id}; local development files were preserved.')
print('Rebuild matching desktop packages and extensions, register the bridge, then test the store identity before distribution.')
