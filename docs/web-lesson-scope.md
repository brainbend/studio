# Web Lesson: scope

A Web Lesson is a scrolling, mobile-friendly lesson built from content blocks, published as a
single responsive page.

## Goals

- Author a lesson as an ordered list of blocks, with no slide canvas and no timeline.
- Publish a single responsive page: phone, tablet and desktop, keyboard and screen-reader friendly.
- Same packaging as Media Lessons: HTML5, SCORM 1.2, SCORM 2004, with completion and score reporting.
- Reuse what already exists: questions, assets, themes, variables, the SCORM layer, print handout.

## Non-goals (first versions)

- Pixel-level free positioning. Blocks flow in a column; layout is chosen per block, not dragged.
- A browser-based editor. The editor stays the desktop app.
- Per-device layouts. The page is responsive by construction.

## Model

`Project.kind = WebLesson` already exists. Add `Project.lesson: Option<WebLesson>`:

- `WebLesson { sections: Vec<Section>, cover: Option<Cover>, nav: LessonNav, look: LessonLook }`
- `Section { id, title, blocks: Vec<Block> }` (a section is a lesson chapter; the menu lists them)
- `Block { id, kind: BlockKind, hidden_title?: .., width/align options per kind }`
- Rich text reuses `Paragraph`/`Run`, so links, bold, colors and the spell checker carry over.
- Knowledge-check blocks hold a `Question`, so every question type already built works unchanged,
  including banks, scoring, feedback and the print handout.
- Images, video, audio use the existing asset store and alt text / captions fields.

## Blocks

Phase 1 (enough to build a real lesson):
Heading, Text (paragraphs, bullets, numbers), Image (caption, width), Video, Audio, Button / link,
Divider, Spacer, Quote, Statement (large callout), Image + text (side by side, collapses on phones),
Knowledge check (any existing question type), Continue gate (learner must press Continue before
the next blocks appear).

Phase 2 (interactive):
Accordion, Tabs, Process (numbered steps), Timeline, Flashcards, Image carousel, Labeled graphic
(reuses the hotspot/label question regions), Table (reuses the table model), Embed (an HTML5 activity
or a Media Lesson).

Phase 3:
Scenario (branching with variables), Sorting / matching activities as non-graded blocks, Code,
Attachment download, Chart.

## Editor (egui)

- Left: section and block outline with drag to reorder and a "+" insert menu between blocks.
- Centre: a single-column preview of the lesson. Each block draws a simplified native preview and
  edits in place (text, captions, items). Fidelity is close, not exact, because egui cannot embed a
  web view; **Preview in browser** shows the real page.
- Right: Properties for the selected block (layout options, colors, alt text, question editor).
- Theme: reuse `Theme` for colors and fonts, plus a cover image / title block and dark mode option.

## Player

- New `web-runtime.js` and page template; the question UI (`runtime.js`, roughly lines 790 to 1450)
  moves into a shared file used by both players so Media and Web lessons score identically.
- Semantic HTML (headings, landmarks, native details/summary where possible), a skip link, focus
  management on Continue gates, `prefers-reduced-motion` respected.
- Progress: blocks seen (IntersectionObserver) and knowledge checks answered; menu shows section
  state. Completion rule is a setting: all blocks viewed, all checks answered, or checks passed.
- Reports completion, score, pass/fail, bookmark and suspend data through the existing SCORM layer.

## Milestones

1. **Foundation:** model, block list editor, text / image / button / divider, player, HTML5 export,
   SCORM reporting, project check for blocks (alt text, empty blocks, contrast). Tests: model,
   jsdom player, GUI render.
2. **Knowledge checks and gating:** question blocks, Continue gate, completion rules, shared
   question code, print handout for a lesson.
3. **Interactive blocks:** accordion, tabs, process, timeline, flashcards, carousel, labeled graphic.
4. **Look and navigation:** cover, section menu, progress bar, themes, dark mode, responsive polish.
5. **Embed block** (a Media Lesson inside a Web Lesson) and a scenario block.

## Risks and choices that matter

- **Two renderers** (egui preview and HTML player) must agree. Keep blocks simple and share
  layout rules (spacing scale, widths) as data so they cannot drift much.
- **Question code reuse** needs a careful refactor of `runtime.js`; the existing player tests are
  the safety net.
- **Accessibility** is a selling point here; build it in from block one rather than adding it later.
- **Rich text in blocks** is the heaviest editor piece; start with the existing run model.

## Decisions (2026-10-18)

1. A simplified in-app preview plus Preview in browser is acceptable.
2. Knowledge checks use the same interactions as Media Lessons: every question type, scoring,
   feedback, media. (Layers, triggers, states and animation stay Media-Lesson-only.)
3. Completion is a setting with both rules available: all blocks viewed, knowledge checks passed,
   or both (the default).
4. A section menu from the start (sidebar on wide screens, drawer on phones).

## Status

Milestones 1 and 2 are built: model (`crates/doc/src/lesson.rs`), publisher
(`crates/publish-html/src/web.rs`, `web_runtime.js`, `web.css`), editor
(`apps/brainbend-studio/src/lesson_view.rs`). The slide player and the lesson player share the LMS layer and
all question code (`shared_lms.js`, `shared_questions.js`). Interactive blocks (accordion, tabs,
process, timeline, flashcards) and the Continue gate are in. Not yet: labeled graphic, carousel,
table and embed blocks, scenarios, drag-to-reorder in the outline, spell squiggles in block text
boxes, lesson print handout, a quiz retry button.

## Questions for the owner (answered above)

1. Is a simplified in-app preview plus Preview in browser acceptable for block editing?
2. Which blocks do you reach for most often, to order Phase 2?
3. Should completion default to "all blocks viewed" or "knowledge checks passed"?
4. Should a lesson have a section menu from the start, or continuous scroll only at first?
