# `.brainbend` format (version 1, provisional)

A ZIP archive:

- `project.json` — the document (UTF-8 JSON, `"format": "brainbend.project/1"`).
- `assets/<n>` — one member per asset (images, video, audio, captions), stored uncompressed.
  `project.json` lists each asset (`name`, `mime`, `file`) under `assets`, keyed by an opaque id.

Readers also accept the earlier ids `storyapp.project/1` and `storyapp.project/0` (plain JSON with asset bytes as number arrays); files are rewritten with the current id when saved.

## Document model (see `crates/doc/src/lib.rs`)

- `Project`: `name`, `slides[]`, `assets{}`, `variables[]`, `pass_mark` (percent), `show_nav`, `notes[]`.
- `Slide`: `id`, `name`, `width`, `height`, `layers[]` (layer 0 is the base layer), `triggers[]`,
  `kind` (`Normal`, `Question{...}`, `Results`), `background`.
- `Object`: frame (`x y w h`), `kind` (`Rect`, `Text`, `Image`, `Video`, `Audio`, `Unsupported`),
  `fill`, `stroke`, `shadow`, `radius`, `margins`, `hidden`, `text` + `style`, `asset`, `captions`,
  `alt`, and optional named `states` (Normal/Hover/Down/Disabled looks) with `initial_state`.
- `Trigger`: `owner` (slide, layer or object) + `event` (SlideStart, LayerStart, Click,
  VariableChanged) + `action` (ShowLayer, HideLayer, SetVariable, SetObjectState, RunJavaScript,
  JumpToSlide, NextSlide, PreviousSlide) + `conditions[]` (evaluated left to right).

Ids are opaque strings, unique within a project. Unknown fields are ignored on read; new optional
fields get defaults. The format is not frozen: version bumps will come with migrations.

Readers enforce size limits and reject asset paths outside `assets/`.
