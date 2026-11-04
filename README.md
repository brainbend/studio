<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brainbend-icon-dark-256.png">
    <img alt="BrainBend Studio icon: a blue and purple brain struck by yellow lightning" src="assets/brainbend-icon-256.png" width="128" height="128">
  </picture>
</p>

<h1 align="center">BrainBend Studio</h1>

<p align="center">
  <b>A free, open-source authoring tool for interactive e-learning.</b><br>
  Build slide-based Media Lessons, open PowerPoint and Storyline files, and publish to the web, an LMS or PDF.
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
> BrainBend Studio is a work in progress, heading for its first release (v0.1.0). Web Lessons are not available to create yet.

## Runs where you do

A native desktop app, not a browser tab. It works offline and keeps your files on your computer.

| Platform | Download |
|---|---|
| **macOS** | One universal `.dmg` that runs natively on Apple silicon and Intel Macs |
| **Windows** | A portable `.exe` in a `.zip` |
| **Linux** | A `.tar.gz` with the binary |

Every change is built and tested on macOS, Windows and Linux. Releases are on the [Releases page](https://github.com/brainbend/studio/releases) and the app can check for updates itself.

## What you can add

**Objects** (Insert tab)

| Group | Objects |
|---|---|
| Media | Picture · Shape · Video · Audio · Caption (speech and thought bubbles) · 360° Image · Equation · Icons (a vector gallery you can recolour) · Web Object |
| Text | Text Box · Table · Slide Number |
| Interactive | Button (with Normal, Hover, Down, Visited, Disabled and Selected states) · Slider · Hotspot (rectangle, oval or freeform) · Dial · Input (check box, radio button, text entry, number entry) · Marker |

**Slides** (Slides tab)

| Kind | Slides |
|---|---|
| Content | A blank slide, or one from a slide layout |
| Graded questions | Multiple choice · Multiple response · True / False · Fill in the blank · Matching · Sequencing · Fill in the blanks · Drag the words · Mark the words · Number · Sort into groups · Find the hotspot · Label the picture · Choose from lists · Dictation |
| Survey questions (not scored) | Rating scale · Free text |
| Results | A quiz results slide that reports the score |

## Publish

Press **Publish** to choose how the project goes out:

- **Web**: an HTML5 folder named for the project, ready for any web server.
- **Other LMS**: a SCORM package (`.zip`) for SCORM 1.2 or SCORM 2004 4th Edition, reporting completion, score and pass/fail.
- **PDF**: a handout in portrait or landscape, with 1, 2 or 3 slides per page, optional correct answers, space for notes, slide notes and alt text.
- **PowerPoint**: a `.pptx` file.

BrainBend Review, BrainBend LMS, Video and Word are marked Coming soon.

## Run

```sh
cargo run --release -p brainbend-studio                      # start empty (File ▸ New / Open)
cargo run --release -p brainbend-studio -- path/to/file.story
```

Requires a recent stable Rust toolchain. Hand-tested on macOS so far; CI builds and tests on macOS, Windows and Linux.

## What it does

**Web Lessons** (File ▸ New ▸ Web Lesson): scrolling, mobile-friendly lessons built from blocks (headings, text, images, video, audio, buttons, quotes, statements, accordion, tabs, process, timeline, flashcards, Continue gates and knowledge checks that use every question type), with a section menu, progress, completion rules and SCORM reporting. The editor shows an outline, a preview and block properties; Preview opens the real page.

**Editor** (egui): slides, layers, text/rectangle/button/image/video/audio objects, resize
handles, snapping guides, zoom and pan, copy/paste/duplicate, undo/redo, a variables panel and a
trigger editor (events, actions, conditions), question slides (multiple choice, multiple response, true/false, fill in the blank, matching,
sequencing) and a results slide, slide backgrounds,
course settings (pass mark, navigation bar), slide thumbnails, entrance animations (fade, slide,
zoom, wipe), exit effects, object timing on layer/slide timelines (appear/leave at set times, a
"timeline ends" event, auto-advance), slide fade-in, autosave/recovery, dark and light themes, a visual timeline, and a project checker (Project ▸
Check project…) for missing alt text, uncaptioned video, unnamed buttons, low text contrast, broken
trigger targets and malformed questions. Find and Replace (⌘F) and a spell checker (Review ▸ Spelling, F7; US English, with a per-project custom word list) cover all slide text, notes and question text.

**Import** (`.story`): scenes/slides in course order, layers, text, rectangles, rounded rectangles,
buttons with Normal/Hover/Down/Disabled states, pictures, variables, triggers (slide/layer start,
click, variable change; show/hide layer, set variable, set object state, JavaScript), theme colors,
borders, shadows, text margins, alt text, hidden objects. Unsupported features are listed in the
import notes in the left panel, never dropped silently. The `.story` is never modified.

**Publish**: the Publish dialog (ribbon or File ▸ Publish) writes the web folder, the SCORM package, the PDF handout or the PowerPoint file; Preview plays the lesson in your browser.
The web player is original code (`crates/publish-html/src/runtime.js`). It runs triggers, plays
questions and results, reports completion, score (percent) and pass/fail to the LMS, and
saves/restores progress (variables, answers, current slide).

## Files

- `.brainbend`: a ZIP with `project.json` and the media as binary members (see docs/format.md).
  Older plain-JSON `.storyapp` files still open.
- Layout: `crates/doc` (model), `crates/import-legacy`, `crates/publish-html`, `apps/brainbend-studio`.

## Testing

```sh
cargo test --workspace                       # model, importer, exporter, editor logic
cd tests/player && npm install && npm test   # web player in jsdom with a mock SCORM LMS
```

Editor GUI tests (egui_kittest) run real frames through the sample and every slide kind to catch
panics; `BRAINBEND_SHOT_DIR=/some/dir cargo test -p brainbend-studio gui_screenshots` also renders PNGs of
several editor states for visual review.

The player tests cover triggers, keyboard activation, quiz gating/scoring/results/retry, SCORM 1.2
and 2004 initialization, score and pass/fail reporting, bookmarking and resume. They use a mock
LMS in jsdom, not a real browser or LMS; real-LMS testing is still needed.

## Known gaps

Reflection/glow, PowerPoint export of layer timelines and of triggers other than links, slide transitions other than a fade, 
"next state" actions, navigation-style triggers, a custom accessibility reading order (objects are read in stacking order; decorative ones can be skipped), and layer state on resume. PowerPoint charts and OLE objects, line and
paragraph spacing, and rotated-object resize handles are not supported yet. Course completion
currently means: every slide visited, or the quiz finished.

## Licensing

BrainBend Studio is dual-licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. Bundled fonts (Montserrat, Open Sans) are under the SIL Open Font
License 1.1; license files are in `assets/fonts/` and are copied into every export.

## Build a macOS app

```sh
./scripts/bundle-macos.sh    # builds dist/BrainBend Studio.app (release build, icon included)
```

You can also drag a `.story`, `.brainbend` or `.pptx` file onto the window to open it.
