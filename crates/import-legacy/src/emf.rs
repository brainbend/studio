//! A small renderer for Windows Enhanced Metafiles (EMF), enough for the vector illustrations in
//! imported character libraries: filled paths, clip paths, polygons, Béziers and device
//! independent bitmaps (including the mask-and-picture pairs GDI uses for transparency).
//!
//! The metafile is replayed record by record into a `tiny-skia` pixmap. Text, gradients, arcs and
//! raster operations other than copy and the transparency pairs are not drawn, and hatches are drawn
//! solid; [`render`] reports which of these a file used.
use std::collections::{BTreeSet, HashMap};
use tiny_skia::{FillRule, Mask, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// The longest side we are willing to draw, in pixels.
const MAX_SIDE: u32 = 2400;

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}
fn i32_at(d: &[u8], o: usize) -> Option<i32> {
    u32_at(d, o).map(|v| v as i32)
}
fn f32_at(d: &[u8], o: usize) -> Option<f32> {
    u32_at(d, o).map(f32::from_bits)
}
fn i16_at(d: &[u8], o: usize) -> Option<i16> {
    Some(i16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

#[derive(Clone, Copy)]
struct Xform([f32; 6]);

impl Xform {
    const IDENTITY: Xform = Xform([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        let m = &self.0;
        (x * m[0] + y * m[2] + m[4], x * m[1] + y * m[3] + m[5])
    }
    /// `self` applied first, then `then`.
    fn then(&self, then: &Xform) -> Xform {
        let (a, b) = (&self.0, &then.0);
        Xform([
            a[0] * b[0] + a[1] * b[2],
            a[0] * b[1] + a[1] * b[3],
            a[2] * b[0] + a[3] * b[2],
            a[2] * b[1] + a[3] * b[3],
            a[4] * b[0] + a[5] * b[2] + b[4],
            a[4] * b[1] + a[5] * b[3] + b[5],
        ])
    }
}

#[derive(Clone)]
enum Brush {
    Solid([u8; 4]),
    Hollow,
}

#[derive(Clone)]
struct Pen {
    color: [u8; 4],
    width: f32,
    hollow: bool,
}

#[derive(Clone)]
enum Obj {
    Brush(Brush),
    Pen(Pen),
}

/// The parts of the device context that save and restore together.
#[derive(Clone)]
struct Dc {
    world: Xform,
    win_org: (f32, f32),
    win_ext: (f32, f32),
    view_org: (f32, f32),
    view_ext: (f32, f32),
    brush: Brush,
    pen: Pen,
    even_odd: bool,
    clip: Option<Mask>,
}

impl Dc {
    fn new() -> Dc {
        Dc {
            world: Xform::IDENTITY,
            win_org: (0.0, 0.0),
            win_ext: (1.0, 1.0),
            view_org: (0.0, 0.0),
            view_ext: (1.0, 1.0),
            brush: Brush::Solid([255, 255, 255, 255]),
            pen: Pen { color: [0, 0, 0, 255], width: 1.0, hollow: false },
            even_odd: true,
            clip: None,
        }
    }
}

/// One decoded bitmap, top row first.
struct Dib {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
    bits: u16,
}

struct Renderer {
    out: Pixmap,
    /// Device units to output pixels.
    to_px: Xform,
    dc: Dc,
    stack: Vec<Dc>,
    objects: HashMap<u32, Obj>,
    /// The path being built between BEGINPATH and ENDPATH, in output pixels.
    path: Option<PathBuilder>,
    /// The current position, in logical units.
    cur: (f32, f32),
    /// A 1-bit mask drawn by a raster operation that pairs it with the picture drawn next.
    pending_mask: Option<(Dib, [i32; 4], u32)>,
    /// What the metafile asked for that is not drawn (or only roughly), by name.
    skipped: BTreeSet<&'static str>,
    /// Whether anything has been painted, and whether the file carries EMF+ records.
    drew: bool,
    emf_plus: bool,
}

impl Renderer {
    /// A point in logical units to output pixels.
    fn pt(&self, x: f32, y: f32) -> (f32, f32) {
        let (wx, wy) = self.dc.world.apply(x, y);
        let dc = &self.dc;
        let dx = (wx - dc.win_org.0) * dc.view_ext.0 / dc.win_ext.0 + dc.view_org.0;
        let dy = (wy - dc.win_org.1) * dc.view_ext.1 / dc.win_ext.1 + dc.view_org.1;
        self.to_px.apply(dx, dy)
    }

    fn scale(&self) -> f32 {
        let (a, b, c) = (self.pt(0.0, 0.0), self.pt(1.0, 0.0), self.pt(0.0, 1.0));
        let det = ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs();
        det.sqrt().max(1e-6)
    }

    fn paint(color: [u8; 4]) -> Paint<'static> {
        let mut p = Paint::default();
        p.set_color_rgba8(color[0], color[1], color[2], color[3]);
        p.anti_alias = true;
        p
    }

    fn fill_rule(&self) -> FillRule {
        if self.dc.even_odd { FillRule::EvenOdd } else { FillRule::Winding }
    }

    fn fill(&mut self, path: &tiny_skia::Path) {
        if let Brush::Solid(c) = self.dc.brush {
            self.drew = true;
            let rule = self.fill_rule();
            self.out.fill_path(path, &Self::paint(c), rule, Transform::identity(), self.dc.clip.as_ref());
        }
    }

    fn stroke(&mut self, path: &tiny_skia::Path) {
        if self.dc.pen.hollow { return; }
        self.drew = true;
        let stroke = Stroke { width: (self.dc.pen.width * self.scale()).max(1.0), ..Stroke::default() };
        self.out.stroke_path(path, &Self::paint(self.dc.pen.color), &stroke, Transform::identity(), self.dc.clip.as_ref());
    }

    fn clip_with(&mut self, path: &tiny_skia::Path, mode: u32) {
        let (w, h) = (self.out.width(), self.out.height());
        let rule = self.fill_rule();
        match mode {
            // RGN_AND
            1 => match self.dc.clip.as_mut() {
                Some(m) => m.intersect_path(path, rule, true, Transform::identity()),
                None => {
                    if let Some(mut m) = Mask::new(w, h) {
                        m.fill_path(path, rule, true, Transform::identity());
                        self.dc.clip = Some(m);
                    }
                }
            },
            // RGN_COPY
            5 => {
                if let Some(mut m) = Mask::new(w, h) {
                    m.fill_path(path, rule, true, Transform::identity());
                    self.dc.clip = Some(m);
                }
            }
            // OR, XOR and DIFF are rare in illustrations; leaving the clip alone is the safe choice.
            _ => {
                self.skipped.insert("some clipping");
            }
        }
    }

    /// An ellipse, rectangle or rounded rectangle in logical units, mapped to output pixels.
    fn box_path(&self, t: u32, l: f32, tp: f32, r: f32, b: f32, cw: f32, ch: f32) -> Option<tiny_skia::Path> {
        let (l, r) = (l.min(r), l.max(r));
        let (tp, b) = (tp.min(b), tp.max(b));
        if t == 43 { return self.rect_path(l, tp, r, b); }
        // Four Bézier arcs (or a rounded box); k is the usual circle constant.
        const K: f32 = 0.552_284_75;
        let (rx, ry) = if t == 42 { ((r - l) / 2.0, (b - tp) / 2.0) } else { ((cw / 2.0).clamp(0.0, (r - l) / 2.0), (ch / 2.0).clamp(0.0, (b - tp) / 2.0)) };
        let mut pb = PathBuilder::new();
        let m = |x: f32, y: f32| self.pt(x, y);
        let p = m(l + rx, tp);
        pb.move_to(p.0, p.1);
        let line = |pb: &mut PathBuilder, x: f32, y: f32| { let q = m(x, y); pb.line_to(q.0, q.1); };
        let arc = |pb: &mut PathBuilder, c1: (f32, f32), c2: (f32, f32), e: (f32, f32)| {
            let (a, bq, cq) = (m(c1.0, c1.1), m(c2.0, c2.1), m(e.0, e.1));
            pb.cubic_to(a.0, a.1, bq.0, bq.1, cq.0, cq.1);
        };
        line(&mut pb, r - rx, tp);
        arc(&mut pb, (r - rx + rx * K, tp), (r, tp + ry - ry * K), (r, tp + ry));
        line(&mut pb, r, b - ry);
        arc(&mut pb, (r, b - ry + ry * K), (r - rx + rx * K, b), (r - rx, b));
        line(&mut pb, l + rx, b);
        arc(&mut pb, (l + rx - rx * K, b), (l, b - ry + ry * K), (l, b - ry));
        line(&mut pb, l, tp + ry);
        arc(&mut pb, (l, tp + ry - ry * K), (l + rx - rx * K, tp), (l + rx, tp));
        pb.close();
        pb.finish()
    }

    fn rect_path(&self, l: f32, t: f32, r: f32, b: f32) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        let pts = [self.pt(l, t), self.pt(r, t), self.pt(r, b), self.pt(l, b)];
        pb.move_to(pts[0].0, pts[0].1);
        for p in &pts[1..] {
            pb.line_to(p.0, p.1);
        }
        pb.close();
        pb.finish()
    }

    /// Hand a finished shape to the open path bracket, or draw it straight away (filled when `closed`).
    /// `joins` is for the "-to" records, which carry on from the current point instead of starting a figure.
    fn emit(&mut self, pb: PathBuilder, closed: bool, joins: bool) {
        match self.path.as_mut() {
            Some(open) => {
                if let Some(p) = pb.finish() { append(open, &p, joins); }
            }
            None => {
                if let Some(p) = pb.finish() {
                    if closed { self.fill(&p); }
                    self.stroke(&p);
                }
            }
        }
    }

    fn run(&mut self, d: &[u8]) {
        let mut o = 0usize;
        while o + 8 <= d.len() {
            let (Some(t), Some(size)) = (u32_at(d, o), u32_at(d, o + 4)) else { break };
            let size = size as usize;
            if size < 8 || o + size > d.len() { break; }
            self.record(t, &d[o..o + size]);
            if t == 14 { break; }
            o += size;
        }
    }

    fn points(r: &[u8], at: usize, n: usize, wide: bool) -> Vec<(f32, f32)> {
        (0..n)
            .filter_map(|i| {
                if wide {
                    Some((i32_at(r, at + i * 8)? as f32, i32_at(r, at + i * 8 + 4)? as f32))
                } else {
                    Some((i16_at(r, at + i * 4)? as f32, i16_at(r, at + i * 4 + 2)? as f32))
                }
            })
            .collect()
    }

    fn record(&mut self, t: u32, r: &[u8]) {
        let f = |o: usize| f32_at(r, o).unwrap_or(0.0);
        let i = |o: usize| i32_at(r, o).unwrap_or(0);
        let u = |o: usize| u32_at(r, o).unwrap_or(0);
        match t {
            9 => self.dc.win_ext = (i(8) as f32, i(12) as f32),
            10 => self.dc.win_org = (i(8) as f32, i(12) as f32),
            11 => self.dc.view_ext = (i(8) as f32, i(12) as f32),
            12 => self.dc.view_org = (i(8) as f32, i(12) as f32),
            // MM_TEXT: one device unit per logical unit.
            17 if u(8) == 1 => {
                self.dc.win_ext = (1.0, 1.0);
                self.dc.view_ext = (1.0, 1.0);
            }
            19 => self.dc.even_odd = u(8) != 2,
            33 => self.stack.push(self.dc.clone()),
            34 => {
                let back = (-i(8)).max(1) as usize;
                for _ in 0..back {
                    if let Some(dc) = self.stack.pop() { self.dc = dc; }
                }
            }
            35 => self.dc.world = Xform([f(8), f(12), f(16), f(20), f(24), f(28)]),
            36 => {
                let m = Xform([f(8), f(12), f(16), f(20), f(24), f(28)]);
                self.dc.world = match u(32) {
                    1 => Xform::IDENTITY,
                    2 => m.then(&self.dc.world),
                    3 => self.dc.world.then(&m),
                    _ => m,
                };
            }
            37 => self.select(u(8)),
            // CREATEPEN: ihPen, style, width, (unused), color
            38 => {
                self.objects.insert(u(8), Obj::Pen(Pen { color: colorref(u(24)), width: i(16).max(1) as f32, hollow: u(12) == 5 }));
            }
            // CREATEBRUSHINDIRECT: ihBrush, style, color, hatch
            39 => {
                let b = match u(12) {
                    0 => Brush::Solid(colorref(u(16))),
                    // Hatches and patterns are approximated by their color.
                    2 => {
                        self.skipped.insert("hatched fills (drawn solid)");
                        Brush::Solid(colorref(u(16)))
                    }
                    3 => {
                        self.skipped.insert("pattern fills (drawn solid)");
                        Brush::Solid(colorref(u(16)))
                    }
                    1 => Brush::Hollow,
                    _ => {
                        self.skipped.insert("pattern fills");
                        Brush::Hollow
                    }
                };
                self.objects.insert(u(8), Obj::Brush(b));
            }
            // EXTCREATEPEN: ihPen, offBmi, cbBmi, offBits, cbBits, then the pen: style, width, brush style, color
            95 => {
                self.objects.insert(u(8), Obj::Pen(Pen { color: colorref(u(48)), width: u(40).max(1) as f32, hollow: u(36) & 0xFF == 5 }));
            }
            40 => {
                self.objects.remove(&u(8));
            }
            27 => {
                self.cur = (i(8) as f32, i(12) as f32);
                let q = self.pt(self.cur.0, self.cur.1);
                if let Some(open) = self.path.as_mut() { open.move_to(q.0, q.1); }
            }
            54 => {
                let (from, to) = (self.cur, (i(8) as f32, i(12) as f32));
                self.cur = to;
                let mut pb = PathBuilder::new();
                let (p, q) = (self.pt(from.0, from.1), self.pt(to.0, to.1));
                pb.move_to(p.0, p.1);
                pb.line_to(q.0, q.1);
                self.emit(pb, false, true);
            }
            59 => self.path = Some(PathBuilder::new()),
            61 => {
                if let Some(p) = self.path.as_mut() { p.close(); }
            }
            62..=64 => {
                if let Some(p) = self.path.take().and_then(|pb| pb.finish()) {
                    if t != 64 { self.fill(&p); }
                    if t != 62 { self.stroke(&p); }
                }
            }
            67 => {
                if let Some(p) = self.path.take().and_then(|pb| pb.finish()) { self.clip_with(&p, u(8)); }
            }
            68 => self.path = None,
            // INTERSECTCLIPRECT
            30 => {
                if let Some(p) = self.rect_path(i(8) as f32, i(12) as f32, i(16) as f32, i(20) as f32) { self.clip_with(&p, 1); }
            }
            // EXTSELECTCLIPRGN: cbRgnData, mode, then RGNDATA (a 32-byte header, then rectangles)
            75 => self.select_region(r),
            // POLYBEZIER, POLYGON, POLYLINE, POLYBEZIERTO, POLYLINETO and their 16-bit forms
            2..=6 | 85..=89 => {
                let wide = t < 80;
                let pts = Self::points(r, 28, u(24) as usize, wide);
                self.poly(if wide { t } else { t - 83 }, &pts);
            }
            // POLYPOLYGON and POLYPOLYLINE16 / POLYPOLYGON16
            8 | 90 | 91 => {
                let wide = t < 80;
                let (np, total) = (u(24) as usize, u(28) as usize);
                let counts: Vec<usize> = (0..np).map(|k| u(32 + k * 4) as usize).collect();
                let pts = Self::points(r, 32 + np * 4, total, wide);
                let closed = t != 90;
                let mut at = 0;
                let mut pb = PathBuilder::new();
                for c in counts {
                    let run = &pts[at.min(pts.len())..(at + c).min(pts.len())];
                    at += c;
                    if run.len() < 2 { continue; }
                    let p0 = self.pt(run[0].0, run[0].1);
                    pb.move_to(p0.0, p0.1);
                    for p in &run[1..] {
                        let q = self.pt(p.0, p.1);
                        pb.line_to(q.0, q.1);
                    }
                    if closed { pb.close(); }
                }
                self.emit(pb, closed, false);
            }
            // ELLIPSE, RECTANGLE, ROUNDRECT: a box in logical units, filled with the brush and outlined with the pen
            42..=44 => {
                let (l, tp, rt, bt) = (i(8) as f32, i(12) as f32, i(16) as f32, i(20) as f32);
                let (cw, ch) = if t == 44 { (i(24) as f32, i(28) as f32) } else { (0.0, 0.0) };
                if let Some(p) = self.box_path(t, l, tp, rt, bt, cw, ch) {
                    self.fill(&p);
                    self.stroke(&p);
                }
            }
            81 => self.stretch_dibits(r),
            // BITBLT / STRETCHBLT: only copies of a bitmap are understood.
            76 | 77 => self.blt(t, r),
            // Comments: an EMF+ signature means the file may carry its drawing in a newer format.
            70 => {
                if r.get(12..16) == Some(b"EMF+") { self.emf_plus = true; }
            }
            _ => {
                if let Some(what) = unsupported(t) { self.skipped.insert(what); }
            }
        }
    }

    fn select(&mut self, h: u32) {
        if h & 0x8000_0000 != 0 {
            match h & 0x7FFF_FFFF {
                0 => self.dc.brush = Brush::Solid([255, 255, 255, 255]),
                1 => self.dc.brush = Brush::Solid([192, 192, 192, 255]),
                2 => self.dc.brush = Brush::Solid([128, 128, 128, 255]),
                3 => self.dc.brush = Brush::Solid([64, 64, 64, 255]),
                4 => self.dc.brush = Brush::Solid([0, 0, 0, 255]),
                5 => self.dc.brush = Brush::Hollow,
                6 => self.dc.pen = Pen { color: [255, 255, 255, 255], width: 1.0, hollow: false },
                7 => self.dc.pen = Pen { color: [0, 0, 0, 255], width: 1.0, hollow: false },
                8 => self.dc.pen.hollow = true,
                _ => {}
            }
        } else {
            match self.objects.get(&h).cloned() {
                Some(Obj::Brush(b)) => self.dc.brush = b,
                Some(Obj::Pen(p)) => self.dc.pen = p,
                None => {}
            }
        }
    }

    fn select_region(&mut self, r: &[u8]) {
        let i = |o: usize| i32_at(r, o).unwrap_or(0);
        let (cb, mode) = (u32_at(r, 8).unwrap_or(0), u32_at(r, 12).unwrap_or(0));
        if cb == 0 {
            if mode == 5 { self.dc.clip = None; }
            return;
        }
        // RGNDATAHEADER: dwSize, iType, nCount, nRgnSize, rcBound; the rectangles follow it.
        let n = u32_at(r, 16 + 8).unwrap_or(0) as usize;
        let mut pb = PathBuilder::new();
        for k in 0..n.min(10_000) {
            let at = 16 + 32 + k * 16;
            let (l, tp, rt, b) = (i(at) as f32, i(at + 4) as f32, i(at + 8) as f32, i(at + 12) as f32);
            let pts = [self.pt(l, tp), self.pt(rt, tp), self.pt(rt, b), self.pt(l, b)];
            pb.move_to(pts[0].0, pts[0].1);
            for p in &pts[1..] { pb.line_to(p.0, p.1); }
            pb.close();
        }
        if let Some(p) = pb.finish() { self.clip_with(&p, if mode == 1 { 1 } else { 5 }); }
    }

    /// Kinds: 2 Bézier, 3 polygon, 4 polyline, 5 Bézier-to, 6 line-to.
    fn poly(&mut self, kind: u32, pts: &[(f32, f32)]) {
        let mut pb = PathBuilder::new();
        let mut closed = false;
        match kind {
            3 | 4 => {
                if pts.len() < 2 { return; }
                let p0 = self.pt(pts[0].0, pts[0].1);
                pb.move_to(p0.0, p0.1);
                for p in &pts[1..] {
                    let q = self.pt(p.0, p.1);
                    pb.line_to(q.0, q.1);
                }
                if kind == 3 {
                    pb.close();
                    closed = true;
                }
                self.cur = pts[pts.len() - 1];
            }
            2 | 5 => {
                let (start, rest) = if kind == 2 {
                    let Some(first) = pts.first() else { return };
                    (*first, &pts[1..])
                } else {
                    (self.cur, pts)
                };
                let s = self.pt(start.0, start.1);
                pb.move_to(s.0, s.1);
                for c in rest.chunks_exact(3) {
                    let (a, b, e) = (self.pt(c[0].0, c[0].1), self.pt(c[1].0, c[1].1), self.pt(c[2].0, c[2].1));
                    pb.cubic_to(a.0, a.1, b.0, b.1, e.0, e.1);
                }
                if let Some(last) = rest.chunks_exact(3).last() { self.cur = last[2]; }
            }
            6 => {
                let s = self.pt(self.cur.0, self.cur.1);
                pb.move_to(s.0, s.1);
                for p in pts {
                    let q = self.pt(p.0, p.1);
                    pb.line_to(q.0, q.1);
                }
                if let Some(last) = pts.last() { self.cur = *last; }
            }
            _ => return,
        }
        self.emit(pb, closed, matches!(kind, 5 | 6));
    }

    // ---- bitmaps -------------------------------------------------------------------------

    /// STRETCHDIBITS: bounds(16) xDest yDest xSrc ySrc cxSrc cySrc offBmi cbBmi offBits cbBits usage rop cxDest cyDest
    fn stretch_dibits(&mut self, r: &[u8]) {
        let i = |o: usize| i32_at(r, o).unwrap_or(0);
        let u = |o: usize| u32_at(r, o).unwrap_or(0) as usize;
        let (xd, yd, xs, ys, cxs, cys) = (i(24), i(28), i(32), i(36), i(40), i(44));
        let (off_bmi, cb_bmi, off_bits, cb_bits, rop) = (u(48), u(52), u(56), u(60), u32_at(r, 68).unwrap_or(0));
        let (cxd, cyd) = (i(72), i(76));
        let (Some(bmi), Some(bits)) = (r.get(off_bmi..off_bmi + cb_bmi), r.get(off_bits..off_bits + cb_bits)) else { return };
        let Some(dib) = decode_dib(bmi, bits, (xs, ys, cxs, cys)) else { return };
        self.draw_dib(dib, [xd, yd, cxd, cyd], rop);
    }

    /// BITBLT / STRETCHBLT: bounds(16) xDest yDest cxDest cyDest rop xSrc ySrc xform(24) bkcolor usage offBmi cbBmi offBits cbBits [cxSrc cySrc]
    fn blt(&mut self, t: u32, r: &[u8]) {
        let i = |o: usize| i32_at(r, o).unwrap_or(0);
        let u = |o: usize| u32_at(r, o).unwrap_or(0) as usize;
        let (xd, yd, cxd, cyd, rop) = (i(24), i(28), i(32), i(36), u32_at(r, 40).unwrap_or(0));
        let (xs, ys) = (i(44), i(48));
        let (off_bmi, cb_bmi, off_bits, cb_bits) = (u(84), u(88), u(92), u(96));
        if cb_bmi == 0 || cb_bits == 0 { return; }
        let (Some(bmi), Some(bits)) = (r.get(off_bmi..off_bmi + cb_bmi), r.get(off_bits..off_bits + cb_bits)) else { return };
        let (cxs, cys) = if t == 77 { (i(100), i(104)) } else { (cxd, cyd) };
        let Some(dib) = decode_dib(bmi, bits, (xs, ys, cxs, cys)) else { return };
        self.draw_dib(dib, [xd, yd, cxd, cyd], rop);
    }

    fn draw_dib(&mut self, mut dib: Dib, dest: [i32; 4], rop: u32) {
        const SRCPAINT: u32 = 0xEE0086;
        const SRCAND: u32 = 0x8800C6;
        // A 1-bit mask drawn with OR or AND is half of a transparency pair; the picture follows.
        if dib.bits == 1 && (rop == SRCPAINT || rop == SRCAND) {
            self.pending_mask = Some((dib, dest, rop));
            return;
        }
        let mut paired = false;
        if let Some((mask, mdest, mrop)) = self.pending_mask.take() {
            if mdest == dest && mask.w == dib.w && mask.h == dib.h && (rop == SRCAND || rop == SRCPAINT) && rop != mrop {
                // OR-ing a mask that is white where opaque, then AND-ing the picture, keeps the picture where
                // the mask is white. AND-ing a mask that is black where opaque, then OR-ing the picture, keeps
                // it where the mask is black.
                paired = true;
                let opaque_when_white = mrop == SRCPAINT;
                for (px, m) in dib.rgba.chunks_exact_mut(4).zip(mask.rgba.chunks_exact(4)) {
                    let white = m[0] as u32 + m[1] as u32 + m[2] as u32 > 382;
                    px[3] = if white == opaque_when_white { 255 } else { 0 };
                }
            }
        }
        // Anything but a plain copy (or the half of a transparency pair) combines with what is underneath in a way we do not do.
        if !paired && rop != 0xCC0020 { self.skipped.insert("unusual bitmap operations"); }
        self.drew = true;
        let Some(mut pix) = Pixmap::new(dib.w, dib.h) else { return };
        for (dst, src) in pix.pixels_mut().iter_mut().zip(dib.rgba.chunks_exact(4)) {
            let a = src[3] as u32;
            let pm = |c: u8| ((c as u32 * a + 127) / 255) as u8;
            *dst = tiny_skia::PremultipliedColorU8::from_rgba(pm(src[0]), pm(src[1]), pm(src[2]), src[3]).unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
        }
        let [x, y, cx, cy] = dest;
        let (p0, p1, p2) = (self.pt(x as f32, y as f32), self.pt((x + cx) as f32, y as f32), self.pt(x as f32, (y + cy) as f32));
        // Source pixel (u, v) lands at p0 + u/w (p1 - p0) + v/h (p2 - p0).
        let (w, h) = (dib.w as f32, dib.h as f32);
        let ts = Transform::from_row((p1.0 - p0.0) / w, (p1.1 - p0.1) / w, (p2.0 - p0.0) / h, (p2.1 - p0.1) / h, p0.0, p0.1);
        let p3 = (p1.0 + p2.0 - p0.0, p1.1 + p2.1 - p0.1);
        let mut pb = PathBuilder::new();
        pb.move_to(p0.0, p0.1);
        pb.line_to(p1.0, p1.1);
        pb.line_to(p3.0, p3.1);
        pb.line_to(p2.0, p2.1);
        pb.close();
        let Some(quad) = pb.finish() else { return };
        let pattern = tiny_skia::Pattern::new(pix.as_ref(), tiny_skia::SpreadMode::Pad, tiny_skia::FilterQuality::Bilinear, 1.0, ts);
        let paint = Paint { shader: pattern, anti_alias: true, ..Paint::default() };
        self.out.fill_path(&quad, &paint, FillRule::Winding, Transform::identity(), self.dc.clip.as_ref());
    }
}

fn append(into: &mut PathBuilder, p: &tiny_skia::Path, joins: bool) {
    use tiny_skia::PathSegment::*;
    let mut first = true;
    for seg in p.segments() {
        match seg {
            // A continuing run starts where the path already is.
            MoveTo(_) if first && joins && !into.is_empty() => {}
            MoveTo(a) => into.move_to(a.x, a.y),
            LineTo(a) => into.line_to(a.x, a.y),
            QuadTo(a, b) => into.quad_to(a.x, a.y, b.x, b.y),
            CubicTo(a, b, c) => into.cubic_to(a.x, a.y, b.x, b.y, c.x, c.y),
            Close => into.close(),
        }
        first = false;
    }
}

fn colorref(c: u32) -> [u8; 4] {
    [(c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8, 255]
}

/// A device independent bitmap to RGBA, top row first, cut to the source rectangle `(x, y, w, h)`
/// (measured from the bottom-left for bottom-up bitmaps, as GDI does).
fn decode_dib(bmi: &[u8], bits: &[u8], src: (i32, i32, i32, i32)) -> Option<Dib> {
    let hdr = u32_at(bmi, 0)? as usize;
    if hdr < 40 { return None; }
    let (w, h_raw) = (i32_at(bmi, 4)?, i32_at(bmi, 8)?);
    let bit_count = u16::from_le_bytes(bmi.get(14..16)?.try_into().ok()?);
    let compression = u32_at(bmi, 16)?;
    let used = u32_at(bmi, 32)? as usize;
    // BI_RGB (0) and BI_BITFIELDS (3) only.
    if w <= 0 || h_raw == 0 || !matches!(compression, 0 | 3) { return None; }
    let (h, top_down) = (h_raw.unsigned_abs() as usize, h_raw < 0);
    let w = w as usize;
    let colors = if bit_count <= 8 { if used > 0 { used } else { 1usize << bit_count } } else { 0 };
    let palette_at = hdr + if compression == 3 && hdr == 40 { 12 } else { 0 };
    let palette: Vec<[u8; 3]> = (0..colors).filter_map(|k| { let o = palette_at + k * 4; Some([*bmi.get(o + 2)?, *bmi.get(o + 1)?, *bmi.get(o)?]) }).collect();
    let stride = (w * bit_count as usize).div_ceil(32) * 4;
    let masks = if compression == 3 { Some([u32_at(bmi, hdr)?, u32_at(bmi, hdr + 4)?, u32_at(bmi, hdr + 8)?]) } else { None };
    // The visible rows, as indices into the top-down image.
    let (sx, sy, sw, sh) = src;
    let sx = sx.clamp(0, w as i32) as usize;
    let sw = if sw <= 0 { w } else { sw as usize }.min(w - sx);
    let sh = if sh <= 0 { h } else { sh as usize }.min(h);
    let top = if top_down { sy.max(0) as usize } else { h.saturating_sub(sy.max(0) as usize + sh) };
    if sw == 0 || sh == 0 || sw * sh > 16_000_000 { return None; }
    let mut rgba = vec![255u8; sw * sh * 4];
    for row in 0..sh {
        let img_row = top + row;
        if img_row >= h { break; }
        let file_row = if top_down { img_row } else { h - 1 - img_row };
        let line = bits.get(file_row * stride..(file_row + 1) * stride)?;
        for col in 0..sw {
            let x = sx + col;
            let px = match bit_count {
                1 => palette.get(((line[x / 8] >> (7 - x % 8)) & 1) as usize).copied().unwrap_or([0, 0, 0]),
                4 => palette.get(((line[x / 2] >> if x % 2 == 0 { 4 } else { 0 }) & 15) as usize).copied().unwrap_or([0, 0, 0]),
                8 => palette.get(line[x] as usize).copied().unwrap_or([0, 0, 0]),
                24 => [line[x * 3 + 2], line[x * 3 + 1], line[x * 3]],
                // 16 bits are 5-5-5 unless the header lists its own channel masks.
                16 => {
                    let v = u16::from_le_bytes([line[x * 2], line[x * 2 + 1]]) as u32;
                    let mk = masks.unwrap_or([0x7C00, 0x03E0, 0x001F]);
                    let ch = |mask: u32| if mask == 0 { 0 } else { (((v & mask) >> mask.trailing_zeros()) * 255 / (mask >> mask.trailing_zeros())) as u8 };
                    [ch(mk[0]), ch(mk[1]), ch(mk[2])]
                }
                32 => match masks {
                    Some(mk) => {
                        let v = u32::from_le_bytes(line[x * 4..x * 4 + 4].try_into().ok()?);
                        let ch = |mask: u32| if mask == 0 { 0 } else { (((v & mask) >> mask.trailing_zeros()) * 255 / (mask >> mask.trailing_zeros())) as u8 };
                        [ch(mk[0]), ch(mk[1]), ch(mk[2])]
                    }
                    None => [line[x * 4 + 2], line[x * 4 + 1], line[x * 4]],
                },
                _ => return None,
            };
            let o = (row * sw + col) * 4;
            rgba[o..o + 3].copy_from_slice(&px);
        }
    }
    Some(Dib { w: sw as u32, h: sh as u32, rgba, bits: bit_count })
}

/// The extent a metafile declares in device units (left, top, right, bottom, inclusive), or `None`
/// if it is not a readable EMF.
fn header(d: &[u8]) -> Option<(i32, i32, i32, i32)> {
    if u32_at(d, 0)? != 1 || d.get(40..44)? != b" EMF" { return None; }
    Some((i32_at(d, 8)?, i32_at(d, 12)?, i32_at(d, 16)?, i32_at(d, 20)?))
}

/// The size, in device pixels, of the area a metafile declares it draws on.
pub fn device_size(d: &[u8]) -> Option<(f32, f32)> {
    let (l, t, r, b) = header(d)?;
    let (w, h) = ((r - l + 1) as f32, (b - t + 1) as f32);
    (w > 0.0 && h > 0.0).then_some((w, h))
}

/// Records that draw something this renderer does not.
fn unsupported(t: u32) -> Option<&'static str> {
    match t {
        83 | 84 | 96 | 97 | 108 => Some("text"),
        118 => Some("gradient fills"),
        41 | 45 | 46 | 47 | 55 => Some("arcs and pies"),
        71..=74 => Some("region fills"),
        29 | 93 | 94 => Some("pattern fills"),
        78 | 79 | 80 | 114 | 116 => Some("some bitmap drawing"),
        _ => None,
    }
}

/// A rendered metafile and what it asked for that was left out.
pub struct Rendered {
    pub png: Vec<u8>,
    /// Names of features the metafile used that are not drawn, or only roughly. Empty when all was drawn.
    pub skipped: Vec<&'static str>,
}

/// Draw the metafile on a transparent canvas whose longest side is `max_side` pixels (at most
/// 2400), and return it as a PNG.
pub fn render_png(d: &[u8], max_side: u32) -> Option<Vec<u8>> {
    render(d, max_side).map(|r| r.png)
}

/// Like [`render_png`], and says which features of the file were not drawn.
pub fn render(d: &[u8], max_side: u32) -> Option<Rendered> {
    let (l, t, r, b) = header(d)?;
    let (dw, dh) = ((r - l + 1) as f32, (b - t + 1) as f32);
    if dw <= 0.0 || dh <= 0.0 { return None; }
    let k = max_side.clamp(16, MAX_SIDE) as f32 / dw.max(dh);
    let (pw, ph) = ((dw * k).ceil().max(1.0) as u32, (dh * k).ceil().max(1.0) as u32);
    let mut rd = Renderer {
        out: Pixmap::new(pw, ph)?,
        to_px: Xform([k, 0.0, 0.0, k, -l as f32 * k, -t as f32 * k]),
        dc: Dc::new(),
        stack: Vec::new(),
        objects: HashMap::new(),
        path: None,
        cur: (0.0, 0.0),
        pending_mask: None,
        skipped: BTreeSet::new(),
        drew: false,
        emf_plus: false,
    };
    rd.run(d);
    // A file that is only EMF+ draws nothing through the classic records.
    if rd.emf_plus && !rd.drew { rd.skipped.insert("EMF+ drawing (a newer metafile format)"); }
    Some(Rendered { png: rd.out.encode_png().ok()?, skipped: rd.skipped.into_iter().collect() })
}

/// An empty metafile declaring a `w` x `h` canvas, for tests elsewhere in the crate.
#[cfg(test)]
pub(crate) fn blank_emf(w: i32, h: i32) -> Vec<u8> {
    let mut d = vec![0u8; 108];
    d[0..4].copy_from_slice(&1u32.to_le_bytes());
    d[4..8].copy_from_slice(&108u32.to_le_bytes());
    for (k, v) in [0, 0, w - 1, h - 1].iter().enumerate() {
        d[8 + k * 4..12 + k * 4].copy_from_slice(&(*v as i32).to_le_bytes());
    }
    d[40..44].copy_from_slice(b" EMF");
    d.extend_from_slice(&14u32.to_le_bytes());
    d.extend_from_slice(&20u32.to_le_bytes());
    d.extend_from_slice(&[0; 12]);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A builder for tiny metafiles.
    struct Emf(Vec<u8>);
    impl Emf {
        fn new(bounds: (i32, i32, i32, i32)) -> Emf {
            let mut h = vec![0u8; 108];
            h[0..4].copy_from_slice(&1u32.to_le_bytes());
            h[4..8].copy_from_slice(&108u32.to_le_bytes());
            for (k, v) in [bounds.0, bounds.1, bounds.2, bounds.3].iter().enumerate() {
                h[8 + k * 4..12 + k * 4].copy_from_slice(&v.to_le_bytes());
            }
            h[40..44].copy_from_slice(b" EMF");
            Emf(h)
        }
        fn rec(mut self, t: u32, body: &[u8]) -> Emf {
            let size = 8 + body.len() as u32;
            self.0.extend_from_slice(&t.to_le_bytes());
            self.0.extend_from_slice(&size.to_le_bytes());
            self.0.extend_from_slice(body);
            self
        }
        fn done(self) -> Vec<u8> {
            self.rec(14, &[0; 12]).0
        }
    }
    fn le(vals: &[i32]) -> Vec<u8> {
        vals.iter().flat_map(|v| v.to_le_bytes()).collect()
    }
    fn decode(png: &[u8]) -> image::RgbaImage {
        image::load_from_memory(png).unwrap().to_rgba8()
    }
    fn brush(rgb: u32) -> Vec<u8> {
        let mut b = le(&[1, 0]);
        b.extend_from_slice(&rgb.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b
    }
    /// POLYGON16 of a rectangle from x0 to x1 and the full height of a 100 x 100 canvas.
    fn square(x0: i16, x1: i16) -> Vec<u8> {
        let mut b = le(&[0, 0, 99, 99, 4]);
        b.extend([x0, 0, x1, 0, x1, 99, x0, 99].iter().flat_map(|v: &i16| v.to_le_bytes()));
        b
    }

    #[test]
    fn a_filled_path_with_a_solid_brush_is_drawn_where_it_says() {
        // A red square over the left half, built as a path (COLORREF is blue-green-red, so 0x0000FF is red).
        let emf = Emf::new((0, 0, 99, 99)).rec(39, &brush(0x0000FF)).rec(37, &le(&[1])).rec(59, &[]).rec(86, &square(0, 50)).rec(61, &[]).rec(60, &[]).rec(62, &[]).done();
        let img = decode(&render_png(&emf, 100).unwrap());
        assert_eq!(img.dimensions(), (100, 100));
        assert_eq!(img.get_pixel(20, 50).0, [255, 0, 0, 255], "inside the square");
        assert_eq!(img.get_pixel(80, 50).0[3], 0, "outside it stays transparent");
    }

    #[test]
    fn a_clip_path_limits_later_fills() {
        // Clip to the left half, then fill the whole canvas green: only the left half shows.
        let emf = Emf::new((0, 0, 99, 99))
            .rec(39, &brush(0x00FF00))
            .rec(37, &le(&[1]))
            .rec(59, &[])
            .rec(86, &square(0, 50))
            .rec(60, &[])
            .rec(67, &le(&[1]))
            .rec(59, &[])
            .rec(86, &square(0, 99))
            .rec(60, &[])
            .rec(62, &[])
            .done();
        let img = decode(&render_png(&emf, 100).unwrap());
        assert_eq!(img.get_pixel(20, 50).0, [0, 255, 0, 255]);
        assert_eq!(img.get_pixel(80, 50).0[3], 0, "clipped away");
    }

    #[test]
    fn the_world_transform_moves_and_scales_what_follows() {
        // Scale by 0.5 and shift by 50: the full-canvas square lands on the right half.
        let m: Vec<u8> = [0.5f32, 0.0, 0.0, 0.5, 50.0, 0.0].iter().flat_map(|v| v.to_bits().to_le_bytes()).collect();
        let emf = Emf::new((0, 0, 99, 99)).rec(39, &brush(0xFF0000)).rec(37, &le(&[1])).rec(35, &m).rec(59, &[]).rec(86, &square(0, 99)).rec(60, &[]).rec(62, &[]).done();
        let img = decode(&render_png(&emf, 100).unwrap());
        assert_eq!(img.get_pixel(75, 25).0, [0, 0, 255, 255], "inside the moved square");
        assert_eq!(img.get_pixel(25, 25).0[3], 0);
        assert_eq!(img.get_pixel(75, 75).0[3], 0, "the square is half the height now");
    }

    #[test]
    fn a_mask_and_picture_pair_draws_the_picture_only_where_the_mask_is_white() {
        // An 8 x 2 picture, all red, with a 1-bit mask that is white on the left four pixels.
        let dib = |bits: u16, colors: &[[u8; 4]], rows: &[u8]| -> (Vec<u8>, Vec<u8>) {
            let mut bmi = vec![0u8; 40];
            bmi[0..4].copy_from_slice(&40u32.to_le_bytes());
            bmi[4..8].copy_from_slice(&8i32.to_le_bytes());
            bmi[8..12].copy_from_slice(&2i32.to_le_bytes());
            bmi[12..14].copy_from_slice(&1u16.to_le_bytes());
            bmi[14..16].copy_from_slice(&bits.to_le_bytes());
            bmi[32..36].copy_from_slice(&(colors.len() as u32).to_le_bytes());
            for c in colors {
                bmi.extend_from_slice(c);
            }
            (bmi, rows.to_vec())
        };
        let sdib = |bmi: &[u8], bits: &[u8], rop: u32| -> Vec<u8> {
            // The record's fixed part, header included, is 80 bytes.
            let off = 80;
            let mut b = le(&[0, 0, 7, 1, 0, 0, 0, 0, 8, 2]);
            b.extend_from_slice(&(off as u32).to_le_bytes());
            b.extend_from_slice(&(bmi.len() as u32).to_le_bytes());
            b.extend_from_slice(&((off + bmi.len()) as u32).to_le_bytes());
            b.extend_from_slice(&(bits.len() as u32).to_le_bytes());
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(&rop.to_le_bytes());
            b.extend_from_slice(&le(&[8, 2]));
            b.extend_from_slice(bmi);
            b.extend_from_slice(bits);
            b
        };
        let (mbmi, mbits) = dib(1, &[[0, 0, 0, 0], [255, 255, 255, 0]], &[0xF0, 0, 0, 0, 0xF0, 0, 0, 0]);
        let (pbmi, pbits) = dib(8, &[[255, 255, 255, 0], [0, 0, 255, 0]], &[1; 16]);
        let emf = Emf::new((0, 0, 7, 1)).rec(81, &sdib(&mbmi, &mbits, 0xEE0086)).rec(81, &sdib(&pbmi, &pbits, 0x8800C6)).done();
        let img = decode(&render_png(&emf, 16).unwrap());
        assert_eq!(img.get_pixel(2, 2).0, [255, 0, 0, 255], "opaque where the mask is white");
        assert_eq!(img.get_pixel(13, 2).0[3], 0, "transparent where it is black");
    }

    #[test]
    fn something_that_is_not_a_metafile_is_refused() {
        assert!(render_png(b"not an emf at all, really not", 100).is_none());
        assert!(render_png(&[0; 200], 100).is_none());
    }

    #[test]
    fn rectangles_ellipses_and_rounded_boxes_are_drawn() {
        let draw = |t: u32, extra: &[i32]| {
            let mut body = le(&[10, 10, 90, 90]);
            body.extend(le(extra));
            let emf = Emf::new((0, 0, 99, 99)).rec(39, &brush(0x0000FF)).rec(37, &le(&[1])).rec(t, &body).done();
            decode(&render_png(&emf, 100).unwrap())
        };
        let ellipse = draw(42, &[]);
        assert_eq!(ellipse.get_pixel(50, 50).0, [255, 0, 0, 255], "the middle is filled");
        assert_eq!(ellipse.get_pixel(13, 13).0[3], 0, "the corner of the box is outside an ellipse");
        let rect = draw(43, &[]);
        assert_eq!(rect.get_pixel(13, 13).0, [255, 0, 0, 255], "a rectangle fills its corner");
        let round = draw(44, &[40, 40]);
        assert_eq!(round.get_pixel(50, 50).0, [255, 0, 0, 255]);
        assert_eq!(round.get_pixel(11, 11).0[3], 0, "a rounded corner leaves the very corner empty");
        assert_eq!(round.get_pixel(50, 12).0[3], 255, "and the straight edge is there");
    }

    #[test]
    fn what_is_not_drawn_is_reported() {
        let clean = Emf::new((0, 0, 99, 99)).rec(39, &brush(0x0000FF)).rec(37, &le(&[1])).rec(59, &[]).rec(86, &square(0, 50)).rec(61, &[]).rec(60, &[]).rec(62, &[]).done();
        assert!(render(&clean, 100).unwrap().skipped.is_empty());
        let mut hatch = brush(0x0000FF);
        hatch[4..8].copy_from_slice(&2u32.to_le_bytes());
        let messy = Emf::new((0, 0, 99, 99)).rec(39, &hatch).rec(84, &[0; 40]).rec(118, &[0; 40]).rec(45, &[0; 40]).done();
        let skipped = render(&messy, 100).unwrap().skipped;
        assert_eq!(skipped, ["arcs and pies", "gradient fills", "hatched fills (drawn solid)", "text"]);
        // An EMF+ comment alone means the picture is in a format we do not read; next to classic drawing it is just extra.
        let mut plus = le(&[16]);
        plus.extend_from_slice(b"EMF+");
        plus.extend_from_slice(&[0; 12]);
        let only = Emf::new((0, 0, 99, 99)).rec(70, &plus).done();
        assert_eq!(render(&only, 100).unwrap().skipped, ["EMF+ drawing (a newer metafile format)"]);
        let dual = Emf::new((0, 0, 99, 99)).rec(70, &plus).rec(39, &brush(0x0000FF)).rec(37, &le(&[1])).rec(43, &le(&[10, 10, 90, 90])).done();
        assert!(render(&dual, 100).unwrap().skipped.is_empty());
    }
}
