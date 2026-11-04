#!/usr/bin/env python3
"""Draws the BrainBend Studio app icon (the BrainBend logo on a rounded navy square).

  make-icon.py ICONSET_DIR            PNGs for a macOS .iconset
  make-icon.py --extras DIR           brainbend-studio.png (512 px) and brainbend-studio.ico (Windows) in DIR
Both can be given together.
"""
import sys, os
from PIL import Image, ImageDraw

args = sys.argv[1:]
extras = None
if "--extras" in args:
    i = args.index("--extras")
    extras = args[i + 1]
    del args[i:i + 2]
out = args[0] if args else None

SOURCE = os.path.join(os.path.dirname(__file__), "..", "assets", "brainbend-icon.png")

def draw(size):
    """The logo on its navy square, clipped to a rounded square with a little margin like other macOS icons."""
    inner = round(size * 0.80)  # the brain keeps clear of the edge
    art = Image.new("RGB", (size, size), (13, 31, 67))
    art.paste(Image.open(SOURCE).convert("RGB").resize((inner, inner), Image.LANCZOS), ((size - inner) // 2, (size - inner) // 2))
    pad = size * 0.10
    mask = Image.new("L", (size * 4, size * 4), 0)
    ImageDraw.Draw(mask).rounded_rectangle([pad * 4, pad * 4, (size - pad) * 4, (size - pad) * 4], radius=size * 0.225 * 4, fill=255)
    mask = mask.resize((size, size), Image.LANCZOS)
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    img.paste(art, (0, 0), mask)
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
