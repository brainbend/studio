//! The menu bar at the top of the screen on macOS, drawn from the same list of commands as the menu bar inside the window.
//!
//! The menus are made once, when the app starts. A click comes back as an event, and runs the command as any other way of asking for it
//! does; each frame the lines are greyed out or ticked by the commands' own rules. Four lines are the system's own (Hide, Hide Others,
//! Show All and Services), and the Window menu's Minimize and Zoom.
//!
//! Keys: a menu line that has a key takes the key from the window before the app sees it. So the lines for typing (Cut, Copy, Paste, Undo,
//! Select All and the rest of Edit) have no key here; the app answers those keys itself, which keeps them working inside a text box.
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};

use eframe::egui;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};

use crate::commands::{self, Cmd, Item};

/// What a line of the menu does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Action {
    Cmd(Cmd),
    /// Add the object at this place of the Insert menu's list.
    Insert(usize),
}

/// One line of the menu, as the app sees it, before it is a native one. (Separate from the native menu so it can be tested.)
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Line {
    Do(Cmd),
    Insert(usize, &'static str),
    Gap,
    Sub(&'static str, Vec<Cmd>),
}

/// The menus: the app's, then those of `commands::menus()` (File, Edit, View, Insert, Tools, Window, Help). The Insert menu lists the objects
/// that are added with one click; the ones with choices (a table's size, a shape) are on the insert bar.
pub(crate) fn lines() -> Vec<(&'static str, Vec<Line>)> {
    let mut out = vec![];
    for (name, items) in commands::menus() {
        let mut v = vec![];
        for item in items {
            match item {
                Item::Do(c) => v.push(Line::Do(c)),
                Item::Gap => v.push(Line::Gap),
                Item::Sub(n, cmds) => v.push(Line::Sub(n, cmds)),
                Item::Gallery => {
                    for (i, label) in crate::App::insert_leaf_labels().into_iter().enumerate() { v.push(Line::Insert(i, label)); }
                }
            }
        }
        out.push((name, v));
    }
    out
}

/// The key a line has in the menu, for the lines that are not about typing.
fn accelerator(c: Cmd) -> Option<Accelerator> {
    let cmd = Modifiers::META;
    Some(match c {
        Cmd::New => Accelerator::new(cmd, Code::KeyN),
        Cmd::Open => Accelerator::new(cmd, Code::KeyO),
        Cmd::Save => Accelerator::new(cmd, Code::KeyS),
        Cmd::SaveAs => Accelerator::new(cmd | Modifiers::SHIFT, Code::KeyS),
        Cmd::Settings => Accelerator::new(cmd, Code::Comma),
        Cmd::Quit => Accelerator::new(cmd, Code::KeyQ),
        _ => return None,
    })
}

/// Whether a command is a switch (shown with a tick).
fn is_switch(c: Cmd) -> bool {
    matches!(c, Cmd::TogglePanel(_) | Cmd::ShowMenuBar | Cmd::ShowInsertBar | Cmd::DarkMode | Cmd::SlideSize(..))
}

enum Native { Plain(MenuItem), Switch(CheckMenuItem) }

struct Entry { action: Action, item: Native, enabled: bool, checked: Option<bool> }

pub(crate) struct NativeMenu {
    _menu: Menu,
    entries: Vec<Entry>,
    ids: HashMap<MenuId, Action>,
    rx: Receiver<MenuEvent>,
}

impl NativeMenu {
    /// Make the menus and put them at the top of the screen. Must run on the main thread, once the app exists.
    pub(crate) fn install(ctx: &egui::Context) -> Option<NativeMenu> {
        let (tx, rx) = channel();
        let repaint = ctx.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| { let _ = tx.send(e); repaint.request_repaint(); }));
        let menu = Menu::new();
        let mut entries: Vec<Entry> = vec![];
        let mut ids = HashMap::new();

        // The app's own menu: About, Settings, the system's lines, Quit.
        let app = Submenu::new("BrainBend Studio", true);
        let mut add = |parent: &Submenu, action: Action, label: String, accel: Option<Accelerator>, entries: &mut Vec<Entry>, ids: &mut HashMap<MenuId, Action>| {
            let (item, id) = match action {
                Action::Cmd(c) if is_switch(c) => { let it = CheckMenuItem::new(label, true, false, accel); let id = it.id().clone(); let _ = parent.append(&it); (Native::Switch(it), id) }
                _ => { let it = MenuItem::new(label, true, accel); let id = it.id().clone(); let _ = parent.append(&it); (Native::Plain(it), id) }
            };
            ids.insert(id, action);
            entries.push(Entry { action, item, enabled: true, checked: None });
        };
        add(&app, Action::Cmd(Cmd::About), Cmd::About.label(), None, &mut entries, &mut ids);
        let _ = app.append(&PredefinedMenuItem::separator());
        add(&app, Action::Cmd(Cmd::Settings), "Settings…".into(), accelerator(Cmd::Settings), &mut entries, &mut ids);
        add(&app, Action::Cmd(Cmd::CheckForUpdates), Cmd::CheckForUpdates.label(), None, &mut entries, &mut ids);
        let _ = app.append(&PredefinedMenuItem::separator());
        let _ = app.append(&PredefinedMenuItem::services(None));
        let _ = app.append(&PredefinedMenuItem::separator());
        let _ = app.append(&PredefinedMenuItem::hide(None));
        let _ = app.append(&PredefinedMenuItem::hide_others(None));
        let _ = app.append(&PredefinedMenuItem::show_all(None));
        let _ = app.append(&PredefinedMenuItem::separator());
        add(&app, Action::Cmd(Cmd::Quit), Cmd::Quit.label(), accelerator(Cmd::Quit), &mut entries, &mut ids);
        let _ = menu.append(&app);

        for (name, lines) in lines() {
            let sub = Submenu::new(name, true);
            for line in lines {
                match line {
                    Line::Gap => { let _ = sub.append(&PredefinedMenuItem::separator()); }
                    // The app's own menu has these.
                    Line::Do(Cmd::Settings | Cmd::Quit | Cmd::CheckForUpdates | Cmd::About) => {}
                    Line::Do(c) => add(&sub, Action::Cmd(c), c.label(), accelerator(c), &mut entries, &mut ids),
                    Line::Insert(i, label) => add(&sub, Action::Insert(i), label.to_string(), None, &mut entries, &mut ids),
                    Line::Sub(n, cmds) => {
                        let inner = Submenu::new(n, true);
                        for c in cmds { add(&inner, Action::Cmd(c), c.label(), None, &mut entries, &mut ids); }
                        let _ = sub.append(&inner);
                    }
                }
            }
            if name == "Window" {
                let _ = sub.append(&PredefinedMenuItem::separator());
                let _ = sub.append(&PredefinedMenuItem::minimize(None));
                let _ = sub.append(&PredefinedMenuItem::maximize(None));
                let _ = sub.append(&PredefinedMenuItem::bring_all_to_front(None));
            }
            let _ = menu.append(&sub);
            if name == "Window" { sub.set_as_windows_menu_for_nsapp(); }
            if name == "Help" { sub.set_as_help_menu_for_nsapp(); }
        }
        menu.init_for_nsapp();
        Some(NativeMenu { _menu: menu, entries, ids, rx })
    }

    /// The lines that were clicked since the last frame.
    pub(crate) fn clicked(&self) -> Vec<Action> {
        self.rx.try_iter().filter_map(|e| self.ids.get(&e.id).copied()).collect()
    }

    /// Grey out or tick the lines as the commands say. Only what changed is told to the system.
    pub(crate) fn refresh(&mut self, app: &crate::App) {
        for e in self.entries.iter_mut() {
            let (enabled, checked) = match e.action {
                Action::Cmd(c) => (app.cmd_enabled(c), app.cmd_checked(c)),
                Action::Insert(_) => (app.project.is_some() && !app.try_mode, None),
            };
            if enabled != e.enabled {
                match &e.item { Native::Plain(i) => i.set_enabled(enabled), Native::Switch(i) => i.set_enabled(enabled) }
                e.enabled = enabled;
            }
            if checked != e.checked {
                if let (Native::Switch(i), Some(on)) = (&e.item, checked) { i.set_checked(on); }
                e.checked = checked;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_native_menus_are_the_commands_menus_with_the_insert_menu_listing_what_one_click_adds() {
        let l = lines();
        let names: Vec<&str> = l.iter().map(|m| m.0).collect();
        assert_eq!(names, ["File", "Edit", "View", "Insert", "Tools", "Window", "Help"]);
        let insert = &l.iter().find(|m| m.0 == "Insert").unwrap().1;
        assert!(insert.iter().all(|x| matches!(x, Line::Insert(..))) && insert.len() >= 6, "the objects that are added by one click");
        let edit = &l.iter().find(|m| m.0 == "Edit").unwrap().1;
        assert!(edit.contains(&Line::Do(Cmd::Copy)) && accelerator(Cmd::Copy).is_none() && accelerator(Cmd::Paste).is_none(), "typing keys are not taken by the menu");
        assert!(accelerator(Cmd::Save).is_some() && accelerator(Cmd::Quit).is_some() && accelerator(Cmd::Settings).is_some());
        assert!(is_switch(Cmd::TogglePanel(0)) && !is_switch(Cmd::Save));
    }
}
