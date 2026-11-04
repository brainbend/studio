#!/usr/bin/env python3
"""Downloads the open-licensed fonts bundled with BrainBend Studio from Google Fonts into assets/fonts/<Family>/.

  fetch-fonts.py            fetch every family in FAMILIES that is not there yet
  fetch-fonts.py --force    fetch them all again

Each family folder holds the font files (variable fonts where Google offers them, else Regular, Bold, Italic and
Bold Italic) and OFL.txt, its license. Then run scripts/font-licenses.py to refresh FONT-LICENSES.md.
Every family listed here is under the SIL Open Font License 1.1 (or the Ubuntu Font Licence for none of them).
"""
import json, os, sys, urllib.request, urllib.parse

FAMILIES = [
    "Montserrat", "Open Sans",
    "Inter", "Roboto", "Lato", "Poppins", "Nunito", "Work Sans", "Raleway", "Source Sans 3", "DM Sans",
    "Libre Baskerville", "Playfair Display", "Lora", "Source Serif 4",
    "Oswald", "Bebas Neue", "JetBrains Mono", "Source Code Pro", "Caveat",
]
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "fonts")
STATIC_WANTED = ("-Regular.ttf", "-Bold.ttf", "-Italic.ttf", "-BoldItalic.ttf")

def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    return urllib.request.urlopen(req, timeout=60).read()

def main():
    force = "--force" in sys.argv
    for family in FAMILIES:
        folder = os.path.join(ROOT, family)
        if os.path.isdir(folder) and not force:
            continue
        raw = get("https://fonts.google.com/download/list?family=" + urllib.parse.quote(family)).decode("utf-8")
        listing = json.loads(raw.split("\n", 1)[1])
        files = listing["manifest"]["fileRefs"]
        variable = [f for f in files if "VariableFont" in f["filename"]]
        if variable:
            chosen = variable
        else:
            chosen = [f for f in files if "/" not in f["filename"] and f["filename"].endswith(STATIC_WANTED)] or [f for f in files if "/" not in f["filename"]][:1]
        os.makedirs(folder, exist_ok=True)
        for f in chosen:
            data = get(f["url"])
            with open(os.path.join(folder, f["filename"]), "wb") as out:
                out.write(data)
        for f in listing["manifest"]["files"]:
            if f["filename"].upper().startswith("OFL"):
                with open(os.path.join(folder, "OFL.txt"), "w", newline="\n") as out:
                    out.write(f["contents"].replace("\r\n", "\n"))
        print(family, [f["filename"] for f in chosen])

main()
