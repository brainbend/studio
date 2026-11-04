#!/usr/bin/env python3
"""Writes FONT-LICENSES.md: the license text of every font bundled with BrainBend Studio (assets/fonts/<Family>/OFL.txt).
Run it after scripts/fetch-fonts.py. The file ships in every download."""
import os

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
fonts = os.path.join(root, "assets", "fonts")
out = ["# Font licenses\n",
       "These fonts are bundled with BrainBend Studio. Each is licensed under the SIL Open Font License 1.1, "
       "which allows them to be used, embedded in the lessons you publish, and redistributed. "
       "The text for each follows.\n"]
for family in sorted(os.listdir(fonts), key=str.lower):
    lic = os.path.join(fonts, family, "OFL.txt")
    if os.path.isfile(lic):
        out.append(f"\n---\n\n## {family}\n\n```\n{open(lic, encoding='utf-8').read().strip()}\n```\n")
with open(os.path.join(root, "FONT-LICENSES.md"), "w", newline="\n") as f:
    f.write("".join(out))
print("Wrote FONT-LICENSES.md")
