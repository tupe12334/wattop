#!/usr/bin/env python3
"""Draw the .dmg window background and write app/dmg/background.tiff.

The TIFF holds two images: 660x400 (1x) and 1320x800 (@2x). Finder picks the
right one for the screen. Needs Pillow and macOS (tiffutil, system fonts).

Run from the repo root:
    python3 app/dmg/make_background.py
"""

import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
W, H = 660, 400
SS = 4  # supersampling factor, downscaled for smooth edges
TOP, BOTTOM = (0x0F, 0x17, 0x2A), (0x1E, 0x29, 0x3B)
WHITE = (255, 255, 255)
MUTED = (0x94, 0xA3, 0xB8)
DIM = (0x64, 0x74, 0x8B)
GREEN = (0x4A, 0xDE, 0x80)  # same as the app's charging color


def load_font(size, bold):
    # ponytail: two system fonts is enough. Pillow's default font is the last resort.
    try:
        font = ImageFont.truetype("/System/Library/Fonts/SFNS.ttf", size)
        font.set_variation_by_name("Bold" if bold else "Regular")
        return font
    except (OSError, ValueError):
        pass
    try:
        return ImageFont.truetype(
            "/System/Library/Fonts/Helvetica.ttc", size, index=1 if bold else 0
        )
    except OSError:
        return ImageFont.load_default(size)


def render(scale):
    k = scale * SS
    img = Image.new("RGB", (W * k, H * k))
    draw = ImageDraw.Draw(img)
    for y in range(H * k):
        t = y / (H * k - 1)
        draw.line(
            [(0, y), (W * k, y)],
            fill=tuple(round(a + (b - a) * t) for a, b in zip(TOP, BOTTOM)),
        )

    cx = W * k / 2
    draw.text((cx, 56 * k), "Wattop", font=load_font(34 * k, True), fill=WHITE, anchor="mt")
    draw.text(
        (cx, 102 * k),
        "Drag Wattop to Applications",
        font=load_font(15 * k, False),
        fill=MUTED,
        anchor="mt",
    )

    # Arrow sits between the two icon slots (centers x=170 and x=490, icon 128 px).
    y, x0, x1 = 200 * k, 268 * k, 392 * k
    shaft, head_len, head_h = 5 * k, 18 * k, 18 * k
    draw.rectangle([x0, y - shaft / 2, x1 - head_len, y + shaft / 2], fill=GREEN)
    draw.polygon(
        [(x1 - head_len, y - head_h), (x1, y), (x1 - head_len, y + head_h)], fill=GREEN
    )

    draw.text(
        (cx, 362 * k),
        "Unsigned build · right-click → Open on first launch",
        font=load_font(12 * k, False),
        fill=DIM,
        anchor="mt",
    )
    return img.resize((W * scale, H * scale), Image.LANCZOS)


def main():
    with tempfile.TemporaryDirectory() as tmp:
        one, two = Path(tmp, "background.png"), Path(tmp, "background@2x.png")
        render(1).save(one)
        render(2).save(two)
        subprocess.run(
            [
                "tiffutil",
                "-cathidpicheck",
                str(one),
                str(two),
                "-out",
                str(HERE / "background.tiff"),
            ],
            check=True,
        )


if __name__ == "__main__":
    main()
