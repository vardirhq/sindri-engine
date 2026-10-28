"""Build a right-facing, foot-aligned sheet from the PixelLab Wick export.

The export's west-facing walk has 68px frames; its idle is 48px. Use the
same 48px canvas and ground row for every frame. Hero mirrors it for left.
"""
from io import BytesIO
from pathlib import Path
from zipfile import ZipFile
import json

from PIL import Image, ImageOps

HERE = Path(__file__).resolve().parent
TEXTURES = HERE.parents[1] / 'assets' / 'textures'


def main():
    with ZipFile(HERE / 'wick-source.zip') as source:
        names = ['Idle/rotations/west.png'] + [
            f'Idle/animations/Walking/west/frame_{i:03}.png' for i in range(8)
        ]
        frames = []
        for name in names:
            image = Image.open(BytesIO(source.read(name))).convert('RGBA')
            image = ImageOps.mirror(image)
            bbox = image.getbbox()
            if bbox is None:
                raise ValueError(f'{name} is empty')
            # Preserve pixel scale and horizontal center, moving the feet to
            # the bottom edge. Reject anything that would be cropped.
            x = (48 - image.width) // 2
            y = 48 - bbox[3]
            if bbox[0] + x < 0 or bbox[2] + x > 48 or bbox[1] + y < 0:
                raise ValueError(f'{name} does not fit its frame')
            frame = Image.new('RGBA', (48, 48))
            frame.alpha_composite(image, (x, y))
            frames.append(frame)
    sheet = Image.new('RGBA', (48 * len(frames), 48))
    for i, frame in enumerate(frames):
        sheet.alpha_composite(frame, (i * 48, 0))
    sheet.save(TEXTURES / 'wick-pixellab.png')
    metadata = {
        'format_version': 1,
        'anchor': 'bottom',
        'grid': {'columns': 9, 'rows': 1,
                 'names': ['idle'] + [f'walk-{i}' for i in range(8)]},
    }
    (TEXTURES / 'wick-pixellab.sheet').write_text(json.dumps(metadata, indent=2) + '\n')


if __name__ == '__main__':
    main()
