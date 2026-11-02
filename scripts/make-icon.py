#!/usr/bin/env python3
"""Draws the BrainBend Studio app icon (blue rounded square, white "S").

  make-icon.py ICONSET_DIR            PNGs for a macOS .iconset
  make-icon.py --extras DIR           brainbend-studio.png (512 px) and brainbend-studio.ico (Windows) in DIR
Both can be given together.
"""
import sys, os
from PIL import Image, ImageDraw, ImageFont

args = sys.argv[1:]
extras = None
if "--extras" in args:
    i = args.index("--extras")
    extras = args[i + 1]
    del args[i:i + 2]
out = args[0] if args else None
FONT = os.path.join(os.path.dirname(__file__), "..", "assets", "fonts", "Montserrat[wght].ttf")

def draw(size):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pad = size * 0.06
    d.rounded_rectangle([pad, pad, size - pad, size - pad], radius=size * 0.22, fill=(3, 83, 158, 255))
    f = ImageFont.truetype(FONT, int(size * 0.62))
    try:
        f.set_variation_by_axes([800])
    except Exception:
        pass
    bbox = d.textbbox((0, 0), "S", font=f)
    w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
    d.text(((size - w) / 2 - bbox[0], (size - h) / 2 - bbox[1]), "S", font=f, fill=(255, 255, 255, 255))
    return img

if out:
    os.makedirs(out, exist_ok=True)
    for base in (16, 32, 128, 256, 512):
        draw(base).save(os.path.join(out, f"icon_{base}x{base}.png"))
        draw(base * 2).save(os.path.join(out, f"icon_{base}x{base}@2x.png"))
if extras:
    os.makedirs(extras, exist_ok=True)
    draw(512).save(os.path.join(extras, "brainbend-studio.png"))
    draw(256).save(os.path.join(extras, "brainbend-studio.ico"), sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
