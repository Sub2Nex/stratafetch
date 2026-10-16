#!/usr/bin/env python3
"""Export every product icon from the committed master (requires Pillow)."""
import base64
from pathlib import Path
from PIL import Image

root = Path(__file__).resolve().parent.parent
master = Image.open(root / 'assets/branding/stratafetch-master.png').convert('RGBA')
master = master.resize((1024, 1024), Image.Resampling.LANCZOS)
for size in [16, 24, 32, 48, 64, 128, 256, 512, 1024]:
    master.resize((size, size), Image.Resampling.LANCZOS).save(root / f'assets/branding/icon-{size}.png')
master.resize((128, 128), Image.Resampling.LANCZOS).save(root / 'assets/stratafetch.png')
master.save(root / 'assets/stratafetch.ico', sizes=[(s, s) for s in [16, 24, 32, 48, 64, 128, 256]])
# Linux consumers of the established SVG path see the identical raster artwork.
data = base64.b64encode((root / 'assets/branding/icon-512.png').read_bytes()).decode('ascii')
(root / 'assets/stratafetch.svg').write_text(f'<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512"><image width="512" height="512" href="data:image/png;base64,{data}"/></svg>\n', encoding='utf-8')
for browser in ['chromium', 'firefox']:
    for size in [16, 32, 48, 128]:
        image = master.resize((size, size), Image.Resampling.LANCZOS)
        if size == 128:
            # Chrome's listing icon specifies 96px artwork with transparent padding.
            image = Image.new('RGBA', (128, 128))
            image.alpha_composite(master.resize((96, 96), Image.Resampling.LANCZOS), (16, 16))
        image.save(root / f'extensions/{browser}/icons/{size}.png')
for name, width, height in [('chrome-small-promo', 440, 280), ('chrome-marquee', 1400, 560), ('github-social-preview', 1280, 640)]:
    image = Image.new('RGB', (width, height), '#081a36')
    size = int(height * .82)
    icon = master.resize((size, size), Image.Resampling.LANCZOS)
    image.paste(icon, ((width-size)//2, (height-size)//2), icon)
    image.save(root / f'assets/branding/{name}.png')
print('Exported master icon for native apps, installers, extensions and GitHub.')
