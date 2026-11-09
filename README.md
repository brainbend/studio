<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brainbend-icon-dark-256.png">
    <img alt="BrainBend Studio icon: a blue and purple brain struck by yellow lightning" src="assets/brainbend-icon-256.png" width="128" height="128">
  </picture>
</p>

<h1 align="center">BrainBend Studio</h1>

<p align="center">
  <b>A free, open-source authoring tool for interactive e-learning.</b><br>
  Build slide-based Media Lessons, open PowerPoint and Storyline files, and publish to the web, an LMS, PDF, Word or PowerPoint.
</p>

<p align="center">
  <img alt="Platforms: macOS, Windows and Linux" src="https://img.shields.io/badge/macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-native-0a6eeb?style=flat-square">
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-b7410e?style=flat-square&logo=rust">
  <img alt="License: MIT or Apache-2.0" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-3a3a3a?style=flat-square">
  <img alt="Status: work in progress" src="https://img.shields.io/badge/status-work%20in%20progress-d69e2e?style=flat-square">
</p>

<p align="center">
  <a href="https://brainbend.org"><b>BrainBend.org</b></a> ·
  <a href="https://brainbend.org/discord/"><b>BrainBend Community on Discord</b></a>
</p>

<p align="center">
  <img src="docs/images/screenshot.png" alt="BrainBend Studio: the slide editor with the ribbon, slide list, canvas, Properties panel, slide layers and timeline" width="100%">
</p>

> [!NOTE]
> BrainBend Studio is a work in progress. New features are being added in every update.

## Runs where you do

A native desktop app, not a browser tab. It works offline and keeps your files on your computer.

| Platform | Downloads |
|---|---|
| **macOS** | One universal `.dmg` that runs natively on Apple silicon and Intel Macs |
| **Windows** | An installer (`.msi`) or a portable `.exe` in a `.zip`, for x64 and arm64 |
| **Linux** | `.deb`, `.rpm`, an AppImage, or a `.tar.gz` with the binary, for x86_64 and aarch64 |

Releases are on the [Releases page](https://github.com/brainbend/studio/releases) and the app can check for updates itself. Every download is signed, and each release lists SHA-256 checksums in a signed `SHA256SUMS.txt`.

## Build with AI

Connect your own AI tool and let it build lessons while you watch. Run Claude Code, Codex or Gemini CLI in a Terminal tab inside Studio, or chat with Claude, OpenAI or Gemini using your own API key. The AI can set up your brand (theme colors, fonts and layouts), build slides on your layouts, find royalty-free pictures, add icons and SVG diagrams, build buttons, check accessibility and set the player's options. A design you already made is not changed until you say yes, and every change can be undone.

## What you can add

**Theme** (Theme tab)

| Group | Options |
|---|---|
| Master | Layout Builder (build your own slide layouts) · Layout (change a slide's layout) · Slide Size (16:9 or 4:3) |
| Brand | Background (color, gradient or picture) · Colors (your project's colors) · Headings and Body fonts |

**Slides** (Slides tab)

| Kind | Slides |
|---|---|
| Content | A blank slide, or one from a slide layout · New Scene |
| Graded questions | Multiple choice · Multiple response · True / False · Fill in the blank · Matching · Sequencing · Fill in the blanks · Drag the words · Mark the words · Number · Sort into groups · Find the hotspot · Label the picture · Choose from lists · Dictation |
| Survey questions (not scored) | Rating scale · Free text |
| Results | A quiz results slide that reports the score |
| Course | Variables · Pass mark |

**Objects** (Objects tab)

| Group | Objects |
|---|---|
| Text | Text Box · Table · Slide Number |
| Media | Picture (including SVG) · Shape · Video · Audio · Caption (speech and thought bubbles) · 360° Image · Equation · Icons (a vector gallery you can recolor) · Web Object |
| Interactive | Button (with Normal, Hover, Down, Visited, Disabled and Selected states) · Slider · Hotspot (rectangle, oval or freeform) · Dial (knob or needle) · Input (check box, radio button, text entry, number entry) · Marker |

Parts can be grouped so a card made of a shape, an icon and some words works as one button for the mouse, the keyboard and screen readers.

## Publish

Press **Publish** to choose how the project goes out:

- **Web**: an HTML5 folder named for the project, ready for any web server.
- **Other LMS**: a SCORM package (`.zip`) for SCORM 1.2 or SCORM 2004 4th Edition, reporting completion, score and pass/fail.
- **PDF**: a PDF file saved directly (no print dialog): a handout in portrait or landscape, with 1, 2 or 3 slides per page, optional correct answers, space for notes, slide notes and alt text.
- **Print**: the same handout in your browser's print dialog, for a printer or its own "Save as PDF".
- **Word**: an editable .docx handout with a picture of each slide (questions as they look in the lesson), on a portrait page: one slide with its notes and alt text, or two slides to a page.
- **PowerPoint**: a `.pptx` file.

## License

BrainBend Studio is dual-licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. The bundled fonts are under the SIL Open Font License 1.1 (texts in [FONT-LICENSES.md](FONT-LICENSES.md)), and the license of each font a lesson uses is copied into its export. Other open-source software the app uses is listed in [THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md). The BrainBend name and logo are not covered by these licenses.
