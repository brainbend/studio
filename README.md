# BrainBend Studio

A free, lightweight, open-source authoring tool for e-learning. Build slide-based Media Lessons
and scrolling Web Lessons, open PowerPoint (`.pptx`) and `.story` files, and publish HTML5 and
SCORM (1.2 and 2004 4th Edition) packages. Work in progress.

## Run

```sh
cargo run --release -p brainbend-studio                      # start empty (File ▸ New / Open)
cargo run --release -p brainbend-studio -- path/to/file.story
```

Requires a recent stable Rust toolchain. Tested on macOS.

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

**Publish**: File ▸ Preview in browser, Export HTML5, Export SCORM 1.2, Export SCORM 2004 4th Ed.
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
