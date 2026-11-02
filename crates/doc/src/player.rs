//! How the published player looks and what it offers: its colours, font, sidebar, title, logo and controls.
//!
//! The defaults reproduce the player as it was before these settings existed (a dark bar along the bottom and a slide
//! menu that opens over the slide), so projects saved earlier look the same until something is changed.
use crate::Rgba;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerTheme {
    #[default]
    Dark,
    Light,
    /// The author's own background colour; text and lines follow from it.
    Custom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidebarSide {
    /// No sidebar: the slide menu opens over the slide from a Menu button.
    #[default]
    Off,
    Left,
    Right,
}

/// How the player's buttons are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonStyle {
    Icon,
    IconText,
    #[default]
    Text,
}

/// Player controls beyond Previous / Next, each shown only when asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerControls {
    /// Mute and unmute the slide's video and audio.
    pub volume: bool,
    /// Turn captions on and off.
    pub captions: bool,
    /// Step through playback speeds for video and audio.
    pub speed: bool,
    pub fullscreen: bool,
}

/// A link or an attached file offered in the player's Notes tab.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub title: String,
    /// A web address; empty when a file is attached.
    #[serde(default)]
    pub url: String,
    /// An attached file (asset key).
    #[serde(default)]
    pub asset: Option<String>,
}

/// A term and what it means, listed (alphabetically) in the player's Notes tab.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GlossaryTerm {
    pub term: String,
    pub definition: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerSettings {
    pub theme: PlayerTheme,
    /// The player's background when the theme is Custom.
    pub background: Rgba,
    /// Buttons, the progress bar and the current slide in the menu.
    pub accent: Rgba,
    pub font: String,
    /// Percent of the usual text size.
    pub font_scale: u32,
    pub buttons: ButtonStyle,
    /// Show the title in a bar along the top.
    pub show_title: bool,
    /// The title; empty means the project's name.
    pub title: String,
    pub sidebar: SidebarSide,
    pub sidebar_collapsed: bool,
    /// A Notes tab in the sidebar: the current slide's notes.
    pub tab_notes: bool,
    /// A Glossary tab in the sidebar, listing the project's terms.
    #[serde(default)]
    pub tab_glossary: bool,
    /// A Resources link at the right of the top bar, opening the project's links and files.
    #[serde(default)]
    pub tab_resources: bool,
    /// A search box over the sidebar's slide list.
    #[serde(default)]
    pub search: bool,
    /// A picture (asset key) above the sidebar's navigation, or in the top bar when there is no sidebar.
    pub logo: Option<String>,
    pub controls: PlayerControls,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        PlayerSettings {
            theme: PlayerTheme::Dark,
            background: Rgba(27, 31, 38, 255),
            accent: Rgba(3, 83, 158, 255),
            font: "Open Sans".into(),
            font_scale: 100,
            buttons: ButtonStyle::Text,
            show_title: false,
            title: String::new(),
            sidebar: SidebarSide::Off,
            sidebar_collapsed: false,
            tab_notes: false,
            tab_glossary: false,
            tab_resources: false,
            search: false,
            logo: None,
            controls: PlayerControls::default(),
        }
    }
}

/// The colours the player is drawn with, worked out from the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerPalette {
    /// Behind the slide.
    pub body: Rgba,
    /// Bars and the sidebar.
    pub panel: Rgba,
    pub fg: Rgba,
    pub muted: Rgba,
    pub hover: Rgba,
    pub line: Rgba,
    pub disabled: Rgba,
    pub accent: Rgba,
    /// Text on the accent.
    pub accent_ink: Rgba,
    /// The progress bar's fill: the accent, lifted when it would be hard to see on the bar.
    pub progress: Rgba,
}

fn luma(c: Rgba) -> f32 {
    (0.299 * c.0 as f32 + 0.587 * c.1 as f32 + 0.114 * c.2 as f32) / 255.0
}

fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    Rgba(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2), 255)
}

const DEFAULT_ACCENT: Rgba = Rgba(3, 83, 158, 255);

impl PlayerSettings {
    pub fn palette(&self) -> PlayerPalette {
        let accent = Rgba(self.accent.0, self.accent.1, self.accent.2, 255);
        // The original accent is a deep blue; its progress fill was always a brighter one.
        let progress = if accent == DEFAULT_ACCENT { Rgba(74, 163, 255, 255) } else { accent };
        let accent_ink = if luma(accent) > 0.62 { Rgba(20, 24, 30, 255) } else { Rgba(255, 255, 255, 255) };
        match self.theme {
            PlayerTheme::Dark => PlayerPalette {
                body: Rgba(17, 17, 17, 255),
                panel: Rgba(27, 31, 38, 255),
                fg: Rgba(255, 255, 255, 255),
                muted: Rgba(154, 164, 178, 255),
                hover: Rgba(42, 49, 60, 255),
                line: Rgba(58, 66, 80, 255),
                disabled: Rgba(74, 82, 96, 255),
                accent,
                accent_ink,
                progress,
            },
            PlayerTheme::Light => PlayerPalette {
                body: Rgba(228, 232, 238, 255),
                panel: Rgba(244, 246, 249, 255),
                fg: Rgba(27, 42, 58, 255),
                muted: Rgba(91, 102, 117, 255),
                hover: Rgba(230, 235, 242, 255),
                line: Rgba(205, 213, 223, 255),
                disabled: Rgba(170, 179, 192, 255),
                accent,
                accent_ink,
                progress,
            },
            PlayerTheme::Custom => {
                let panel = Rgba(self.background.0, self.background.1, self.background.2, 255);
                let dark = luma(panel) < 0.5;
                let fg = if dark { Rgba(255, 255, 255, 255) } else { Rgba(20, 24, 30, 255) };
                PlayerPalette {
                    body: mix(panel, if dark { Rgba(0, 0, 0, 255) } else { Rgba(0, 0, 0, 255) }, 0.35),
                    panel,
                    fg,
                    muted: mix(panel, fg, 0.62),
                    hover: mix(panel, fg, 0.12),
                    line: mix(panel, fg, 0.24),
                    disabled: mix(panel, fg, 0.38),
                    accent,
                    accent_ink,
                    progress,
                }
            }
        }
    }

    /// The title the player shows: the author's, else the project's name.
    pub fn shown_title<'a>(&'a self, project_name: &'a str) -> &'a str {
        if self.title.trim().is_empty() { project_name } else { &self.title }
    }
}

impl PlayerPalette {
    /// The palette as CSS custom properties, set on `:root` of the published page.
    pub fn css_vars(&self) -> String {
        let c = |x: Rgba| format!("#{:02x}{:02x}{:02x}", x.0, x.1, x.2);
        format!(
            "--p-body:{};--p-panel:{};--p-fg:{};--p-muted:{};--p-hover:{};--p-line:{};--p-disabled:{};--p-accent:{};--p-accent-ink:{};--p-progress:{}",
            c(self.body), c(self.panel), c(self.fg), c(self.muted), c(self.hover), c(self.line), c(self.disabled), c(self.accent), c(self.accent_ink), c(self.progress)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_palette_is_the_player_as_it_was() {
        let p = PlayerSettings::default().palette();
        assert_eq!((p.panel, p.body, p.accent), (Rgba(27, 31, 38, 255), Rgba(17, 17, 17, 255), Rgba(3, 83, 158, 255)));
        assert_eq!(p.accent_ink, Rgba(255, 255, 255, 255));
    }

    #[test]
    fn a_custom_background_picks_readable_text_and_lines() {
        let light = PlayerSettings { theme: PlayerTheme::Custom, background: Rgba(250, 245, 230, 255), ..Default::default() }.palette();
        assert!(luma(light.fg) < 0.2, "dark text on a light background");
        let dark = PlayerSettings { theme: PlayerTheme::Custom, background: Rgba(10, 30, 60, 255), ..Default::default() }.palette();
        assert!(luma(dark.fg) > 0.9, "light text on a dark background");
        assert!(luma(dark.line) > luma(dark.panel), "lines show against the panel");
    }

    #[test]
    fn a_light_accent_gets_dark_text() {
        let p = PlayerSettings { accent: Rgba(250, 220, 60, 255), ..Default::default() }.palette();
        assert_eq!(p.accent_ink, Rgba(20, 24, 30, 255));
    }

    #[test]
    fn settings_round_trip_and_older_projects_get_the_defaults() {
        let s = PlayerSettings { sidebar: SidebarSide::Left, logo: Some("k".into()), ..Default::default() };
        let back: PlayerSettings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }
}
