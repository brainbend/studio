//! Working out what a lesson's video holds: which pictures to take, how long each shows, and where each sound starts.
//!
//! The video is made from still pictures of the slides. A slide's picture only changes when an object appears or
//! disappears on its timeline, so one picture is taken for each stretch between those moments and shown for that long.
//! Objects pop in and out on their timing; entrance, exit and emphasis effects and motion paths are not played yet
//! (they need the app to draw a slide at any moment, which is #13). A slide's own transition is a blend between the
//! last picture of the slide before and the first pictures of the slide, done by ffmpeg.
use brainbend_doc::{Edge, Kind, Object, Project, SlideKind, Timing, Transition, TransitionKind};

/// What the author chose on the Video page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Pixels high: 720 or 1080. The width follows the slides' shape.
    pub height: u32,
    pub fps: u32,
    /// How long a slide with no timeline and no sound shows.
    pub hold_ms: u32,
    /// Put the slides' sound in the video.
    pub audio: bool,
}

impl Default for Settings {
    fn default() -> Self { Self { height: 720, fps: 30, hold_ms: 5000, audio: true } }
}

/// What is known about a media file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaInfo {
    pub duration_ms: u64,
    pub has_audio: bool,
}

/// One picture of a slide, and how long it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shot {
    /// Index of the slide in the project.
    pub slide: usize,
    /// Moment within the slide that the picture shows.
    pub at_ms: u32,
    /// Where this picture starts in the whole video.
    pub start_ms: u64,
    pub length_ms: u32,
}

/// A sound, placed in the video.
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    /// The asset key.
    pub asset: String,
    pub start_ms: u64,
    /// The most of it that plays.
    pub length_ms: u64,
    /// 0.0 - 1.0.
    pub volume: f32,
    pub looped: bool,
}

/// A transition into a slide: it blends the slide before into the slide's first pictures, over `ms`, from the moment
/// the slide starts. It does not make the video longer: the slide's own time runs underneath it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blend {
    /// Index in `Plan::shots` of the slide's first picture.
    pub shot: usize,
    /// The name of ffmpeg's `xfade` transition that looks most like it.
    pub name: &'static str,
    pub ms: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub shots: Vec<Shot>,
    pub clips: Vec<Clip>,
    pub blends: Vec<Blend>,
    pub total_ms: u64,
}

/// The ffmpeg `xfade` transition that looks most like a slide transition. The edge is the one the new slide comes from.
pub fn xfade_name(t: &Transition) -> &'static str {
    use Edge::*;
    match (t.kind, t.from) {
        (TransitionKind::Fade, _) => "fade",
        (TransitionKind::Push, Right) => "slideleft",
        (TransitionKind::Push, Left) => "slideright",
        (TransitionKind::Push, Top) => "slidedown",
        (TransitionKind::Push, Bottom) => "slideup",
        (TransitionKind::Wipe, Right) => "wipeleft",
        (TransitionKind::Wipe, Left) => "wiperight",
        (TransitionKind::Wipe, Top) => "wipedown",
        (TransitionKind::Wipe, Bottom) => "wipeup",
        (TransitionKind::Cover, Right) => "coverleft",
        (TransitionKind::Cover, Left) => "coverright",
        (TransitionKind::Cover, Top) => "coverdown",
        (TransitionKind::Cover, Bottom) => "coverup",
        (TransitionKind::Uncover, Right) => "revealleft",
        (TransitionKind::Uncover, Left) => "revealright",
        (TransitionKind::Uncover, Top) => "revealdown",
        (TransitionKind::Uncover, Bottom) => "revealup",
        // Vertical lines for left and right, horizontal ones for top and bottom; "in" closes to the middle.
        (TransitionKind::Split, Left | Right) => if t.inward { "vertclose" } else { "vertopen" },
        (TransitionKind::Split, Top | Bottom) => if t.inward { "horzclose" } else { "horzopen" },
        (TransitionKind::Zoom, _) => if t.inward { "zoomin" } else { "circleclose" },
        (TransitionKind::Dissolve, _) => "dissolve",
    }
}

/// Whether an object with `timing` is on show `t` milliseconds into its slide.
pub fn visible_at(timing: Option<Timing>, t: u32) -> bool {
    match timing {
        None => true,
        Some(Timing { start_ms, dur_ms }) => t >= start_ms && dur_ms.is_none_or(|d| t < start_ms.saturating_add(d)),
    }
}

/// Whether the object has anything to see in the video: not one that starts hidden, a sound (audio has no picture) or a
/// hotspot (a click target).
pub fn draws(o: &Object) -> bool { !o.hidden && !matches!(o.kind, Kind::Audio | Kind::Hotspot) }

/// Whether the object is drawn in the video `t` milliseconds into its slide: it has something to see, and its turn on
/// the timeline has come and not gone.
pub fn drawn_at(o: &Object, t: u32) -> bool { draws(o) && visible_at(o.timing, t) }

/// Sound that starts by itself: audio, or video with its own sound, set to play automatically and not muted.
fn plays_sound(o: &Object) -> bool {
    matches!(o.kind, Kind::Audio | Kind::Video) && o.asset.is_some() && o.media.autoplay && !o.media.muted && o.media.volume > 0.0 && !o.hidden
}

/// What the video leaves out, one line for each thing and slide, for the author to read after it is made. `info` is the
/// same as for [`plan`]: a sound it knows nothing about could not be read.
pub fn left_out(project: &Project, slides: &[usize], settings: &Settings, info: &dyn Fn(&str) -> Option<MediaInfo>) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    if !settings.audio { out.push("Sound was turned off, so none of the slides' sound is in the video.".into()); }
    let first_shape = slides.iter().find_map(|&i| project.slides.get(i)).map(|s| if s.height > 0.0 { s.width / s.height } else { 1.0 });
    let plural = |n: usize, one: &str, many: &str| if n == 1 { format!("1 {one}") } else { format!("{n} {many}") };
    for &si in slides {
        let Some(slide) = project.slides.get(si) else { continue };
        // The slide's number, with its name unless that only says the number again.
        let name = if slide.name.trim().is_empty() || slide.name.trim() == format!("Slide {}", si + 1) { format!("Slide {}", si + 1) } else { format!("Slide {} ({})", si + 1, slide.name.trim()) };
        if let (Some(a), true) = (first_shape, slide.height > 0.0) {
            if (slide.width / slide.height - a).abs() > a * 0.01 { out.push(format!("{name} is a different shape from the first slide, so it is centred on white.")); }
        }
        match &slide.kind {
            SlideKind::Question(_) => { out.push(format!("{name} is a question. It shows unanswered for {} s; nothing can be answered in a video.", settings.hold_ms / 1000)); continue; }
            SlideKind::Results => { out.push(format!("{name} is a results slide. It shows for {} s without a score.", settings.hold_ms / 1000)); continue; }
            SlideKind::Normal => {}
        }
        let shown = slide.start_layers();
        let objects = || shown.iter().flat_map(|&li| slide.layers[li].objects.iter());
        let effects = objects().filter(|o| draws(o)).map(|o| [o.entrance.is_some(), o.exit.is_some(), o.emphasis.is_some(), o.motion.is_some()].iter().filter(|b| **b).count()).sum::<usize>();
        if effects > 0 {
            out.push(format!("{name}: {} (entrance, exit, emphasis or motion path) {} not played. Objects appear and disappear at their times on the timeline.", plural(effects, "effect", "effects"), if effects == 1 { "is" } else { "are" }));
        }
        let later: Vec<&str> = slide.layers.iter().enumerate().filter(|(li, _)| !shown.contains(li)).map(|(_, l)| l.name.as_str()).collect();
        if !later.is_empty() {
            out.push(format!("{name}: {} only {} when something triggers {}, so {} not in the video.", later.iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(", "), if later.len() == 1 { "shows" } else { "show" }, if later.len() == 1 { "it" } else { "them" }, if later.len() == 1 { "it is" } else { "they are" }));
        }
        let hidden = objects().filter(|o| o.hidden && !matches!(o.kind, Kind::Hotspot)).count();
        if hidden > 0 {
            out.push(format!("{name}: {} {} hidden and a trigger shows {}, so {} not in the video.", plural(hidden, "object", "objects"), if hidden == 1 { "starts" } else { "start" }, if hidden == 1 { "it" } else { "them" }, if hidden == 1 { "it is" } else { "they are" }));
        }
        let triggers = slide.triggers.iter().filter(|t| !matches!((&t.event, &t.action), (brainbend_doc::Event::SlideStart, brainbend_doc::Action::ShowLayer(_)))).count();
        if triggers > 0 { out.push(format!("{name}: {} (buttons, variables, conditions) {} nothing in a video.", plural(triggers, "trigger", "triggers"), if triggers == 1 { "does" } else { "do" })); }
        for o in objects().filter(|o| matches!(o.kind, Kind::Video | Kind::Audio) && o.asset.is_some() && !o.hidden) {
            let what = if matches!(o.kind, Kind::Video) { "video" } else { "audio" };
            let known = info(o.asset.as_deref().unwrap_or_default());
            if matches!(o.kind, Kind::Video) { out.push(format!("{name}: the video \"{}\" shows its picture and does not play.", o.name)); }
            if o.captions.is_some() { out.push(format!("{name}: the captions of \"{}\" are not in the video.", o.name)); }
            if !settings.audio || o.media.muted { continue; }
            if !o.media.autoplay {
                if known.is_none_or(|m| m.has_audio) { out.push(format!("{name}: the {what} \"{}\" waits to be clicked, so its sound is not in the video.", o.name)); }
            } else if known.is_none() {
                out.push(format!("{name}: the sound of the {what} \"{}\" could not be read, so it is not in the video.", o.name));
            } else if matches!(o.kind, Kind::Audio) && known.is_some_and(|m| !m.has_audio) {
                out.push(format!("{name}: the audio \"{}\" has no sound, so it is not in the video.", o.name));
            }
        }
    }
    out
}

/// The assets whose sound could play in the video of `slides`, each once: what [`plan`] will ask `info` about.
pub fn sound_assets(project: &Project, slides: &[usize]) -> Vec<String> {
    let mut keys: Vec<String> = vec![];
    for &si in slides {
        let Some(slide) = project.slides.get(si).filter(|s| matches!(s.kind, SlideKind::Normal)) else { continue };
        for &li in &slide.start_layers() {
            for o in slide.layers[li].objects.iter().filter(|o| plays_sound(o)) {
                if let Some(k) = &o.asset { if !keys.contains(k) { keys.push(k.clone()); } }
            }
        }
    }
    keys
}

/// The plan for `slides` (indices into the project, in the order they play). `info` says how long a media file is and
/// whether it has sound; a file it knows nothing about adds no sound.
pub fn plan(project: &Project, slides: &[usize], settings: &Settings, info: &dyn Fn(&str) -> Option<MediaInfo>) -> Plan {
    let mut out = Plan::default();
    for &si in slides {
        let Some(slide) = project.slides.get(si) else { continue };
        let offset = out.total_ms;
        // Only a normal slide has a timeline; a question or results slide is held, as the player waits on it.
        let normal = matches!(slide.kind, SlideKind::Normal);
        let shown = if normal { slide.start_layers() } else { vec![] };
        let mut clips: Vec<(Clip, u64, Option<u64>)> = vec![];   // the clip, its length if played through, the object's end
        if normal && settings.audio {
            for &li in &shown {
                for o in slide.layers[li].objects.iter().filter(|o| plays_sound(o)) {
                    let key = o.asset.clone().unwrap_or_default();
                    let Some(m) = info(&key).filter(|m| m.has_audio && m.duration_ms > 0) else { continue };
                    let start = o.timing.map_or(0, |t| t.start_ms as u64);
                    let end = o.timing.and_then(|t| t.dur_ms.map(|d| t.start_ms as u64 + d as u64));
                    clips.push((Clip { asset: key, start_ms: offset + start, length_ms: m.duration_ms, volume: o.media.volume.clamp(0.0, 1.0), looped: o.media.looped }, m.duration_ms, end));
                }
            }
        }
        let timeline = shown.iter().filter_map(|&li| slide.layers[li].duration_ms).max().unwrap_or(0) as u64;
        let sound_end = clips.iter().filter(|(c, _, _)| !c.looped).map(|(c, len, end)| (c.start_ms - offset + len).min(end.unwrap_or(u64::MAX))).max().unwrap_or(0);
        let length = if timeline > 0 || sound_end > 0 { timeline.max(sound_end) } else { settings.hold_ms as u64 }.max(1);
        // A sound plays no further than its own turn on the timeline, nor past the end of the slide.
        for (mut clip, len, end) in clips {
            let from = clip.start_ms - offset;
            let stop = end.unwrap_or(u64::MAX).min(length);
            clip.length_ms = if clip.looped { stop.saturating_sub(from) } else { len.min(stop.saturating_sub(from)) };
            if clip.length_ms > 0 { out.clips.push(clip); }
        }
        // The moments the picture changes: when an object's time comes or goes.
        let mut cuts: Vec<u32> = vec![0];
        for &li in &shown {
            for t in slide.layers[li].objects.iter().filter(|o| draws(o)).filter_map(|o| o.timing) {
                cuts.push(t.start_ms);
                if let Some(d) = t.dur_ms { cuts.push(t.start_ms.saturating_add(d)); }
            }
        }
        cuts.retain(|&c| (c as u64) < length);
        cuts.sort_unstable();
        cuts.dedup();
        // What shows in each stretch; neighbours that show the same things are one picture.
        let signature = |at: u32| -> Vec<(usize, usize)> {
            shown.iter().flat_map(|&li| slide.layers[li].objects.iter().enumerate().filter(move |(_, o)| drawn_at(o, at)).map(move |(oi, _)| (li, oi))).collect()
        };
        let mut stretches: Vec<(u32, Vec<(usize, usize)>)> = vec![];
        for &at in &cuts {
            let sig = if normal { signature(at) } else { vec![] };
            if stretches.last().is_none_or(|(_, last)| *last != sig) { stretches.push((at, sig)); }
        }
        // The slide's transition blends the one before into it; the first slide has nothing before it to blend.
        if !out.shots.is_empty() && slide.fade_ms > 0 {
            out.blends.push(Blend { shot: out.shots.len(), name: xfade_name(&slide.transition), ms: slide.fade_ms.min(length.min(u32::MAX as u64) as u32) });
        }
        for (i, (at, _)) in stretches.iter().enumerate() {
            let end = stretches.get(i + 1).map_or(length, |(next, _)| *next as u64);
            out.shots.push(Shot { slide: si, at_ms: *at, start_ms: offset + *at as u64, length_ms: (end - *at as u64) as u32 });
        }
        out.total_ms += length;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainbend_doc::{Slide, SlideKind};

    fn project(slides: Vec<Slide>) -> Project {
        let mut p = Project::new("t");
        p.slides = slides;
        p
    }

    fn obj(id: &str, kind: Kind, timing: Option<Timing>) -> Object {
        let mut o = Object::new(id, id, kind, 0.0, 0.0, 10.0, 10.0);
        o.timing = timing;
        o
    }

    fn sound(id: &str, asset: &str, timing: Option<Timing>) -> Object {
        let mut o = obj(id, Kind::Audio, timing);
        o.asset = Some(asset.into());
        o.media.autoplay = true;
        o
    }

    fn t(start: u32, dur: Option<u32>) -> Option<Timing> { Some(Timing { start_ms: start, dur_ms: dur }) }
    fn no_media(_: &str) -> Option<MediaInfo> { None }

    #[test]
    fn an_object_shows_during_its_turn() {
        assert!(visible_at(None, 0) && visible_at(None, 99_999));
        assert!(!visible_at(t(1000, Some(500)), 999));
        assert!(visible_at(t(1000, Some(500)), 1000) && visible_at(t(1000, Some(500)), 1499));
        assert!(!visible_at(t(1000, Some(500)), 1500));
        assert!(visible_at(t(1000, None), 1_000_000), "no end: until the end of the slide");
    }

    #[test]
    fn a_slide_with_nothing_timed_is_one_picture_held_for_the_default_time() {
        let mut s = Slide::new("s1", "S1", 1280.0, 720.0);
        s.layers[0].objects.push(obj("a", Kind::Rect, None));
        let p = plan(&project(vec![s]), &[0], &Settings { hold_ms: 4000, ..Default::default() }, &no_media);
        assert_eq!(p.shots, vec![Shot { slide: 0, at_ms: 0, start_ms: 0, length_ms: 4000 }]);
        assert_eq!(p.total_ms, 4000);
        assert!(p.clips.is_empty());
    }

    #[test]
    fn the_picture_changes_only_when_something_appears_or_leaves() {
        let mut s = Slide::new("s1", "S1", 1280.0, 720.0);
        s.layers[0].duration_ms = Some(6000);
        s.layers[0].objects.push(obj("always", Kind::Rect, None));
        s.layers[0].objects.push(obj("title", Kind::Text, t(0, Some(2000))));
        s.layers[0].objects.push(obj("late", Kind::Text, t(2000, None)));
        let p = plan(&project(vec![s]), &[0], &Settings::default(), &no_media);
        // 0-2s: always + title. 2s-end: always + late. The title leaving and the other arriving are one change.
        assert_eq!(p.shots.iter().map(|s| (s.at_ms, s.length_ms)).collect::<Vec<_>>(), vec![(0, 2000), (2000, 4000)]);
        assert_eq!(p.total_ms, 6000);
    }

    #[test]
    fn a_moment_that_changes_nothing_does_not_make_another_picture() {
        let mut s = Slide::new("s1", "S1", 1280.0, 720.0);
        s.layers[0].duration_ms = Some(5000);
        let mut h = obj("hidden", Kind::Text, t(1000, Some(1000)));
        h.hidden = true;
        s.layers[0].objects.push(h);
        s.layers[0].objects.push(obj("a", Kind::Rect, None));
        let p = plan(&project(vec![s]), &[0], &Settings::default(), &no_media);
        assert_eq!(p.shots.len(), 1, "an object that starts hidden never shows, so its times are not cuts");
        assert_eq!(p.shots[0].length_ms, 5000);
    }

    #[test]
    fn slides_follow_one_another_and_a_question_is_held_without_sound() {
        let mut a = Slide::new("a", "A", 1280.0, 720.0);
        a.layers[0].duration_ms = Some(3000);
        let mut q = Slide::new("q", "Q", 1280.0, 720.0);
        q.kind = SlideKind::Results;
        q.layers[0].objects.push(sound("snd", "clip", None));
        let info = |_: &str| Some(MediaInfo { duration_ms: 9000, has_audio: true });
        let p = plan(&project(vec![a, q]), &[0, 1], &Settings { hold_ms: 2500, ..Default::default() }, &info);
        assert_eq!(p.shots.iter().map(|s| (s.slide, s.start_ms, s.length_ms)).collect::<Vec<_>>(), vec![(0, 0, 3000), (1, 3000, 2500)]);
        assert!(p.clips.is_empty(), "the held slide is silent");
        assert_eq!(p.total_ms, 5500);
    }

    #[test]
    fn sound_lengthens_a_slide_and_starts_at_its_place_on_the_timeline() {
        let mut a = Slide::new("a", "A", 1280.0, 720.0);
        a.layers[0].duration_ms = Some(2000);
        a.layers[0].objects.push(sound("intro", "narration", t(500, None)));
        let mut b = Slide::new("b", "B", 1280.0, 720.0);
        b.layers[0].objects.push(sound("bing", "bing", None));
        let info = |k: &str| Some(MediaInfo { duration_ms: if k == "narration" { 6000 } else { 1000 }, has_audio: true });
        let p = plan(&project(vec![a, b]), &[0, 1], &Settings::default(), &info);
        assert_eq!(p.shots[0].length_ms, 6500, "slide A lasts until its narration ends (0.5s + 6s)");
        assert_eq!(p.clips[0], Clip { asset: "narration".into(), start_ms: 500, length_ms: 6000, volume: 1.0, looped: false });
        assert_eq!(p.clips[1].start_ms, 6500, "slide B's sound starts when B does");
        assert_eq!(p.total_ms, 7500);
    }

    #[test]
    fn sound_that_does_not_start_by_itself_or_is_muted_or_has_no_audio_is_left_out() {
        let mut s = Slide::new("s", "S", 1280.0, 720.0);
        let mut manual = sound("manual", "c", None);
        manual.media.autoplay = false;
        let mut muted = sound("muted", "c", None);
        muted.media.muted = true;
        s.layers[0].objects.extend([manual, muted, sound("silent", "none", None)]);
        let info = |k: &str| Some(MediaInfo { duration_ms: 3000, has_audio: k != "none" });
        let p = plan(&project(vec![s.clone()]), &[0], &Settings::default(), &info);
        assert!(p.clips.is_empty());
        assert_eq!(p.total_ms, 5000, "so the slide is just held");
        s.layers[0].objects.push(sound("on", "c", None));
        let off = Settings { audio: false, ..Default::default() };
        assert!(plan(&project(vec![s]), &[0], &off, &info).clips.is_empty(), "audio can be left out");
    }

    #[test]
    fn a_sound_stops_when_its_turn_ends_and_a_loop_runs_to_then() {
        let mut s = Slide::new("s", "S", 1280.0, 720.0);
        s.layers[0].duration_ms = Some(10_000);
        s.layers[0].objects.push(sound("short", "long", t(1000, Some(2000))));
        let mut music = sound("music", "loop", t(4000, None));
        music.media.looped = true;
        s.layers[0].objects.push(music);
        let info = |k: &str| Some(MediaInfo { duration_ms: if k == "long" { 30_000 } else { 3000 }, has_audio: true });
        let p = plan(&project(vec![s]), &[0], &Settings::default(), &info);
        assert_eq!(p.clips[0].length_ms, 2000, "cut at the end of its turn");
        assert_eq!((p.clips[1].start_ms, p.clips[1].length_ms, p.clips[1].looped), (4000, 6000, true), "the loop runs to the end of the slide");
        assert_eq!(p.total_ms, 10_000);
    }

    #[test]
    fn the_assets_to_look_at_are_the_ones_that_could_play() {
        let mut s = Slide::new("s", "S", 1280.0, 720.0);
        let mut manual = sound("manual", "never", None);
        manual.media.autoplay = false;
        s.layers[0].objects.extend([sound("a", "one", None), sound("b", "one", None), sound("c", "two", None), manual]);
        assert_eq!(sound_assets(&project(vec![s]), &[0]), vec!["one".to_string(), "two".to_string()]);
    }

    #[test]
    fn a_slides_transition_blends_the_one_before_into_it_but_the_first_slide_has_none() {
        let mut a = Slide::new("a", "A", 1280.0, 720.0);
        a.fade_ms = 500;
        a.layers[0].duration_ms = Some(3000);
        let mut b = Slide::new("b", "B", 1280.0, 720.0);
        b.fade_ms = 800;
        b.transition = Transition { kind: TransitionKind::Wipe, from: Edge::Left, inward: false };
        b.layers[0].duration_ms = Some(4000);
        let mut c = Slide::new("c", "C", 1280.0, 720.0);
        c.fade_ms = 0;
        let p = plan(&project(vec![a, b, c]), &[0, 1, 2], &Settings::default(), &no_media);
        assert_eq!(p.blends, vec![Blend { shot: 1, name: "wiperight", ms: 800 }], "slide A has nothing before it and C is a cut");
        assert_eq!(p.total_ms, 12_000, "a transition does not add time");
    }

    #[test]
    fn a_transition_is_never_longer_than_its_slide() {
        let mut a = Slide::new("a", "A", 1280.0, 720.0);
        a.layers[0].duration_ms = Some(2000);
        let mut b = Slide::new("b", "B", 1280.0, 720.0);
        b.fade_ms = 5000;
        b.layers[0].duration_ms = Some(1500);
        let p = plan(&project(vec![a, b]), &[0, 1], &Settings::default(), &no_media);
        assert_eq!(p.blends[0].ms, 1500);
    }

    #[test]
    fn every_kind_of_transition_has_an_ffmpeg_blend() {
        for kind in TransitionKind::ALL {
            for t in Transition::variants(kind) {
                let name = xfade_name(&t);
                assert!(!name.is_empty(), "{kind:?}");
            }
        }
        assert_eq!(xfade_name(&Transition::default()), "fade");
        assert_eq!(xfade_name(&Transition { kind: TransitionKind::Push, from: Edge::Right, inward: false }), "slideleft");
        assert_eq!(xfade_name(&Transition { kind: TransitionKind::Split, from: Edge::Left, inward: true }), "vertclose");
    }

    fn lines(p: &Project, settings: &Settings, info: &dyn Fn(&str) -> Option<MediaInfo>) -> Vec<String> { left_out(p, &[0, 1, 2, 3], settings, info) }

    #[test]
    fn a_plain_lesson_has_nothing_to_report() {
        let a = Slide::new("a", "A", 1280.0, 720.0);
        let b = Slide::new("b", "B", 1280.0, 720.0);
        assert!(left_out(&project(vec![a, b]), &[0, 1], &Settings::default(), &no_media).is_empty());
    }

    #[test]
    fn effects_hidden_objects_other_layers_and_triggers_are_reported_by_slide() {
        use brainbend_doc::{Action, Entrance, EffectKind, Event, Layer, Owner, Trigger};
        let _ = EffectKind::Fade;
        let mut s = Slide::new("s", "Intro", 1280.0, 720.0);
        let mut fx = obj("fx", Kind::Rect, None);
        fx.entrance = Some(Entrance { kind: EffectKind::Fade, delay_ms: 0, duration_ms: 500 });
        let mut gone = obj("gone", Kind::Text, None);
        gone.hidden = true;
        s.layers[0].objects.extend([fx, gone]);
        s.layers.push(Layer { id: "pop".into(), name: "Pop-up".into(), objects: vec![], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] });
        s.triggers.push(Trigger { id: "t1".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::ShowLayer("s-base".into()), conditions: vec![] });
        s.triggers.push(Trigger { id: "t2".into(), owner: Owner::Slide, event: Event::SlideStart, action: Action::ShowLayer("pop".into()), conditions: vec![] });
        s.triggers.push(Trigger { id: "t3".into(), owner: Owner::Slide, event: Event::SlideEnd, action: Action::ShowLayer("pop".into()), conditions: vec![] });
        let r = lines(&project(vec![s]), &Settings::default(), &no_media);
        assert_eq!(r.len(), 3, "{r:?}");
        assert_eq!(r[0], "Slide 1 (Intro): 1 effect (entrance, exit, emphasis or motion path) is not played. Objects appear and disappear at their times on the timeline.");
        assert_eq!(r[1], "Slide 1 (Intro): 1 object starts hidden and a trigger shows it, so it is not in the video.");
        assert_eq!(r[2], "Slide 1 (Intro): 1 trigger (buttons, variables, conditions) does nothing in a video.");
    }

    #[test]
    fn a_slide_is_named_by_number_alone_when_its_name_says_the_same() {
        let mut s = Slide::new("s", "Slide 1", 1280.0, 720.0);
        let mut ghost = obj("ghost", Kind::Text, None);
        ghost.hidden = true;
        s.layers[0].objects.push(ghost);
        let mut unnamed = Slide::new("u", "  ", 1280.0, 720.0);
        let mut ghost = obj("ghost", Kind::Text, None);
        ghost.hidden = true;
        unnamed.layers[0].objects.push(ghost);
        let r = lines(&project(vec![s, unnamed]), &Settings::default(), &no_media);
        assert!(r[0].starts_with("Slide 1: ") && r[1].starts_with("Slide 2: "), "{r:?}");
    }

    #[test]
    fn a_layer_that_shows_only_when_triggered_is_named() {
        use brainbend_doc::Layer;
        let mut s = Slide::new("s", "Intro", 1280.0, 720.0);
        s.layers.push(Layer { id: "pop".into(), name: "Pop-up".into(), objects: vec![], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] });
        let r = lines(&project(vec![s]), &Settings::default(), &no_media);
        assert_eq!(r, vec!["Slide 1 (Intro): \"Pop-up\" only shows when something triggers it, so it is not in the video.".to_string()]);
    }

    #[test]
    fn questions_and_results_and_other_shaped_slides_are_reported() {
        let a = Slide::new("a", "A", 1280.0, 720.0);
        let mut q = Slide::new("q", "Quiz", 1280.0, 720.0);
        q.kind = SlideKind::Question(brainbend_doc::Question::new_single());
        let mut res = Slide::new("r", "Score", 1280.0, 720.0);
        res.kind = SlideKind::Results;
        let tall = Slide::new("t", "Tall", 720.0, 1280.0);
        let r = lines(&project(vec![a, q, res, tall]), &Settings { hold_ms: 4000, ..Default::default() }, &no_media);
        assert_eq!(r, vec![
            "Slide 2 (Quiz) is a question. It shows unanswered for 4 s; nothing can be answered in a video.".to_string(),
            "Slide 3 (Score) is a results slide. It shows for 4 s without a score.".to_string(),
            "Slide 4 (Tall) is a different shape from the first slide, so it is centred on white.".to_string(),
        ]);
    }

    #[test]
    fn media_is_reported_for_what_does_not_play_and_what_sound_is_missing() {
        let mut s = Slide::new("s", "Media", 1280.0, 720.0);
        let mut video = obj("v", Kind::Video, None);
        video.asset = Some("clip".into());
        video.captions = Some("cc".into());
        let click = { let mut o = sound("click", "ding", None); o.media.autoplay = false; o };
        let broken = sound("broken", "bad", None);
        let silent = sound("silent", "quiet", None);
        let muted = { let mut o = sound("muted", "ding", None); o.media.muted = true; o };
        s.layers[0].objects.extend([video, click, broken, silent, muted]);
        let info = |k: &str| match k { "bad" => None, "quiet" => Some(MediaInfo { duration_ms: 1000, has_audio: false }), _ => Some(MediaInfo { duration_ms: 1000, has_audio: true }) };
        let r = lines(&project(vec![s]), &Settings::default(), &info);
        assert_eq!(r, vec![
            "Slide 1 (Media): the video \"v\" shows its picture and does not play.".to_string(),
            "Slide 1 (Media): the captions of \"v\" are not in the video.".to_string(),
            "Slide 1 (Media): the video \"v\" waits to be clicked, so its sound is not in the video.".to_string(),
            "Slide 1 (Media): the audio \"click\" waits to be clicked, so its sound is not in the video.".to_string(),
            "Slide 1 (Media): the sound of the audio \"broken\" could not be read, so it is not in the video.".to_string(),
            "Slide 1 (Media): the audio \"silent\" has no sound, so it is not in the video.".to_string(),
        ], "a muted sound was meant to be silent, so it is not reported");
    }

    #[test]
    fn with_sound_turned_off_one_line_says_so_instead_of_one_for_each_sound() {
        let mut s = Slide::new("s", "S", 1280.0, 720.0);
        let mut click = sound("click", "ding", None);
        click.media.autoplay = false;
        s.layers[0].objects.extend([click, sound("a", "ding", None)]);
        let info = |_: &str| Some(MediaInfo { duration_ms: 1000, has_audio: true });
        let r = lines(&project(vec![s]), &Settings { audio: false, ..Default::default() }, &info);
        assert_eq!(r, vec!["Sound was turned off, so none of the slides' sound is in the video.".to_string()]);
    }

    #[test]
    fn slides_that_are_not_there_are_skipped() {
        let s = Slide::new("s", "S", 1280.0, 720.0);
        let p = plan(&project(vec![s]), &[5, 0], &Settings::default(), &no_media);
        assert_eq!(p.shots.len(), 1);
    }
}
