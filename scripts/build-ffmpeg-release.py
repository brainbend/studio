#!/usr/bin/env python3
"""Builds the ffmpeg downloads for https://github.com/brainbend/studio-components (the app fetches them from there).
  build-ffmpeg-release.py DIR [--cache DIR2]
      writes one zip per platform and manifest.json into DIR; downloads already in DIR2 (by file name) are used
      instead of fetched.

Studio does not ship ffmpeg. The first time a video feature needs it, the app asks, downloads the zip for the computer
it runs on, and checks it against the SHA-256 in crates/video/ffmpeg-manifest.json. ffmpeg is a GPL build, run as a
separate program (never linked in). Each upstream download here is checked against a pinned SHA-256 before anything is
unpacked, so a changed file at the source stops the build.

To move to a newer ffmpeg:
  1. change VERSION, the URLs and the SHA-256 values below;
  2. run this script, then create the release in studio-components:
       gh release create ffmpeg-<version> --repo brainbend/studio-components DIR/*
  3. copy DIR/manifest.json into crates/video/ffmpeg-manifest.json, adding the "version" and "base" lines the file
     already has, and release the app. The app only trusts the checksums built into it."""
import hashlib, io, os, sys, tarfile, tempfile, urllib.request, zipfile

BTBN = "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-11-06-12-50/"
VERSION = "9.0.2"

# platform -> (url, sha256 of the download, where the program is inside it, source description)
BUILDS = {
    "windows-x86_64": (
        BTBN + "ffmpeg-n9.0.2-47-gb204e94e90-win64-gpl-9.0.zip",
        "8436c89156c3695659bf968e26bab5b08351ed8c86c3e9dbe3d6cc66a23af0a8",
        "ffmpeg-n9.0.2-47-gb204e94e90-win64-gpl-9.0/bin/ffmpeg.exe",
        "BtbN/FFmpeg-Builds, release autobuild-2026-11-06-12-50 (https://github.com/BtbN/FFmpeg-Builds)",
    ),
    "linux-x86_64": (
        BTBN + "ffmpeg-n9.0.2-47-gb204e94e90-linux64-gpl-9.0.tar.xz",
        "f44c6aa39c1b258d3ceafcee26d961445afdb810c8a1ffe85c4ba6a55b6ce60b",
        "ffmpeg-n9.0.2-47-gb204e94e90-linux64-gpl-9.0/bin/ffmpeg",
        "BtbN/FFmpeg-Builds, release autobuild-2026-11-06-12-50 (https://github.com/BtbN/FFmpeg-Builds)",
    ),
    # Intel only; on Apple silicon it runs under Rosetta.
    "macos-x86_64": (
        "https://evermeet.cx/ffmpeg/ffmpeg-" + VERSION + ".zip",
        "4acc0be580f9b2788029eb7bd4d645ff87968911b0a62aeeb3940d42d54558d5",
        "ffmpeg",
        "evermeet.cx static build of FFmpeg " + VERSION + " (https://evermeet.cx/ffmpeg/)",
    ),
}

GPL_URL = "https://www.gnu.org/licenses/gpl-3.0.txt"
FFMPEG_SOURCE = "https://ffmpeg.org/releases/ffmpeg-" + VERSION + ".tar.xz"


def download(url):
    print("  downloading", url)
    req = urllib.request.Request(url, headers={"User-Agent": "brainbend-studio-build"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def extract(blob, member):
    if member.endswith("ffmpeg.exe") or blob[:2] == b"PK":
        with zipfile.ZipFile(io.BytesIO(blob)) as z:
            return z.read(member)
    with tarfile.open(fileobj=io.BytesIO(blob), mode="r:xz") as t:
        return t.extractfile(member).read()


def write_notices(out, sha, source):
    """COPYING.GPLv3 and SOURCE.txt, which travel with every copy of the program."""
    try:
        with open(os.path.join(out, "COPYING.GPLv3"), "wb") as f:
            f.write(download(GPL_URL))
    except Exception as e:
        sys.exit("  could not fetch the GPL text: " + str(e))
    with open(os.path.join(out, "SOURCE.txt"), "w", newline="\n") as f:
        f.write(
            "This folder holds a build of FFmpeg " + VERSION + ", licensed under the GNU General Public License\n"
            "version 3 (see COPYING.GPLv3). BrainBend Studio runs it as a separate program; it is not part of\n"
            "BrainBend Studio's own code, which keeps its own license.\n\n"
            "Build: " + source + "\n"
            "SHA-256 of the download: " + sha + "\n"
            "Source code: " + FFMPEG_SOURCE + "\n"
            "The build's exact configuration is printed by `ffmpeg -buildconf`.\n\n"
            "Written offer: on request, BrainBend gives anyone who received this program the complete source code\n"
            "of the FFmpeg build and of the libraries linked into it, for at least three years from the day they\n"
            "received it. Open an issue at https://github.com/brainbend/studio-components to ask.\n"
            "H.264 and AAC are patented formats in some countries; the author of each lesson is responsible for\n"
            "using them where that is allowed.\n"
        )


def release(outdir, cache):
    """Zips for the studio-components release: the program, its license and where its source is, plus manifest.json."""
    import json
    os.makedirs(outdir, exist_ok=True)
    manifest = {"ffmpeg": {"version": VERSION, "files": {}}}
    for name, (url, sha, member, source) in BUILDS.items():
        print(name + ":")
        local = os.path.join(cache, os.path.basename(url)) if cache else None
        if local and os.path.exists(local):
            print("  using", local)
            blob = open(local, "rb").read()
        else:
            blob = download(url)
        got = hashlib.sha256(blob).hexdigest()
        if got != sha:
            sys.exit("  the download does not match the pinned SHA-256\n    expected " + sha + "\n    got      " + got)
        data = extract(blob, member)
        with tempfile.TemporaryDirectory() as tmp:
            write_notices(tmp, sha, source)
            program = "ffmpeg.exe" if name.startswith("windows") else "ffmpeg"
            archive = "ffmpeg-" + VERSION + "-" + name + ".zip"
            path = os.path.join(outdir, archive)
            with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
                for fname, body, mode in [(program, data, 0o755), ("COPYING.GPLv3", open(os.path.join(tmp, "COPYING.GPLv3"), "rb").read(), 0o644),
                                          ("SOURCE.txt", open(os.path.join(tmp, "SOURCE.txt"), "rb").read(), 0o644)]:
                    info = zipfile.ZipInfo(fname, (2026, 11, 7, 0, 0, 0))
                    info.compress_type = zipfile.ZIP_DEFLATED
                    info.external_attr = (0o100000 | mode) << 16
                    z.writestr(info, body)
        zsha = hashlib.sha256(open(path, "rb").read()).hexdigest()
        manifest["ffmpeg"]["files"][name] = {"file": archive, "sha256": zsha, "size": os.path.getsize(path), "program": program}
        print("  wrote", path, os.path.getsize(path) // 1000000, "MB")
    with open(os.path.join(outdir, "manifest.json"), "w", newline="\n") as f:
        json.dump(manifest, f, indent=2); f.write("\n")
    print("wrote manifest.json")


def main():
    argv = sys.argv[1:]
    if not argv or argv[0].startswith("--"):
        sys.exit(__doc__)
    cache = argv[argv.index("--cache") + 1] if "--cache" in argv else None
    release(argv[0], cache)


if __name__ == "__main__":
    main()
