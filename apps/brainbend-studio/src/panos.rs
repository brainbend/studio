//! 360° images in the editor: inserting one, showing it on the slide, and the window where its view and annotations
//! are set. The viewing maths is in `brainbend_doc::pano`; the player does the same in a shader.
use super::*;
use std::sync::Arc;
use brainbend_doc::pano::{self, Pano, PanoPin};
pub use brainbend_doc::pano::PinKind;

/// The widest a panorama is kept for the editor's previews. Bigger pictures are shrunk once, here only.
const PREVIEW_WIDTH: u32 = 2048;

/// A panorama decoded for previews: equirectangular RGB.
pub struct Source {
    w: usize,
    h: usize,
    rgb: Vec<u8>,
}

impl Source {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut img = image::load_from_memory(bytes).ok()?;
        if img.width() > PREVIEW_WIDTH {
            let h = (img.height() as f32 * PREVIEW_WIDTH as f32 / img.width() as f32).round().max(1.0) as u32;
            img = img.resize_exact(PREVIEW_WIDTH, h, image::imageops::FilterType::Triangle);
        }
        let rgb = img.to_rgb8();
        Some(Self { w: rgb.width() as usize, h: rgb.height() as usize, rgb: rgb.into_raw() })
    }

    /// Bilinear colour at picture coordinates `(u, v)` in 0..1; the picture wraps sideways.
    fn sample(&self, u: f32, v: f32) -> [f32; 3] {
        let x = u * self.w as f32 - 0.5;
        let y = (v * self.h as f32 - 0.5).clamp(0.0, (self.h - 1) as f32);
        let x0 = x.floor();
        let (fx, fy) = (x - x0, y - y.floor());
        let xa = (x0 as i64).rem_euclid(self.w as i64) as usize;
        let xb = (xa + 1) % self.w;
        let ya = y.floor() as usize;
        let yb = (ya + 1).min(self.h - 1);
        let px = |x: usize, y: usize| { let i = (y * self.w + x) * 3; [self.rgb[i] as f32, self.rgb[i + 1] as f32, self.rgb[i + 2] as f32] };
        let (a, b, c, d) = (px(xa, ya), px(xb, ya), px(xa, yb), px(xb, yb));
        std::array::from_fn(|k| (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy)
    }

    /// The view `(yaw, pitch, fov)` seen through a `w` by `h` window.
    pub fn view(&self, view: (f32, f32, f32), w: usize, h: usize) -> egui::ColorImage {
        let (f, r, u) = pano::camera(view.0, view.1);
        let t = (view.2.to_radians() / 2.0).tan();
        let aspect = w as f32 / h as f32;
        let mut rgb = Vec::with_capacity(w * h * 3);
        for j in 0..h {
            let y = (1.0 - 2.0 * (j as f32 + 0.5) / h as f32) * t / aspect;
            for i in 0..w {
                let x = (2.0 * (i as f32 + 0.5) / w as f32 - 1.0) * t;
                let d: [f32; 3] = std::array::from_fn(|k| f[k] + r[k] * x + u[k] * y);
                let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                let lon = d[0].atan2(d[2]);
                let lat = (d[1] / len).clamp(-1.0, 1.0).asin();
                let c = self.sample(0.5 + lon / std::f32::consts::TAU, 0.5 - lat / std::f32::consts::PI);
                rgb.extend(c.iter().map(|v| v.round().clamp(0.0, 255.0) as u8));
            }
        }
        egui::ColorImage::from_rgb([w, h], &rgb)
    }
}

/// The editor's 360° state: the open editing mode and the decoded pictures and rendered views, kept between frames.
#[derive(Default)]
pub struct State {
    pub edit: Option<Edit>,
    sources: HashMap<String, Option<Arc<Source>>>,
    pub(super) views: HashMap<String, ((i32, i32, i32, usize, usize), egui::TextureHandle)>,
}

/// What the pointer is doing on the picture.
#[derive(Clone, Debug, PartialEq)]
enum Drag {
    Look,
    Item(usize),
    Corner(usize, usize),
}

/// The tool in use: pointing, or drawing a hotspot a corner at a time.
#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Draw(Vec<(f32, f32)>),
}

/// The 360° editing mode: which object, the view being looked through, what is selected.
pub struct Edit {
    pub target: (usize, usize),
    pub slide: usize,
    asset: String,
    pub view: (f32, f32, f32),
    pub selected: Option<usize>,
    pub tool: Tool,
    drag: Option<Drag>,
    /// Whether anything was changed (an untouched visit leaves no undo step).
    pub changed: bool,
}

/// Things the ribbon asks the editing mode to do.
pub enum Cmd {
    AddMarker,
    StartHotspot,
    SetIcon(Option<&'static crate::icons::Icon>),
    SetColor(Option<Rgba>),
    AddTrigger,
    SwitchType,
    Delete,
    SetInitialView,
    Close,
}

/// Names of the icons a marker can wear (all in the bundled icon font).
pub const MARKER_ICONS: [&str; 18] = [
    "plus", "arrow-right", "arrow-up", "arrow-down", "arrow-left", "check", "x", "info", "question", "heart", "thumbs-up",
    "magnifying-glass", "link", "play", "star", "lightbulb", "flag", "map-pin",
];

const MARKER_R: f32 = 20.0;

/// The character that draws an icon in the icon font.
pub fn icon_char(name: &str) -> Option<char> {
    crate::icons::all().iter().find(|i| i.names.contains(&name)).map(|i| i.ch)
}

/// An icon's outline fitted into the middle of a 0..1 box, `inner` of its width: what a marker's icon is stored as.
pub fn icon_in_box(icon: &crate::icons::Icon, inner: f32) -> Vec<PathCmd> {
    let Some((cmds, aspect)) = crate::icons::outline(icon) else { return vec![] };
    let (w, h) = if aspect >= 1.0 { (inner, inner / aspect) } else { (inner * aspect, inner) };
    let (x0, y0) = (0.5 - w / 2.0, 0.5 - h / 2.0);
    cmds.iter().map(|c| match *c {
        PathCmd::Move(x, y) => PathCmd::Move(x0 + x * w, y0 + y * h),
        PathCmd::Line(x, y) => PathCmd::Line(x0 + x * w, y0 + y * h),
        PathCmd::Curve(a, b, c, d, e, f) => PathCmd::Curve(x0 + a * w, y0 + b * h, x0 + c * w, y0 + d * h, x0 + e * w, y0 + f * h),
        PathCmd::Close => PathCmd::Close,
    }).collect()
}

/// A marker or hotspot's name in the list: its title, else "Marker 2" / "Hotspot 1" counting items of its kind.
pub fn item_name(pins: &[PanoPin], i: usize) -> String {
    let p = &pins[i];
    if !p.title.trim().is_empty() { return p.title.trim().to_string(); }
    let n = pins[..=i].iter().filter(|q| q.kind == p.kind).count();
    format!("{} {n}", if p.kind == PinKind::Hotspot { "Hotspot" } else { "Marker" })
}

fn col(c: Rgba, a: u8) -> egui::Color32 { egui::Color32::from_rgba_unmultiplied(c.0, c.1, c.2, a) }

/// Where a marker shows in `rect` for `view`; `None` when it is behind the viewer or off the view.
fn marker_at(rect: egui::Rect, view: (f32, f32, f32), p: &PanoPin) -> Option<egui::Pos2> {
    let aspect = rect.width() / rect.height().max(1.0);
    let (x, y) = pano::project(view, aspect, p.yaw, p.pitch)?;
    (x.abs() <= 1.04 && y.abs() <= 1.04).then(|| rect.center() + egui::vec2(x * rect.width() / 2.0, -y * rect.height() / 2.0))
}

/// A hotspot's corners in `rect` for `view`; `None` when any corner is behind the viewer.
fn corners_at(rect: egui::Rect, view: (f32, f32, f32), p: &PanoPin) -> Option<Vec<egui::Pos2>> {
    let aspect = rect.width() / rect.height().max(1.0);
    p.points.iter().map(|&(yaw, pitch)| {
        let (x, y) = pano::project(view, aspect, yaw, pitch)?;
        Some(rect.center() + egui::vec2(x * rect.width() / 2.0, -y * rect.height() / 2.0))
    }).collect()
}

/// Fill a polygon (it may be concave).
fn fill_polygon(painter: &egui::Painter, pts: &[egui::Pos2], color: egui::Color32) {
    if pts.len() < 3 { return; }
    let flat: Vec<f64> = pts.iter().flat_map(|p| [p.x as f64, p.y as f64]).collect();
    let Ok(tris) = earcutr::earcut(&flat, &[], 2) else { return };
    let mut mesh = egui::epaint::Mesh::default();
    for p in pts { mesh.colored_vertex(*p, color); }
    for t in tris.chunks(3) { mesh.add_triangle(t[0] as u32, t[1] as u32, t[2] as u32); }
    painter.add(egui::Shape::mesh(mesh));
}

/// The markers and hotspots of a 360° image drawn over its view in `rect`. `r` is a marker's radius.
fn draw_items(painter: &egui::Painter, rect: egui::Rect, view: (f32, f32, f32), p: &Pano, selected: Option<usize>, r: f32) {
    let painter = painter.with_clip_rect(rect);
    for (i, it) in p.pins.iter().enumerate().filter(|(_, it)| it.kind == PinKind::Hotspot) {
        let Some(pts) = corners_at(rect, view, it) else { continue };
        let c = it.colour();
        fill_polygon(&painter, &pts, col(c, if p.reveal || selected == Some(i) { 92 } else { 22 }));
        let mut ring = pts.clone();
        ring.push(pts[0]);
        painter.add(egui::Shape::line(ring, egui::Stroke::new((r / 10.0).max(1.0), col(c, 255))));
    }
    for (i, it) in p.pins.iter().enumerate().filter(|(_, it)| it.kind == PinKind::Marker) {
        let Some(at) = marker_at(rect, view, it) else { continue };
        let c = it.colour();
        if p.pulse { painter.circle_stroke(at, r * 1.45, egui::Stroke::new(r / 5.0, col(c, 90))); }
        let on = selected == Some(i);
        painter.circle(at, r, col(c, 255), egui::Stroke::new((r / 7.0).max(1.0), if on { egui::Color32::from_rgb(255, 214, 10) } else { egui::Color32::WHITE }));
        if let Some(ch) = icon_char(&it.icon) {
            painter.text(at, egui::Align2::CENTER_CENTER, ch.to_string(), egui::FontId::new(r * 1.1, ribbon::icons()), egui::Color32::WHITE);
        }
    }
}

impl App {
    /// The decoded panorama for asset `key`, decoded once.
    fn pano_source(&mut self, key: &str) -> Option<Arc<Source>> {
        if !self.pano.sources.contains_key(key) {
            let src = self.project.as_ref().and_then(|p| p.assets.get(key)).and_then(|a| Source::decode(&a.bytes)).map(Arc::new);
            self.pano.sources.insert(key.to_string(), src);
        }
        self.pano.sources.get(key).cloned().flatten()
    }

    /// The view as a texture, rendered again only when the view or the size changes. `id` names the cache slot.
    fn pano_texture(&mut self, ctx: &egui::Context, id: &str, key: &str, view: (f32, f32, f32), w: usize, h: usize) -> Option<egui::TextureHandle> {
        let q = |v: f32| (v * 20.0).round() as i32;
        let stamp = (q(view.0), q(view.1), q(view.2), w, h);
        if let Some((s, t)) = self.pano.views.get(id) { if *s == stamp { return Some(t.clone()); } }
        let src = self.pano_source(key)?;
        let img = src.view(view, w, h);
        let tex = ctx.load_texture(format!("pano-{id}"), img, egui::TextureOptions::LINEAR);
        self.pano.views.insert(id.to_string(), (stamp, tex.clone()));
        Some(tex)
    }

    /// A panorama object on the slide: its start view, its markers and hotspots and a small "360°" mark.
    pub(super) fn paint_pano(&mut self, ctx: &egui::Context, painter: &egui::Painter, o: &Object, p: &Pano, r: egui::Rect, rot: &dyn Fn(egui::Pos2) -> egui::Pos2) {
        let view = (p.yaw, p.pitch, p.fov);
        // The thumbnail and the canvas draw the same object at different sizes, so each size bucket has a slot of its own
        // (one shared slot would be re-rendered every frame, freeing the texture the other just used).
        let w = (r.width().round().max(1.0) as usize).next_power_of_two().clamp(128, 640);
        let h = ((w as f32 * o.h / o.w.max(1.0)).round() as usize).clamp(54, 640);
        let tex = o.asset.as_deref().and_then(|k| self.pano_texture(ctx, &format!("obj-{}-{w}", o.id), k, view, w, h));
        match tex {
            Some(t) => {
                let mut mesh = egui::epaint::Mesh::with_texture(t.id());
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                for (pos, uv) in [(r.left_top(), uv.left_top()), (r.right_top(), uv.right_top()), (r.right_bottom(), uv.right_bottom()), (r.left_bottom(), uv.left_bottom())] {
                    mesh.vertices.push(egui::epaint::Vertex { pos: rot(pos), uv, color: egui::Color32::WHITE });
                }
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                painter.add(egui::Shape::mesh(mesh));
                draw_items(painter, r, view, p, None, (r.width() / 60.0).clamp(5.0, 14.0));
            }
            None => {
                painter.rect_filled(r, 4.0, egui::Color32::from_gray(60));
                painter.text(r.center(), egui::Align2::CENTER_CENTER, ribbon::PANORAMA.to_string(), egui::FontId::new((r.height().min(r.width()) * 0.3).clamp(14.0, 96.0), ribbon::icons()), egui::Color32::from_gray(170));
            }
        }
        let tag = egui::Rect::from_min_size(r.left_top() + egui::vec2(6.0, 6.0), egui::vec2((r.width() * 0.09).clamp(30.0, 64.0), (r.width() * 0.045).clamp(16.0, 30.0)));
        painter.rect_filled(tag, tag.height() / 2.0, egui::Color32::from_black_alpha(170));
        painter.text(tag.center(), egui::Align2::CENTER_CENTER, "360°", egui::FontId::proportional(tag.height() * 0.62), egui::Color32::WHITE);
    }

    /// Insert > 360° Image: pick a picture, then place it on the slide as a viewer.
    pub(super) fn insert_pano(&mut self) {
        let Some((key, name, w, h)) = self.load_image_asset_titled("Insert 360° Image") else { return };
        self.add_pano(key, &name, w, h);
    }

    /// A 360° image object for the asset `key` (a picture `w` by `h`).
    pub(super) fn add_pano(&mut self, key: String, name: &str, w: f32, h: f32) {
        let (sw, sh) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).map(|s| (s.width, s.height)).unwrap_or((1920.0, 1080.0));
        let vw = (sw * 0.66).min(1280.0).round();
        let mut o = blank_object(self.new_id("obj"), "360° image", Kind::Pano(Pano::new()), 0.0, 0.0, vw, (vw * 9.0 / 16.0).min(sh * 0.8).round());
        o.asset = Some(key);
        self.insert(o);
        let ratio = w / h.max(1.0);
        self.status = if (1.9..=2.1).contains(&ratio) {
            format!("Added {name}. Double-click it to add markers and hotspots.")
        } else {
            format!("Added {name}, but a 360° picture is normally twice as wide as it is tall ({w:.0} by {h:.0}); it will look stretched.")
        };
    }

    /// Open the editing mode for the 360° image at (layer, object) of the current slide.
    pub(super) fn open_pano_dialog(&mut self, (li, oi): (usize, usize)) {
        if self.pano.edit.is_some() { return; }
        let Some(o) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layers.get(li)).and_then(|l| l.objects.get(oi)) else { return };
        let (Kind::Pano(p), Some(key)) = (&o.kind, o.asset.clone()) else { return };
        let view = (p.yaw, p.pitch, p.fov);
        self.snapshot();
        self.pano.edit = Some(Edit { target: (li, oi), slide: self.slide, asset: key, view, selected: None, tool: Tool::Select, drag: None, changed: false });
        self.leave_master_view();
    }

    /// Close the editing mode; a visit that changed nothing leaves no undo step.
    pub(super) fn close_pano_edit(&mut self) {
        let Some(e) = self.pano.edit.take() else { return };
        if e.changed { self.dirty = true; } else { self.undo.pop(); }
    }

    /// The 360° image being edited.
    fn edited_object(&mut self) -> Option<&mut Object> {
        let (slide, (li, oi)) = { let e = self.pano.edit.as_ref()?; (e.slide, e.target) };
        self.project.as_mut()?.slides.get_mut(slide)?.layers.get_mut(li)?.objects.get_mut(oi)
    }

    /// The name of the 360° image being edited, for its tab.
    pub(super) fn pano_object_name(&mut self) -> String {
        self.edited_object().map(|o| o.name.clone()).unwrap_or_default()
    }

    /// Replace the settings of the 360° image being edited (from the ribbon).
    pub(super) fn pano_set(&mut self, p: Pano) { self.set_edited_pano(p); }

    /// The settings of the 360° image being edited (a copy).
    pub(super) fn edited_pano(&mut self) -> Option<Pano> {
        match &self.edited_object()?.kind { Kind::Pano(p) => Some(p.clone()), _ => None }
    }

    /// Replace the settings of the 360° image being edited.
    fn set_edited_pano(&mut self, p: Pano) {
        if let Some(o) = self.edited_object() { o.kind = Kind::Pano(p); }
        if let Some(e) = self.pano.edit.as_mut() { e.changed = true; }
        self.dirty = true;
    }

    /// Run a command from the ribbon.
    pub(super) fn pano_cmd(&mut self, cmd: Cmd) {
        let Some(mut p) = self.edited_pano() else { return };
        let Some((view, selected)) = self.pano.edit.as_ref().map(|e| (e.view, e.selected)) else { return };
        let mut select = selected;
        match cmd {
            Cmd::Close => { self.close_pano_edit(); return; }
            Cmd::AddMarker => {
                let id = p.new_pin_id();
                let mut m = PanoPin::marker(id, view.0, view.1);
                if let Some(plus) = crate::icons::all().iter().find(|i| i.names.contains(&"plus")) { m.icon = "plus".into(); m.icon_path = icon_in_box(plus, 0.5); }
                p.pins.push(m);
                select = Some(p.pins.len() - 1);
            }
            Cmd::StartHotspot => {
                if let Some(e) = self.pano.edit.as_mut() { e.tool = Tool::Draw(vec![]); }
                self.status = "Click each corner of the hotspot; double-click or press Enter to finish, Esc to cancel.".into();
                return;
            }
            Cmd::SetIcon(icon) => {
                let Some(it) = select.and_then(|i| p.pins.get_mut(i)).filter(|it| it.kind == PinKind::Marker) else { return };
                match icon {
                    Some(ic) => { it.icon = ic.names[0].to_string(); it.icon_path = icon_in_box(ic, 0.5); }
                    None => { it.icon.clear(); it.icon_path.clear(); }
                }
            }
            Cmd::SetColor(c) => { let Some(it) = select.and_then(|i| p.pins.get_mut(i)) else { return }; it.color = c; }
            Cmd::SwitchType => {
                let Some(it) = select.and_then(|i| p.pins.get_mut(i)) else { return };
                match it.kind {
                    PinKind::Marker => {
                        // A hotspot the size of the marker, round the marker's place.
                        let view = (it.yaw, it.pitch, 70.0);
                        let corners = [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| pano::look_at(view.0, view.1, view.2, 1.0, x * 0.12, y * 0.12));
                        it.kind = PinKind::Hotspot;
                        it.points = corners.to_vec();
                        it.recenter();
                    }
                    PinKind::Hotspot => {
                        it.kind = PinKind::Marker;
                        it.points.clear();
                        if it.icon.is_empty() { if let Some(plus) = crate::icons::all().iter().find(|i| i.names.contains(&"plus")) { it.icon = "plus".into(); it.icon_path = icon_in_box(plus, 0.5); } }
                    }
                }
            }
            Cmd::Delete => {
                let Some(i) = select.filter(|i| *i < p.pins.len()) else { return };
                let (obj, pin) = (self.edited_object().map(|o| o.id.clone()).unwrap_or_default(), p.pins[i].id.clone());
                p.pins.remove(i);
                select = None;
                // Triggers that watched it go with it.
                if let Some(s) = self.project.as_mut().and_then(|pr| pr.slides.get_mut(self.slide)) {
                    s.triggers.retain(|t| !matches!(&t.event, Event::PanoPin(o, q) if *o == obj && *q == pin));
                }
            }
            Cmd::AddTrigger => {
                let Some(pin) = select.and_then(|i| p.pins.get(i)).map(|it| it.id.clone()) else { self.status = "Select a marker or hotspot first.".into(); return };
                let obj = self.edited_object().map(|o| o.id.clone()).unwrap_or_default();
                let id = self.new_id("trig");
                let slide = self.slide;
                if let Some(s) = self.project.as_mut().and_then(|pr| pr.slides.get_mut(slide)) {
                    let layer = s.layers.get(1).or(s.layers.first()).map(|l| l.id.clone()).unwrap_or_default();
                    s.triggers.push(Trigger { id, owner: Owner::Object(obj.clone()), event: Event::PanoPin(obj, pin), action: Action::ShowLayer(layer), conditions: vec![] });
                }
                if let Some(e) = self.pano.edit.as_mut() { e.changed = true; }
                self.dirty = true;
                return;
            }
            Cmd::SetInitialView => { (p.yaw, p.pitch, p.fov) = view; self.status = "The learner will start looking here.".into(); }
        }
        if let Some(e) = self.pano.edit.as_mut() { e.selected = select; }
        self.set_edited_pano(p);
    }

    /// The editing mode's panels and canvas. Call after the ribbon; returns true while the mode is open.
    pub(super) fn pano_edit_ui(&mut self, root: &mut egui::Ui, ctx: &egui::Context) -> bool {
        if self.pano.edit.is_none() { return false; }
        if self.edited_pano().is_none() { self.close_pano_edit(); return false; }
        let mut pano = self.edited_pano().unwrap();
        let before = pano.clone();
        let (obj_id, aspect) = self.edited_object().map(|o| (o.id.clone(), o.w / o.h.max(1.0))).unwrap();
        let mut e = self.pano.edit.take().unwrap();

        // Left: how the learner moves, and the list of markers and hotspots.
        let mut select = e.selected;
        let mut remove: Option<usize> = None;
        let mut swap: Option<(usize, usize)> = None;
        egui::Panel::left("pano-items").default_size(260.0).show(root, |ui| {
            ui.add_space(8.0);
            ui.strong("Markers & Hotspots");
            ui.add_space(6.0);
            if ui.radio(!pano.tour, "Free navigation").on_hover_text("The learner looks around and presses what they find").clicked() { pano.tour = false; }
            if ui.radio(pano.tour, "Guided tour").on_hover_text("The learner is taken from one item to the next, in the order of this list, and is shown how many they have seen").clicked() { pano.tour = true; }
            ui.checkbox(&mut pano.progress, "Show progress indicator");
            ui.separator();
            ui.checkbox(&mut pano.auto_rotate, "Turn slowly on its own").on_hover_text("Stops as soon as the learner drags it, and they can pause it with a button");
            ui.checkbox(&mut pano.zoom, "Learner can zoom");
            ui.separator();
            egui::ScrollArea::vertical().id_salt("pano-list").show(ui, |ui| {
                for i in 0..pano.pins.len() {
                    let it = &pano.pins[i];
                    let glyph = if it.kind == PinKind::Hotspot { ribbon::NODES } else { icon_char(&it.icon).unwrap_or(ribbon::CIRCLE) };
                    let label = format!("{}  {}", glyph, item_name(&pano.pins, i));
                    let on = select == Some(i);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::click());
                    let v = ui.style().interact_selectable(&resp, on);
                    ui.painter().rect(rect, 4.0, if on { ui.visuals().selection.bg_fill.gamma_multiply(0.25) } else { ui.visuals().extreme_bg_color }, egui::Stroke::new(if on { 2.0 } else { 1.0 }, if on { ui.visuals().selection.bg_fill } else { v.bg_stroke.color }), egui::StrokeKind::Inside);
                    ui.painter().text(rect.left_center() + egui::vec2(10.0, 0.0), egui::Align2::LEFT_CENTER, glyph.to_string(), egui::FontId::new(16.0, ribbon::icons()), ui.visuals().text_color());
                    ui.painter().with_clip_rect(rect.shrink2(egui::vec2(8.0, 0.0))).text(rect.left_center() + egui::vec2(36.0, 0.0), egui::Align2::LEFT_CENTER, item_name(&pano.pins, i), egui::FontId::proportional(14.0), ui.visuals().text_color());
                    let _ = label;
                    if resp.clicked() { select = Some(i); e.view = (it.yaw, it.pitch, e.view.2); }
                }
                if pano.pins.is_empty() { ui.weak("Add a marker or a hotspot from the ribbon."); }
            });
            if let Some(i) = select.filter(|i| *i < pano.pins.len()) {
                ui.horizontal(|ui| {
                    if ui.add_enabled(i > 0, egui::Button::new("Move up")).on_hover_text("Earlier in a guided tour").clicked() { swap = Some((i, i - 1)); }
                    if ui.add_enabled(i + 1 < pano.pins.len(), egui::Button::new("Move down")).clicked() { swap = Some((i, i + 1)); }
                    if ui.button("Delete").clicked() { remove = Some(i); }
                });
            }
        });
        if let Some((a, b)) = swap { pano.pins.swap(a, b); select = Some(b); }
        if let Some(i) = remove {
            let pin = pano.pins.remove(i).id;
            select = None;
            if let Some(s) = self.project.as_mut().and_then(|pr| pr.slides.get_mut(e.slide)) {
                s.triggers.retain(|t| !matches!(&t.event, Event::PanoPin(o, q) if *o == obj_id && *q == pin));
            }
            e.changed = true;
        }

        // Right: the triggers of this 360° image.
        let mut trig_changed = false;
        egui::Panel::right("pano-triggers").default_size(326.0).show(root, |ui| {
            ui.add_space(8.0);
            ui.strong("Triggers");
            ui.weak("Pressing a marker or a hotspot can run these.");
            ui.add_space(4.0);
            let mut ctr = self.id_counter;
            let t0 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
            let mut gen = || { ctr += 1; format!("trig-{t0:x}-{ctr}") };
            if let Some(proj) = self.project.as_mut() {
                let vars = proj.variables.clone();
                let slide_list: Vec<(String, String)> = proj.slides.iter().map(|s| (s.id.clone(), s.name.clone())).collect();
                let new_event = select.and_then(|i| pano.pins.get(i)).map(|it| Event::PanoPin(obj_id.clone(), it.id.clone()));
                egui::ScrollArea::vertical().id_salt("pano-trig-scroll").show(ui, |ui| {
                    if let Some(s) = proj.slides.get_mut(e.slide) {
                        trig_changed = triggers_ui(ui, s, &slide_list, &vars, &Owner::Object(obj_id.clone()), new_event, &mut gen);
                    }
                });
            }
            self.id_counter = ctr;
        });
        if trig_changed { e.changed = true; self.dirty = true; }

        // The picture.
        let mut finish: Option<Vec<(f32, f32)>> = None;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::from_rgb(176, 184, 192))).show_inside(root, |ui| {
            let avail = ui.available_rect_before_wrap().shrink(24.0);
            let size = if avail.width() / avail.height().max(1.0) > aspect { egui::vec2(avail.height() * aspect, avail.height()) } else { egui::vec2(avail.width(), avail.width() / aspect) };
            let cv = egui::Rect::from_center_size(avail.center(), size.max(egui::vec2(64.0, 36.0)));
            let w = (cv.width().round() as usize).clamp(64, 960);
            let h = ((w as f32 / aspect).round() as usize).max(36);
            let tex = self.pano_texture(ctx, "edit", &e.asset, e.view, w, h);
            let resp = ui.interact(cv, egui::Id::new("pano-canvas"), egui::Sense::click_and_drag());
            match &tex {
                Some(t) => { ui.painter().image(t.id(), cv, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE); }
                None => { ui.painter().rect_filled(cv, 4.0, egui::Color32::from_gray(50)); ui.painter().text(cv.center(), egui::Align2::CENTER_CENTER, "The picture could not be read.", egui::FontId::proportional(14.0), egui::Color32::WHITE); }
            }
            draw_items(ui.painter(), cv, e.view, &pano, select, MARKER_R);
            let view0 = e.view;
            let to_world = |at: egui::Pos2| {
                let x = (at.x - cv.center().x) / (cv.width() / 2.0);
                let y = -(at.y - cv.center().y) / (cv.height() / 2.0);
                pano::look_at(view0.0, view0.1, view0.2, cv.width() / cv.height().max(1.0), x, y)
            };
            // What is under the pointer: a corner of the selected hotspot, a marker, or a hotspot.
            let hit = |at: egui::Pos2, pano: &Pano, select: Option<usize>| -> Option<Drag> {
                if let Some(i) = select.filter(|i| pano.pins.get(*i).is_some_and(|it| it.kind == PinKind::Hotspot)) {
                    if let Some(pts) = corners_at(cv, view0, &pano.pins[i]) {
                        if let Some(k) = pts.iter().position(|p| p.distance(at) <= 10.0) { return Some(Drag::Corner(i, k)); }
                    }
                }
                if let Some((i, _)) = pano.pins.iter().enumerate().rev().find(|(_, it)| it.kind == PinKind::Marker && marker_at(cv, view0, it).is_some_and(|m| m.distance(at) <= MARKER_R + 2.0)) { return Some(Drag::Item(i)); }
                pano.pins.iter().enumerate().rev().find(|(_, it)| it.kind == PinKind::Hotspot && corners_at(cv, view0, it).is_some_and(|pts| inside(&pts, at))).map(|(i, _)| Drag::Item(i))
            };
            match e.tool.clone() {
                Tool::Draw(mut pts) => {
                    if resp.hovered() { ctx.set_cursor_icon(egui::CursorIcon::Crosshair); }
                    if let Some(at) = resp.interact_pointer_pos().filter(|_| resp.clicked() || resp.double_clicked()) {
                        let near_first = pts.first().and_then(|&(y, p)| marker_at(cv, e.view, &PanoPin::marker(String::new(), y, p))).is_some_and(|f| f.distance(at) < 14.0);
                        if resp.double_clicked() || (near_first && pts.len() >= 3) {
                            if pts.len() >= 3 { finish = Some(pts.clone()); } else { e.tool = Tool::Select; }
                        } else {
                            pts.push(to_world(at));
                            e.tool = Tool::Draw(pts.clone());
                        }
                    }
                    // The outline so far, with a line to the pointer.
                    let mut screen: Vec<egui::Pos2> = pts.iter().filter_map(|&(y, p)| marker_at(cv, e.view, &PanoPin::marker(String::new(), y, p))).collect();
                    if let Some(hover) = resp.hover_pos() { if !screen.is_empty() { screen.push(hover); } }
                    ui.painter().add(egui::Shape::line(screen.clone(), egui::Stroke::new(2.0, col(pano::HOTSPOT_COLOR, 255))));
                    for p in &screen { ui.painter().circle(*p, 4.0, egui::Color32::WHITE, egui::Stroke::new(1.0, col(pano::HOTSPOT_COLOR, 255))); }
                    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && pts.len() >= 3 { finish = Some(pts.clone()); }
                    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { e.tool = Tool::Select; self.status.clear(); }
                }
                Tool::Select => {
                    if resp.drag_started() {
                        e.drag = resp.interact_pointer_pos().and_then(|at| hit(at, &pano, select)).or(Some(Drag::Look));
                        if let Some(Drag::Item(i) | Drag::Corner(i, _)) = &e.drag { select = Some(*i); }
                    }
                    if resp.dragged() {
                        let at = resp.interact_pointer_pos();
                        match (e.drag.clone(), at) {
                            (Some(Drag::Item(i)), Some(at)) => {
                                let (yaw, pitch) = to_world(at);
                                if let Some(it) = pano.pins.get_mut(i) {
                                    if it.kind == PinKind::Marker { it.yaw = yaw; it.pitch = pitch; }
                                    else {
                                        // The whole hotspot moves by how far its middle moved.
                                        let (dy, dp) = (pano::wrap_yaw(yaw - it.yaw), pitch - it.pitch);
                                        for pt in it.points.iter_mut() { pt.0 = pano::wrap_yaw(pt.0 + dy); pt.1 = pano::clamp_pitch(pt.1 + dp); }
                                        it.recenter();
                                    }
                                }
                            }
                            (Some(Drag::Corner(i, k)), Some(at)) => {
                                let w = to_world(at);
                                if let Some(it) = pano.pins.get_mut(i) { if let Some(pt) = it.points.get_mut(k) { *pt = w; } it.recenter(); }
                            }
                            _ => {
                                let k = e.view.2 / cv.width();
                                let dr = resp.drag_delta();
                                e.view.0 = pano::wrap_yaw(e.view.0 - dr.x * k);
                                e.view.1 = pano::clamp_pitch(e.view.1 + dr.y * k);
                            }
                        }
                        ctx.request_repaint();
                    }
                    if resp.drag_stopped() { e.drag = None; }
                    if resp.clicked() {
                        select = resp.interact_pointer_pos().and_then(|at| hit(at, &pano, select)).and_then(|d| match d { Drag::Item(i) | Drag::Corner(i, _) => Some(i), Drag::Look => None });
                    }
                    if resp.hovered() {
                        let dy = ctx.input(|i| i.smooth_scroll_delta.y);
                        if dy != 0.0 { e.view.2 = pano::clamp_fov(e.view.2 * (-dy * 0.004).exp()); }
                        let over = ctx.pointer_hover_pos().and_then(|at| hit(at, &pano, select));
                        // A corner shows the resize arrows a shape's handles show: the diagonal that points away from the middle.
                        let corner_cursor = |i: usize, k: usize| {
                            let pts = pano.pins.get(i).and_then(|it| corners_at(cv, view0, it));
                            match pts.and_then(|pts| { let n = pts.len() as f32; let mid = egui::pos2(pts.iter().map(|p| p.x).sum::<f32>() / n, pts.iter().map(|p| p.y).sum::<f32>() / n); pts.get(k).map(|p| (p.x - mid.x, p.y - mid.y)) }) {
                                Some((dx, dy)) if (dx < 0.0) == (dy < 0.0) => egui::CursorIcon::ResizeNwSe,
                                Some(_) => egui::CursorIcon::ResizeNeSw,
                                None => egui::CursorIcon::Move,
                            }
                        };
                        ctx.set_cursor_icon(match (&e.drag, over) {
                            (Some(Drag::Corner(i, k)), _) => corner_cursor(*i, *k),
                            (Some(_), _) => egui::CursorIcon::Grabbing,
                            (None, Some(Drag::Corner(i, k))) => corner_cursor(i, k),
                            (None, Some(Drag::Item(i))) if select == Some(i) => egui::CursorIcon::Grab,
                            (None, Some(_)) => egui::CursorIcon::PointingHand,
                            _ => egui::CursorIcon::Grab,
                        });
                    }
                    // The selected item's corners can be dragged.
                    if let Some(it) = select.and_then(|i| pano.pins.get(i)).filter(|it| it.kind == PinKind::Hotspot) {
                        if let Some(pts) = corners_at(cv, e.view, it) { for p in pts { ui.painter().rect(egui::Rect::from_center_size(p, egui::vec2(9.0, 9.0)), 1.0, egui::Color32::WHITE, egui::Stroke::new(1.0, col(it.colour(), 255)), egui::StrokeKind::Middle); } }
                    }
                }
            }
            // The selected item's card, edited where it sits.
            if let Some(i) = select.filter(|i| *i < pano.pins.len()) {
                let it = &pano.pins[i];
                let anchor = if it.kind == PinKind::Marker { marker_at(cv, e.view, it) } else { corners_at(cv, e.view, it).map(|pts| { let n = pts.len() as f32; egui::pos2(pts.iter().map(|p| p.x).sum::<f32>() / n, pts.iter().map(|p| p.y).sum::<f32>() / n) }) };
                if let Some(at) = anchor {
                    let (fill, ink) = if pano.dark { (egui::Color32::from_rgb(24, 24, 24), egui::Color32::WHITE) } else { (egui::Color32::WHITE, egui::Color32::from_rgb(24, 24, 24)) };
                    let width = 240.0f32;
                    let pos = egui::pos2((at.x + MARKER_R + 14.0).min(cv.right() - width - 12.0).max(cv.left() + 8.0), (at.y - 36.0).clamp(cv.top() + 8.0, cv.bottom() - 120.0));
                    egui::Area::new(egui::Id::new("pano-card")).fixed_pos(pos).constrain(false).order(egui::Order::Foreground).show(ctx, |ui| {
                        egui::Frame::new().fill(fill).corner_radius(if pano.rounded { 10 } else { 0 }).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                            ui.set_width(width);
                            ui.visuals_mut().override_text_color = Some(ink);
                            let it = &mut pano.pins[i];
                            ui.add(egui::TextEdit::singleline(&mut it.title).hint_text("Enter Title...").desired_width(f32::INFINITY).font(egui::FontId::proportional(17.0)).frame(egui::Frame::NONE));
                            ui.add(egui::TextEdit::multiline(&mut it.description).hint_text("Enter body...").desired_rows(2).desired_width(f32::INFINITY).font(egui::FontId::proportional(13.0)).frame(egui::Frame::NONE));
                            if it.kind == PinKind::Hotspot && !pano.reveal {
                                ui.add(egui::TextEdit::singleline(&mut it.hint).hint_text("Hint: where to look").desired_width(f32::INFINITY).font(egui::FontId::proportional(12.0)).frame(egui::Frame::NONE));
                            }
                        });
                    });
                }
            }
            if matches!(e.tool, Tool::Select) { ui.painter().text(cv.left_bottom() + egui::vec2(8.0, -8.0), egui::Align2::LEFT_BOTTOM, "Drag to look around. Scroll to zoom.", egui::FontId::proportional(12.0), egui::Color32::from_white_alpha(200)); }
        });
        if let Some(pts) = finish {
            let id = pano.new_pin_id();
            pano.pins.push(PanoPin::hotspot(id, pts));
            select = Some(pano.pins.len() - 1);
            e.tool = Tool::Select;
            self.status.clear();
        }
        e.selected = select;
        let moved = pano != before;
        if moved { e.changed = true; }
        self.pano.edit = Some(e);
        if moved { self.set_edited_pano(pano); }
        true
    }
}

/// Whether `at` is inside the polygon `pts`.
fn inside(pts: &[egui::Pos2], at: egui::Pos2) -> bool {
    let mut yes = false;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        if (a.y > at.y) != (b.y > at.y) && at.x < (b.x - a.x) * (at.y - a.y) / (b.y - a.y) + a.x { yes = !yes; }
    }
    yes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4 x 2 panorama whose columns are different colours, to see which part a view shows.
    fn strip() -> Source {
        let cols = [[255u8, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0]];
        let mut rgb = Vec::new();
        for _ in 0..2 { for c in cols { rgb.extend(c); } }
        Source { w: 4, h: 2, rgb }
    }

    #[test]
    fn a_view_shows_the_part_of_the_picture_it_points_at() {
        let s = strip();
        let centre = |yaw: f32| { let i = s.view((yaw, 0.0, 30.0), 9, 9); i.pixels[4 * 9 + 4] };
        // Heading 0 is the middle of the picture, the boundary of columns 1 and 2; 45 degrees either side is inside one.
        assert!(centre(45.0).b() > 200 && centre(45.0).r() < 60, "{:?}", centre(45.0));
        assert!(centre(-45.0).g() > 200 && centre(-45.0).r() < 60, "{:?}", centre(-45.0));
        assert!(centre(135.0).r() > 200 && centre(135.0).g() > 200 && centre(135.0).b() < 60, "{:?}", centre(135.0));
        assert!(centre(-135.0).r() > 200 && centre(-135.0).g() < 60, "{:?}", centre(-135.0));
    }

    #[test]
    fn pins_show_where_they_are_and_not_behind_the_viewer() {
        let pins = [
            PanoPin::marker("a".into(), 0.0, 0.0),
            PanoPin::marker("b".into(), 180.0, 0.0),
            PanoPin::marker("c".into(), 80.0, 0.0),
        ];
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(640.0, 360.0));
        let shown: Vec<_> = pins.iter().enumerate().filter_map(|(i, p)| marker_at(rect, (0.0, 0.0, 90.0), p).map(|at| (i, at))).collect();
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].0, 0);
        assert!((shown[0].1 - rect.center()).length() < 0.5);
    }

    #[test]
    fn large_pictures_are_shrunk_for_previews() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(4096, 2048, image::Rgb([10, 20, 30]))).write_to(&mut png, image::ImageFormat::Png).unwrap();
        let s = Source::decode(&png.into_inner()).unwrap();
        assert_eq!((s.w, s.h), (2048, 1024));
        assert!(Source::decode(b"not a picture").is_none());
    }
}
