//! The Marker gallery: round buttons with an icon (or a number) in them. A marker is an
//! ordinary object with button states (Normal, Hover, Down, Visited, ...), so any trigger can be put on it.
use super::*;
use crate::icons::{self, Icon};

/// The gallery's sections, with the Phosphor icons each offers (names as in `assets/icons/names.txt`).
pub const CATEGORIES: [(&str, &str); 9] = [
    ("Navigation", "arrow-left arrow-right arrow-up arrow-down arrow-up-left arrow-up-right arrow-down-left arrow-down-right arrow-circle-left arrow-circle-right arrow-circle-up arrow-circle-down arrow-u-up-left arrow-u-up-right arrow-clockwise arrow-counter-clockwise caret-left caret-right caret-up caret-down caret-double-left caret-double-right caret-double-up caret-double-down caret-circle-left caret-circle-right caret-circle-up caret-circle-down arrows-out arrows-in house trash power list list-checks clock play triangle cursor cursor-click git-fork anchor compass"),
    ("Help", "info question warning lifebuoy push-pin map-pin flag flag-banner map-trifold compass signpost"),
    ("Office", "file file-text notepad clipboard-text note table tray folder folder-open floppy-disk archive notebook book book-open books calendar-check newspaper pencil pencil-simple timer gavel briefcase coffee calculator"),
    ("Media", "monitor laptop television presentation film-strip youtube-logo image camera video-camera device-mobile phone phone-call speaker-high speaker-low megaphone rewind fast-forward pause play stop record"),
    ("Tools", "wrench gear gear-six link paperclip lightbulb magnifying-glass key lock lock-open toolbox hammer"),
    ("Feedback", "check check-square check-circle x x-circle x-square minus-circle prohibit thumbs-up thumbs-down plus minus star heart seal-check"),
    ("Social", "user users users-three chat-text chat-circle chats cloud quotes x-logo facebook-logo twitter-logo"),
    ("Industry", "car truck bus airplane shield shopping-cart tag credit-card currency-circle-dollar first-aid-kit pill bandaids asterisk wheelchair thermometer radioactive lightning buildings globe"),
    ("Charts", "chart-bar chart-line chart-pie chart-line-up presentation-chart pulse trend-up"),
];

/// How many numbered markers the gallery offers (1 to this).
pub const NUMBERS: usize = 20;

/// The icons of a section, in the order listed.
pub fn icons_of(category: usize) -> Vec<&'static Icon> {
    CATEGORIES[category].1.split(' ').filter_map(|n| icons::all().iter().find(|i| i.names.contains(&n))).collect()
}

fn area(poly: &[(f32, f32)]) -> f32 {
    (0..poly.len()).map(|i| { let (a, b) = (poly[i], poly[(i + 1) % poly.len()]); a.0 * b.1 - b.0 * a.1 }).sum::<f32>() / 2.0
}

fn inside(p: (f32, f32), poly: &[(f32, f32)]) -> bool {
    let mut yes = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 { yes = !yes; }
    }
    yes
}

/// One closed figure's commands run backwards (so it is wound the other way round and draws the same outline).
fn reversed(fig: &[PathCmd]) -> Vec<PathCmd> {
    // The points the figure passes through, and what leads to each.
    let mut pts: Vec<(f32, f32)> = vec![];
    let mut segs: Vec<PathCmd> = vec![];
    for c in fig {
        match *c {
            PathCmd::Move(x, y) => pts.push((x, y)),
            PathCmd::Line(x, y) => { pts.push((x, y)); segs.push(*c); }
            PathCmd::Curve(_, _, _, _, x, y) => { pts.push((x, y)); segs.push(*c); }
            PathCmd::Close => {}
        }
    }
    let Some(&last) = pts.last() else { return vec![] };
    let mut out = vec![PathCmd::Move(last.0, last.1)];
    for k in (0..segs.len()).rev() {
        let back = pts[k];
        out.push(match segs[k] { PathCmd::Curve(a, b, c, d, _, _) => PathCmd::Curve(c, d, a, b, back.0, back.1), _ => PathCmd::Line(back.0, back.1) });
    }
    out.push(PathCmd::Close);
    out
}

/// A disc in the 0..1 frame with the icon cut out of it, so the slide shows through the icon (white on a white slide).
/// `inner` is the icon's share of the disc's width. A glyph's figures can be wound either way inside its holes (an arrow
/// inside a ring is wound against the ring's inside), so each figure is first wound by how deeply it is nested: outlines
/// one way, holes the other. The disc then goes the opposite way to the outlines and cancels exactly where the icon is.
pub fn disc_with_icon(icon: &Icon, inner: f32) -> Option<Vec<PathCmd>> {
    let (cmds, aspect) = icons::outline(icon)?;
    let (w, h) = if aspect >= 1.0 { (inner, inner / aspect) } else { (inner * aspect, inner) };
    let (x0, y0) = (0.5 - w / 2.0, 0.5 - h / 2.0);
    let moved: Vec<PathCmd> = cmds.iter().map(|c| match *c {
        PathCmd::Move(x, y) => PathCmd::Move(x0 + x * w, y0 + y * h),
        PathCmd::Line(x, y) => PathCmd::Line(x0 + x * w, y0 + y * h),
        PathCmd::Curve(a, b, c, d, e, f) => PathCmd::Curve(x0 + a * w, y0 + b * h, x0 + c * w, y0 + d * h, x0 + e * w, y0 + f * h),
        PathCmd::Close => PathCmd::Close,
    }).collect();
    let mut figures: Vec<Vec<PathCmd>> = vec![];
    for c in moved {
        match c {
            PathCmd::Move(..) => figures.push(vec![c]),
            _ => { if let Some(f) = figures.last_mut() { f.push(c); } }
        }
    }
    let polys: Vec<Vec<(f32, f32)>> = figures.iter().map(|f| f.iter().filter_map(|c| match *c { PathCmd::Move(x, y) | PathCmd::Line(x, y) | PathCmd::Curve(_, _, _, _, x, y) => Some((x, y)), PathCmd::Close => None }).collect()).collect();
    let mut icon_cmds = vec![];
    for (i, fig) in figures.iter().enumerate() {
        let depth = (0..figures.len()).filter(|&j| j != i && polys[j].len() > 2 && inside(polys[i][0], &polys[j])).count();
        // Outlines wound with a positive area, holes with a negative one.
        let want_positive = depth % 2 == 0;
        if (area(&polys[i]) > 0.0) == want_positive { icon_cmds.extend(fig.iter().copied()); } else { icon_cmds.extend(reversed(fig)); }
    }
    // Increasing angle is a positive area; the disc goes the other way round.
    let (c, r, k) = ((0.5_f32, 0.5_f32), 0.5_f32, 0.5523_f32);
    let at = |a: f32| (c.0 + r * a.cos(), c.1 + r * a.sin());
    let mut disc = vec![PathCmd::Move(at(0.0).0, at(0.0).1)];
    for q in 0..4 {
        let (a, b) = (-(q as f32) * std::f32::consts::FRAC_PI_2, -((q + 1) as f32) * std::f32::consts::FRAC_PI_2);
        let (p0, p3) = (at(a), at(b));
        let (t0, t3) = ((a.sin(), -a.cos()), (b.sin(), -b.cos()));
        disc.push(PathCmd::Curve(p0.0 + k * r * t0.0, p0.1 + k * r * t0.1, p3.0 - k * r * t3.0, p3.1 - k * r * t3.1, p3.0, p3.1));
    }
    disc.push(PathCmd::Close);
    disc.extend(icon_cmds);
    Some(disc)
}

/// The face of a marker: an icon from the gallery, or a number.
#[derive(Clone, Copy)]
pub enum Face { Icon(&'static Icon), Number(usize) }

impl App {
    /// Insert a marker: a round button (the theme's accent color, lighter on hover and darker pressed) with the icon
    /// cut out of it, or the number written in it. Triggers are put on it like on any button.
    pub(super) fn insert_marker(&mut self, face: Face) {
        let accent = self.project.as_ref().map(|p| p.theme.color(4)).unwrap_or(Rgba(3, 83, 158, 255));
        let look = button_styles::StyleDef { name: "Marker", fill: accent, text: Rgba(255, 255, 255, 255), look: button_styles::Look::Plain, radius: 0.0, hue: None };
        let size = 120.0;
        let mut o = match face {
            Face::Icon(icon) => {
                let Some(path) = disc_with_icon(icon, 0.56) else { return };
                let mut o = blank_object(self.new_id("obj"), &format!("Marker: {}", icon.title()), Kind::Path(path), 0.0, 0.0, size, size);
                o.alt = icon.title();
                o
            }
            Face::Number(n) => {
                let mut o = blank_object(self.new_id("obj"), &format!("Marker {n}"), Kind::Ellipse, 0.0, 0.0, size, size);
                o.text = n.to_string();
                o.style = plain_style(if n >= 10 { 44.0 } else { 52.0 }, Rgba(255, 255, 255, 255), true, Align::Center);
                o.alt = format!("Marker {n}");
                o
            }
        };
        button_styles::apply(&mut o, &look);
        o.marker = Some(MarkerInfo::new());
        // Flat: the button style's soft shadow is for rectangular buttons.
        o.shadow = None;
        for st in o.states.iter_mut() { st.shadow = None; }
        o.radius = 0.0;
        o.fill = Some(accent);
        self.insert(o);
    }

    /// The Marker gallery: every section's markers as round buttons, ten to a row.
    pub(super) fn marker_gallery(&mut self, ui: &mut egui::Ui) {
        const COLS: usize = 10;
        const CELL: f32 = 40.0;
        let accent = self.project.as_ref().map(|p| p.theme.color(4)).unwrap_or(Rgba(3, 83, 158, 255));
        ui.set_width(COLS as f32 * (CELL + 4.0) + 8.0);
        let mut pick: Option<Face> = None;
        let disc = |ui: &mut egui::Ui, label: &str, draw: &dyn Fn(&egui::Painter, egui::Rect)| -> bool {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::click());
            if resp.hovered() { ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(210, 226, 245)); }
            let r = rect.shrink(3.0);
            ui.painter().circle_filled(r.center(), r.width() / 2.0, color(accent));
            draw(ui.painter(), r);
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
            resp.on_hover_text(label).clicked()
        };
        egui::ScrollArea::vertical().max_height(420.0).auto_shrink([false, true]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            for (ci, (name, _)) in CATEGORIES.iter().enumerate() {
                ui.label(egui::RichText::new(*name).weak().size(12.0));
                ui.horizontal_wrapped(|ui| {
                    for icon in icons_of(ci) {
                        if disc(ui, &icon.title(), &|p, r| { p.text(r.center(), egui::Align2::CENTER_CENTER, icon.ch.to_string(), egui::FontId::new(r.width() * 0.56, ribbon::icons()), egui::Color32::WHITE); }) { pick = Some(Face::Icon(icon)); }
                    }
                });
                if ci == 2 {
                    ui.label(egui::RichText::new("Numbers").weak().size(12.0));
                    ui.horizontal_wrapped(|ui| {
                        for n in 1..=NUMBERS {
                            if disc(ui, &format!("Marker {n}"), &|p, r| { p.text(r.center(), egui::Align2::CENTER_CENTER, n.to_string(), egui::FontId::proportional(r.width() * if n >= 10 { 0.42 } else { 0.5 }), egui::Color32::WHITE); }) { pick = Some(Face::Number(n)); }
                        }
                    });
                }
            }
        });
        if let Some(face) = pick { self.insert_marker(face); ui.close(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_icon_exists_and_is_cut_out_of_its_disc() {
        use i_overlay::core::fill_rule::FillRule;
        use i_overlay::float::simplify::SimplifyShape;
        let frame = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 1000.0));
        let poly_area = |f: &[egui::Pos2]| -> f32 { (0..f.len()).map(|i| { let (a, b) = (f[i], f[(i + 1) % f.len()]); a.x * b.y - b.x * a.y }).sum::<f32>().abs() / 2.0 };
        let region = |cmds: &[PathCmd]| -> f32 {
            let contours: Vec<Vec<[f32; 2]>> = path_polygons(cmds, frame).iter().filter(|f| f.len() > 2).map(|f| f.iter().map(|p| [p.x, p.y]).collect()).collect();
            contours.simplify_shape(FillRule::NonZero).iter().map(|sh| sh.iter().enumerate().map(|(k, c)| { let a = poly_area(&c.iter().map(|p| egui::pos2(p[0], p[1])).collect::<Vec<_>>()); if k == 0 { a } else { -a } }).sum::<f32>()).sum()
        };
        let mut bad = vec![];
        for (ci, (name, list)) in CATEGORIES.iter().enumerate() {
            let found = icons_of(ci);
            assert_eq!(found.len(), list.split(' ').count(), "{name}: every name is a real icon");
            for icon in found {
                let path = disc_with_icon(icon, 0.56).unwrap_or_else(|| panic!("{} has an outline", icon.names[0]));
                let (plain, _) = icons::outline(icon).unwrap();
                // The icon's own filled area when drawn at the size it has in the disc.
                let aspect = icons::outline(icon).unwrap().1;
                let (w, h) = if aspect >= 1.0 { (0.56, 0.56 / aspect) } else { (0.56 * aspect, 0.56) };
                let icon_area = region(&plain) * w * h;
                // The disc takes the icon's area out of itself: what is left is the disc minus the icon.
                let left = region(&path);
                let expect = std::f32::consts::PI * 0.25 * 1_000_000.0 - icon_area;
                if (left - expect).abs() > expect * 0.03 { bad.push((icon.names[0], left as i32, expect as i32)); }
            }
        }
        assert!(bad.is_empty(), "{} markers do not have the icon cut out cleanly: {:?}", bad.len(), &bad[..bad.len().min(6)]);
    }
}
