//! The commands of the app, written down once.
//!
//! A command knows its name, its shortcut, whether it can run right now and whether it is switched on; running it does the same thing wherever
//! it is asked for from: a menu, a button, the keyboard. The Edit commands do the right thing for what has the keyboard (the slides when the slide
//! list or Scene View has it, the objects on the slide otherwise); the Window commands show and hide the panels and lay them out.
use crate::{dock, App};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Cmd {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    Duplicate,
    SelectAll,
    Find,
    /// Show or hide a panel (a tab number, see `main.rs`).
    TogglePanel(usize),
    /// Lay the panels out the way a preset says.
    Preset(dock::Preset),
}

/// The Edit menu, in order (a gap between the groups goes after Redo, Delete and SelectAll).
pub(crate) const EDIT: [Cmd; 9] = [Cmd::Undo, Cmd::Redo, Cmd::Cut, Cmd::Copy, Cmd::Paste, Cmd::Delete, Cmd::Duplicate, Cmd::SelectAll, Cmd::Find];

/// The panels the Window menu switches on and off, in the order they sit on screen.
pub(crate) const PANELS: [usize; 8] = [crate::P_SLIDES, 0, crate::P_TRIGGERS, 2, crate::P_TIMELINE, crate::P_NOTES, crate::P_STATES, dock::LAYERS];

/// The Window menu: a switch for each panel, then the layouts.
pub(crate) fn window() -> Vec<Cmd> {
    PANELS.iter().map(|&g| Cmd::TogglePanel(g)).chain(dock::Preset::ALL.iter().map(|&p| Cmd::Preset(p))).collect()
}

impl Cmd {
    pub(crate) fn label(self) -> String {
        match self {
            Cmd::Undo => "Undo".into(),
            Cmd::Redo => "Redo".into(),
            Cmd::Cut => "Cut".into(),
            Cmd::Copy => "Copy".into(),
            Cmd::Paste => "Paste".into(),
            Cmd::Delete => "Delete".into(),
            Cmd::Duplicate => "Duplicate".into(),
            Cmd::SelectAll => "Select All".into(),
            Cmd::Find => "Find and Replace".into(),
            Cmd::TogglePanel(g) => App::panel_title(g).into(),
            Cmd::Preset(p) => p.name().into(),
        }
    }

    /// The keys that run it, as a menu shows them (⌘ on a Mac, Ctrl elsewhere).
    pub(crate) fn shortcut(self) -> Option<String> {
        let key = match self {
            Cmd::Undo => "Z",
            Cmd::Redo => "⇧Z",
            Cmd::Cut => "X",
            Cmd::Copy => "C",
            Cmd::Paste => "V",
            Cmd::Duplicate => "D",
            Cmd::SelectAll => "A",
            Cmd::Find => "F",
            Cmd::Delete => return Some(if cfg!(target_os = "macos") { "⌫".into() } else { "Del".into() }),
            Cmd::TogglePanel(_) | Cmd::Preset(_) => return None,
        };
        Some(if cfg!(target_os = "macos") { format!("⌘{}", key.replace("⇧", "⇧")) } else { format!("Ctrl+{}", key.replace('⇧', "Shift+")) })
    }
}

impl App {
    /// Whether the keys and the Edit menu are about slides: the slide list or the Scene View is what was last clicked.
    pub(crate) fn editing_slides(&self) -> bool { self.scene_view || self.slides_focus }

    /// Whether a command can run now (a menu greys it out when not).
    pub(crate) fn cmd_enabled(&self, c: Cmd) -> bool {
        if self.project.is_none() { return false; }
        let slides = self.editing_slides();
        match c {
            Cmd::Undo => !self.undo.is_empty(),
            Cmd::Redo => !self.redo.is_empty(),
            Cmd::Cut | Cmd::Delete => if slides { self.project.as_ref().is_some_and(|p| p.slides.len() > self.slide_selection().len()) } else { self.selected.is_some() },
            Cmd::Copy | Cmd::Duplicate => slides || self.selected.is_some(),
            Cmd::Paste => if slides { !self.slide_clip.is_empty() } else { self.clipboard.is_some() },
            Cmd::SelectAll | Cmd::Find | Cmd::TogglePanel(_) | Cmd::Preset(_) => true,
        }
    }

    /// Whether a switch is on: `None` for a command that is not a switch.
    pub(crate) fn cmd_checked(&self, c: Cmd) -> Option<bool> {
        match c { Cmd::TogglePanel(g) => Some(self.panel_shown(g)), _ => None }
    }

    /// Do it.
    pub(crate) fn run_cmd(&mut self, c: Cmd) {
        let slides = self.editing_slides();
        match c {
            Cmd::Undo => self.undo(),
            Cmd::Redo => self.redo(),
            Cmd::Copy => if slides { self.copy_slides() } else { self.copy_selected() },
            Cmd::Cut => if slides { self.copy_slides(); let sel = self.slide_selection(); self.delete_slides(sel); } else { self.copy_selected(); self.delete_selected(); },
            Cmd::Paste => if slides { self.paste_slides() } else { self.paste() },
            Cmd::Delete => if slides { let sel = self.slide_selection(); self.delete_slides(sel); } else { self.delete_selected() },
            Cmd::Duplicate => if slides { self.duplicate_slides() } else { self.duplicate_selected() },
            Cmd::SelectAll => if slides { self.select_all_slides() } else { self.select_all_objects() },
            Cmd::Find => { self.find.get_or_insert_with(Default::default).focus = true; }
            Cmd::TogglePanel(g) => { let on = !self.panel_shown(g); self.set_panel_shown(g, on); }
            Cmd::Preset(p) => self.apply_preset(p),
        }
    }

    /// Every slide of the scene being shown (or every slide, in the Scene View).
    fn select_all_slides(&mut self) {
        let Some(p) = &self.project else { return };
        let scene = p.slides.get(self.slide).map(|s| s.scene.clone());
        let all = self.scene_view;
        self.slide_multi = p.slides.iter().enumerate().filter(|(i, s)| *i != self.slide && (all || Some(&s.scene) == scene.as_ref())).map(|(_, s)| s.id.clone()).collect();
    }

    /// Every object on the layer being worked on.
    fn select_all_objects(&mut self) {
        let Some(p) = &self.project else { return };
        let li = self.layer.min(p.slides[self.slide].layers.len().saturating_sub(1));
        let objects = &p.slides[self.slide].layers[li].objects;
        let Some(first) = objects.iter().position(|o| !o.hidden || self.show_hidden) else { return };
        let ids: Vec<String> = objects.iter().enumerate().filter(|(k, o)| *k != first && (!o.hidden || self.show_hidden)).map(|(_, o)| o.id.clone()).collect();
        self.selected = Some((li, first));
        self.multi = ids.into_iter().collect();
    }

    /// Show or hide a panel, and remember it. A panel that is shown comes back where it was.
    pub(crate) fn set_panel_shown(&mut self, g: usize, on: bool) {
        let (key, flag) = match g {
            0 => ("show_properties", &mut self.panels.properties), 1 => ("show_triggers", &mut self.panels.triggers), 2 => ("show_ai", &mut self.panels.ai),
            crate::P_TIMELINE => ("show_timeline", &mut self.panels.timeline), crate::P_NOTES => ("show_notes", &mut self.panels.notes), crate::P_STATES => ("show_states", &mut self.panels.states),
            dock::LAYERS => ("show_layers", &mut self.panels.layers), crate::P_SLIDES => ("show_slides", &mut self.panels.slides),
            _ => return,
        };
        *flag = on;
        crate::prefs::set_bool(key, on);
        if on { self.show_panel_tab(g); }
    }
}
