//! Tables: a grid of rectangle objects that share a `group` and carry their (row, column).
//! Because cells are ordinary objects they draw, edit, export and animate like any other;
//! these helpers only build the grid and keep it a grid when rows or columns come and go.
//! A merged cell is one object with a `table_span`; the cells it covers simply do not exist.
use crate::{Align, Kind, Object, Rgba, Stroke, Theme, VAlign};

/// A table look: where the colors go. `None` fills and strokes are left out.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TableStyle {
    Accent,
    Light,
    Dark,
    Grid,
    Minimal,
}

impl TableStyle {
    pub const ALL: [TableStyle; 5] = [TableStyle::Accent, TableStyle::Light, TableStyle::Dark, TableStyle::Grid, TableStyle::Minimal];

    pub fn label(self) -> &'static str {
        match self {
            TableStyle::Accent => "Accent header",
            TableStyle::Light => "Light",
            TableStyle::Dark => "Dark",
            TableStyle::Grid => "Grid",
            TableStyle::Minimal => "Minimal",
        }
    }
}

/// Which parts of a style apply.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct StyleOptions {
    pub header_row: bool,
    pub banded_rows: bool,
    pub first_column: bool,
}

impl Default for StyleOptions {
    fn default() -> Self { StyleOptions { header_row: true, banded_rows: true, first_column: false } }
}

/// The colors of a style.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Look {
    pub header: Option<Rgba>,
    pub header_text: Rgba,
    pub band: Option<Rgba>,
    pub body: Option<Rgba>,
    pub text: Rgba,
    pub stroke: Option<Stroke>,
}

pub fn look(style: TableStyle, theme: &Theme) -> Look {
    let white = Rgba(255, 255, 255, 255);
    let ink = Rgba(33, 33, 33, 255);
    let accent = theme.color(4);
    let gray = |v: u8| Rgba(v, v, v, 255);
    let line = |c: Rgba, w: f32| Some(Stroke { color: c, width: w });
    match style {
        TableStyle::Accent => Look {
            header: Some(accent), header_text: white,
            band: Some(Rgba(accent.0 / 8 + 224, accent.1 / 8 + 224, accent.2 / 8 + 224, 255)), body: Some(white), text: ink, stroke: line(white, 2.0),
        },
        TableStyle::Light => Look { header: Some(gray(232)), header_text: ink, band: Some(gray(246)), body: Some(white), text: ink, stroke: line(gray(205), 1.5) },
        TableStyle::Dark => Look { header: Some(gray(20)), header_text: white, band: Some(gray(58)), body: Some(gray(44)), text: white, stroke: line(gray(80), 1.5) },
        TableStyle::Grid => Look { header: None, header_text: ink, band: None, body: None, text: ink, stroke: line(ink, 1.5) },
        TableStyle::Minimal => Look { header: None, header_text: accent, band: None, body: None, text: ink, stroke: line(gray(215), 1.0) },
    }
}

/// Restyle every cell of table `group`. Text, sizes and merges are kept.
pub fn apply_style(objects: &mut [Object], group: &str, style: TableStyle, opts: StyleOptions, theme: &Theme) {
    let l = look(style, theme);
    for o in objects.iter_mut().filter(|o| o.group.as_deref() == Some(group)) {
        let Some((r, c)) = o.table else { continue };
        let header = opts.header_row && r == 0;
        let body_row = r.saturating_sub(opts.header_row as u16);
        o.fill = if header { l.header } else if opts.banded_rows && body_row % 2 == 0 { l.band.or(l.body) } else { l.body };
        o.stroke = l.stroke;
        o.table_borders = None;
        o.style.color = if header { l.header_text } else { l.text };
        o.style.bold = header || (opts.first_column && c == 0);
        o.kind = Kind::Rect;
    }
}

/// A new table of `rows` x `cols` cells filling the box at (`x`, `y`) of size `w` x `h`.
/// The first row is a header in the theme's first accent color; the rest are banded.
pub fn new_table(group: &str, rows: u16, cols: u16, (x, y, w, h): (f32, f32, f32, f32), theme: &Theme, new_id: &mut dyn FnMut() -> String) -> Vec<Object> {
    let (cw, rh) = (w / cols as f32, h / rows as f32);
    let mut out = vec![];
    for r in 0..rows {
        for c in 0..cols {
            let mut o = Object::new(&new_id(), &format!("Cell {},{}", r + 1, c + 1), Kind::Rect, x + c as f32 * cw, y + r as f32 * rh, cw, rh);
            o.group = Some(group.to_string());
            o.table = Some((r, c));
            o.margins = [14.0, 8.0, 14.0, 8.0];
            o.style.size = 32.0;
            o.style.valign = VAlign::Middle;
            o.style.align = Align::Left;
            out.push(o);
        }
    }
    apply_style(&mut out, group, TableStyle::Accent, StyleOptions::default(), theme);
    out
}

/// The border of each side of a cell: left, right, top, bottom.
pub fn sides(o: &Object) -> [Option<Stroke>; 4] {
    o.table_borders.unwrap_or([o.stroke; 4])
}

/// Store a cell's borders; four equal sides are kept as the plain `stroke`.
fn store_sides(o: &mut Object, b: [Option<Stroke>; 4]) {
    if b.iter().all(|s| *s == b[0]) {
        o.stroke = b[0];
        o.table_borders = None;
    } else {
        o.table_borders = Some(b);
    }
}

/// Which borders of a block of cells to change.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BorderSet {
    All,
    Outside,
    Inside,
    Left,
    Right,
    Top,
    Bottom,
}

/// Give the borders `set` of the block of cells (row0, col0, row1, col1) the `line` (`None` removes them).
/// A border shared with a cell next to the block changes on both sides so the two agree.
pub fn set_borders(objects: &mut [Object], group: &str, (r0, c0, r1, c1): (u16, u16, u16, u16), set: BorderSet, line: Option<Stroke>) {
    use BorderSet::*;
    let (outer, inner) = (matches!(set, All | Outside), matches!(set, All | Inside));
    for o in objects.iter_mut().filter(|o| o.group.as_deref() == Some(group) && o.table.is_some()) {
        let (or0, or1, oc0, oc1) = (start(o, Ax::Row), end(o, Ax::Row), start(o, Ax::Col), end(o, Ax::Col));
        let inside = or0 >= r0 && or1 <= r1 && oc0 >= c0 && oc1 <= c1;
        let (rows, cols) = (or0 <= r1 && or1 >= r0, oc0 <= c1 && oc1 >= c0);
        let mut b = sides(o);
        let mut want = [false; 4];
        if inside {
            want[0] = if oc0 == c0 { outer || set == Left } else { inner };
            want[1] = if oc1 == c1 { outer || set == Right } else { inner };
            want[2] = if or0 == r0 { outer || set == Top } else { inner };
            want[3] = if or1 == r1 { outer || set == Bottom } else { inner };
        } else {
            want[1] = rows && oc1 + 1 == c0 && (outer || set == Left);
            want[0] = rows && oc0 == c1 + 1 && (outer || set == Right);
            want[3] = cols && or1 + 1 == r0 && (outer || set == Top);
            want[2] = cols && or0 == r1 + 1 && (outer || set == Bottom);
        }
        if !want.iter().any(|w| *w) { continue; }
        for k in 0..4 { if want[k] { b[k] = line; } }
        store_sides(o, b);
    }
}

/// Rows or columns.
#[derive(Clone, Copy, PartialEq)]
enum Ax {
    Row,
    Col,
}

impl Ax {
    fn other(self) -> Ax { if self == Ax::Row { Ax::Col } else { Ax::Row } }
}

fn start(o: &Object, ax: Ax) -> u16 { let t = o.table.unwrap_or((0, 0)); if ax == Ax::Row { t.0 } else { t.1 } }
fn span(o: &Object, ax: Ax) -> u16 { let s = o.table_span.unwrap_or((1, 1)); if ax == Ax::Row { s.0 } else { s.1 }.max(1) }
fn end(o: &Object, ax: Ax) -> u16 { start(o, ax) + span(o, ax) - 1 }
fn off(o: &Object, ax: Ax) -> f32 { if ax == Ax::Row { o.y } else { o.x } }
fn len(o: &Object, ax: Ax) -> f32 { if ax == Ax::Row { o.h } else { o.w } }
fn set_start(o: &mut Object, ax: Ax, v: u16) {
    let t = o.table.unwrap_or((0, 0));
    o.table = Some(if ax == Ax::Row { (v, t.1) } else { (t.0, v) });
}
fn set_span(o: &mut Object, ax: Ax, v: u16) {
    let s = o.table_span.unwrap_or((1, 1));
    let s = if ax == Ax::Row { (v, s.1) } else { (s.0, v) };
    o.table_span = if s == (1, 1) { None } else { Some(s) };
}
fn set_off(o: &mut Object, ax: Ax, v: f32) { if ax == Ax::Row { o.y = v } else { o.x = v } }
fn set_len(o: &mut Object, ax: Ax, v: f32) { if ax == Ax::Row { o.h = v } else { o.w = v } }

/// Number of rows and columns of table `group`.
pub fn dims(objects: &[Object], group: &str) -> (u16, u16) {
    let mut d = (0, 0);
    for o in objects.iter().filter(|o| o.group.as_deref() == Some(group) && o.table.is_some()) {
        d = (d.0.max(end(o, Ax::Row) + 1), d.1.max(end(o, Ax::Col) + 1));
    }
    d
}

/// Indices of the cells of table `group`.
fn cells(objects: &[Object], group: &str) -> Vec<usize> {
    (0..objects.len()).filter(|&i| objects[i].group.as_deref() == Some(group) && objects[i].table.is_some()).collect()
}

/// Positions of the lines between rows (or columns), including both outer edges.
/// A line no cell touches (inside a merged block) is spread evenly between its neighbours.
fn edges(objects: &[Object], group: &str, ax: Ax) -> Vec<f32> {
    let n = match ax { Ax::Row => dims(objects, group).0, Ax::Col => dims(objects, group).1 } as usize;
    let mut e: Vec<Option<f32>> = vec![None; n + 1];
    for &i in &cells(objects, group) {
        let o = &objects[i];
        e[start(o, ax) as usize] = Some(off(o, ax));
        e[end(o, ax) as usize + 1] = Some(off(o, ax) + len(o, ax));
    }
    let mut out = vec![0.0; n + 1];
    let mut k = 0;
    while k <= n {
        if let Some(v) = e[k] { out[k] = v; k += 1; continue; }
        let a = k - 1; // the edge before always exists: edge 0 is set by the first cell
        let b = (k..=n).find(|&j| e[j].is_some()).unwrap_or(n);
        let (va, vb) = (out[a], e[b].unwrap_or(out[a]));
        for j in k..b { out[j] = va + (vb - va) * (j - a) as f32 / (b - a) as f32; }
        k = b;
    }
    out
}

/// Positions of the inner borders of table `group`: (x of each column border, y of each row border).
/// Border `i` lies between column (or row) `i` and `i + 1`.
pub fn borders(objects: &[Object], group: &str) -> (Vec<f32>, Vec<f32>) {
    let inner = |ax| { let e = edges(objects, group, ax); if e.len() > 2 { e[1..e.len() - 1].to_vec() } else { vec![] } };
    (inner(Ax::Col), inner(Ax::Row))
}

/// Positions of every line of table `group` including the outer edges: (column lines, row lines).
pub fn grid_edges(objects: &[Object], group: &str) -> (Vec<f32>, Vec<f32>) {
    (edges(objects, group, Ax::Col), edges(objects, group, Ax::Row))
}

/// (rows, columns) a cell covers.
pub fn cell_span(o: &Object) -> (u16, u16) {
    o.table_span.unwrap_or((1, 1))
}

/// Smallest a column or row may be dragged to.
pub const MIN_SIZE: f32 = 24.0;

/// Drag inner border `index` of table `group` by `delta`. A column border trades width between the
/// two columns beside it (the table keeps its size); a row border changes that row's height and
/// pushes the rows below it. Returns the delta actually applied.
pub fn drag_border(objects: &mut [Object], group: &str, vertical: bool, index: u16, delta: f32) -> f32 {
    let ax = if vertical { Ax::Col } else { Ax::Row };
    let e = edges(objects, group, ax);
    let i = index as usize;
    if i + 2 > e.len() - 1 + usize::from(!vertical) { return 0.0; }
    let mut d = delta.max(MIN_SIZE - (e[i + 1] - e[i]));
    if vertical { d = d.min((e[i + 2] - e[i + 1]) - MIN_SIZE); }
    for o in objects.iter_mut().filter(|o| o.group.as_deref() == Some(group) && o.table.is_some()) {
        let (s, t) = (start(o, ax), end(o, ax));
        let l = len(o, ax);
        if vertical {
            if t == index { set_len(o, ax, l + d); } else if s == index + 1 { let x = off(o, ax); set_off(o, ax, x + d); set_len(o, ax, l - d); }
        } else if s <= index && t >= index {
            set_len(o, ax, l + d);
        } else if s > index {
            let y = off(o, ax);
            set_off(o, ax, y + d);
        }
    }
    d
}

/// Add a row (or column) beside row `at`. New cells copy the look of the cells in `at`, with empty
/// text; merged cells that cross the new line just grow.
fn insert(objects: &mut Vec<Object>, group: &str, ax: Ax, at: u16, after: bool, new_id: &mut dyn FnMut() -> String) {
    let e = edges(objects, group, ax);
    let oe = edges(objects, group, ax.other());
    let at_i = at as usize;
    if at_i + 1 >= e.len() { return; }
    let n = if after { at + 1 } else { at };
    let size = e[at_i + 1] - e[at_i];
    // A new line under the header looks like a body line.
    let style_at = if after && at == 0 && e.len() > 2 { 1 } else { at };
    let idx = cells(objects, group);
    let other_n = oe.len() - 1;
    let mut covered = vec![false; other_n];
    let mut sources: Vec<Option<Object>> = vec![None; other_n];
    for &i in &idx {
        let o = &objects[i];
        let (s, t) = (start(o, ax), end(o, ax));
        let (os, ot) = (start(o, ax.other()) as usize, end(o, ax.other()) as usize);
        if s < n && t >= n { for k in os..=ot.min(other_n - 1) { covered[k] = true; } }
        if s <= style_at && style_at <= t { for k in os..=ot.min(other_n - 1) { sources[k] = Some(o.clone()); } }
    }
    for &i in &idx {
        let o = &mut objects[i];
        let (s, t) = (start(o, ax), end(o, ax));
        let (p, l) = (off(o, ax), len(o, ax));
        if s >= n { set_start(o, ax, s + 1); set_off(o, ax, p + size); } else if t >= n { let sp = span(o, ax); set_span(o, ax, sp + 1); set_len(o, ax, l + size); }
    }
    for k in 0..other_n {
        if covered[k] { continue; }
        let Some(mut o) = sources[k].take() else { continue };
        o.id = new_id();
        o.text = String::new();
        o.rich = None;
        o.table_span = None;
        set_start(&mut o, ax, n);
        set_off(&mut o, ax, e[n as usize]);
        set_len(&mut o, ax, size);
        set_start(&mut o, ax.other(), k as u16);
        set_off(&mut o, ax.other(), oe[k]);
        set_len(&mut o, ax.other(), oe[k + 1] - oe[k]);
        objects.push(o);
    }
    rename(objects, group);
}

/// Remove row (or column) `at`; merged cells that cross it shrink. A table keeps at least one of each.
fn delete(objects: &mut Vec<Object>, group: &str, ax: Ax, at: u16) {
    let e = edges(objects, group, ax);
    if e.len() <= 2 || at as usize + 1 >= e.len() { return; }
    let size = e[at as usize + 1] - e[at as usize];
    objects.retain(|o| !(o.group.as_deref() == Some(group) && o.table.is_some() && start(o, ax) == at && span(o, ax) == 1));
    for o in objects.iter_mut().filter(|o| o.group.as_deref() == Some(group) && o.table.is_some()) {
        let (s, t) = (start(o, ax), end(o, ax));
        if s > at {
            set_start(o, ax, s - 1);
            let p = off(o, ax);
            set_off(o, ax, p - size);
        } else if s <= at && t >= at {
            let (sp, l) = (span(o, ax), len(o, ax));
            set_span(o, ax, sp - 1);
            set_len(o, ax, l - size);
        }
    }
    rename(objects, group);
}

/// Add a row next to row `at` (below it, or above).
pub fn insert_row(objects: &mut Vec<Object>, group: &str, at: u16, below: bool, new_id: &mut dyn FnMut() -> String) {
    insert(objects, group, Ax::Row, at, below, new_id);
}

/// Add a column next to column `at` (right of it, or left).
pub fn insert_col(objects: &mut Vec<Object>, group: &str, at: u16, right: bool, new_id: &mut dyn FnMut() -> String) {
    insert(objects, group, Ax::Col, at, right, new_id);
}

/// Remove row `at` (a table keeps at least one row).
pub fn delete_row(objects: &mut Vec<Object>, group: &str, at: u16) {
    delete(objects, group, Ax::Row, at);
}

/// Remove column `at` (a table keeps at least one column).
pub fn delete_col(objects: &mut Vec<Object>, group: &str, at: u16) {
    delete(objects, group, Ax::Col, at);
}

/// The cell covering grid position (`r`, `c`).
pub fn cell_at(objects: &[Object], group: &str, r: u16, c: u16) -> Option<usize> {
    cells(objects, group).into_iter().find(|&i| start(&objects[i], Ax::Row) <= r && r <= end(&objects[i], Ax::Row) && start(&objects[i], Ax::Col) <= c && c <= end(&objects[i], Ax::Col))
}

/// Indices of the cells of table `group` that lie in the block (row0, col0, row1, col1).
pub fn cells_in(objects: &[Object], group: &str, (r0, c0, r1, c1): (u16, u16, u16, u16)) -> Vec<usize> {
    cells(objects, group).into_iter().filter(|&i| {
        let o = &objects[i];
        start(o, Ax::Row) <= r1 && end(o, Ax::Row) >= r0 && start(o, Ax::Col) <= c1 && end(o, Ax::Col) >= c0
    }).collect()
}

/// The block of cells between corners `a` and `b`, grown until it cuts no merged cell: (row0, col0, row1, col1).
pub fn expand_range(objects: &[Object], group: &str, a: (u16, u16), b: (u16, u16)) -> (u16, u16, u16, u16) {
    let (mut r0, mut r1, mut c0, mut c1) = (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
    loop {
        let before = (r0, r1, c0, c1);
        for &i in &cells(objects, group) {
            let o = &objects[i];
            let (or0, or1, oc0, oc1) = (start(o, Ax::Row), end(o, Ax::Row), start(o, Ax::Col), end(o, Ax::Col));
            if or0 <= r1 && or1 >= r0 && oc0 <= c1 && oc1 >= c0 {
                r0 = r0.min(or0); r1 = r1.max(or1); c0 = c0.min(oc0); c1 = c1.max(oc1);
            }
        }
        if (r0, r1, c0, c1) == before { return (r0, c0, r1, c1); }
    }
}

/// Screen box (x, y, w, h) of a block of cells.
pub fn range_rect(objects: &[Object], group: &str, (r0, c0, r1, c1): (u16, u16, u16, u16)) -> Option<(f32, f32, f32, f32)> {
    let (ys, xs) = (edges(objects, group, Ax::Row), edges(objects, group, Ax::Col));
    let (x0, x1, y0, y1) = (*xs.get(c0 as usize)?, *xs.get(c1 as usize + 1)?, *ys.get(r0 as usize)?, *ys.get(r1 as usize + 1)?);
    Some((x0, y0, x1 - x0, y1 - y0))
}

/// Merge the block of cells between `a` and `b` into one cell holding all their text.
/// Returns the merged block's top-left position, or `None` when it is a single cell.
pub fn merge(objects: &mut Vec<Object>, group: &str, a: (u16, u16), b: (u16, u16)) -> Option<(u16, u16)> {
    let (r0, c0, r1, c1) = expand_range(objects, group, a, b);
    if (r0, c0) == (r1, c1) { return None; }
    let (x, y, w, h) = range_rect(objects, group, (r0, c0, r1, c1))?;
    let inside = |o: &Object| o.group.as_deref() == Some(group) && o.table.is_some_and(|(r, c)| r >= r0 && r <= r1 && c >= c0 && c <= c1);
    let mut order: Vec<usize> = (0..objects.len()).filter(|&i| inside(&objects[i])).collect();
    order.sort_by_key(|&i| objects[i].table);
    let base = *order.first()?;
    let extra: Vec<String> = order[1..].iter().filter(|&&i| !objects[i].text.is_empty()).map(|&i| objects[i].text.clone()).collect();
    let keep = objects[base].id.clone();
    // The merged cell's right and bottom borders are those of the far corner cells.
    let borders = if order.iter().any(|&i| objects[i].table_borders.is_some()) {
        let side = |r: u16, c: u16, k: usize| cell_at(objects, group, r, c).map(|i| sides(&objects[i])[k]).unwrap_or(None);
        Some([side(r0, c0, 0), side(r0, c1, 1), side(r0, c0, 2), side(r1, c0, 3)])
    } else { None };
    let b = &mut objects[base];
    if let Some(bs) = borders { store_sides(b, bs); }
    if !extra.is_empty() {
        b.text = std::iter::once(b.text.clone()).filter(|t| !t.is_empty()).chain(extra).collect::<Vec<_>>().join("\n");
        b.rich = None;
    }
    (b.x, b.y, b.w, b.h) = (x, y, w, h);
    b.table = Some((r0, c0));
    b.table_span = Some((r1 - r0 + 1, c1 - c0 + 1));
    objects.retain(|o| o.id == keep || !inside(o));
    rename(objects, group);
    Some((r0, c0))
}

/// Undo a merge: the merged cell becomes a block of cells again (the text stays in the first).
pub fn split(objects: &mut Vec<Object>, group: &str, at: (u16, u16), new_id: &mut dyn FnMut() -> String) -> bool {
    let Some(i) = cells(objects, group).into_iter().find(|&i| objects[i].table == Some(at) && objects[i].table_span.is_some()) else { return false };
    let (rs, cs) = objects[i].table_span.unwrap_or((1, 1));
    let (ys, xs) = (edges(objects, group, Ax::Row), edges(objects, group, Ax::Col));
    let proto = objects[i].clone();
    for dr in 0..rs {
        for dc in 0..cs {
            let (r, c) = (at.0 + dr, at.1 + dc);
            let (x, y) = (xs[c as usize], ys[r as usize]);
            let (w, h) = (xs[c as usize + 1] - x, ys[r as usize + 1] - y);
            if dr == 0 && dc == 0 {
                let o = &mut objects[i];
                o.table_span = None;
                (o.x, o.y, o.w, o.h) = (x, y, w, h);
                continue;
            }
            let mut o = proto.clone();
            o.id = new_id();
            o.text = String::new();
            o.rich = None;
            o.table = Some((r, c));
            o.table_span = None;
            (o.x, o.y, o.w, o.h) = (x, y, w, h);
            objects.push(o);
        }
    }
    rename(objects, group);
    true
}

/// Keep cell names current.
fn rename(objects: &mut [Object], group: &str) {
    for o in objects.iter_mut().filter(|o| o.group.as_deref() == Some(group)) {
        if let Some((r, c)) = o.table { o.name = format!("Cell {},{}", r + 1, c + 1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Vec<Object> {
        let mut n = 0;
        new_table("t", 3, 2, (100.0, 100.0, 600.0, 300.0), &Theme::default(), &mut || { n += 1; format!("c{n}") })
    }

    fn at(objs: &[Object], r: u16, c: u16) -> &Object {
        objs.iter().find(|o| o.table == Some((r, c))).unwrap_or_else(|| panic!("no cell {r},{c}"))
    }

    #[test]
    fn a_new_table_is_a_grid_with_a_header_row() {
        let t = table();
        assert_eq!(t.len(), 6);
        assert_eq!(dims(&t, "t"), (3, 2));
        assert_eq!((at(&t, 1, 1).x, at(&t, 1, 1).y), (400.0, 200.0));
        assert!(at(&t, 0, 0).style.bold && !at(&t, 1, 0).style.bold);
        assert_ne!(at(&t, 1, 0).fill, at(&t, 2, 0).fill, "rows are banded");
    }

    #[test]
    fn dragging_borders_resizes_columns_and_rows() {
        let mut t = table(); // 3 rows of 100, 2 columns of 300
        assert_eq!(borders(&t, "t"), (vec![400.0], vec![200.0, 300.0]));
        assert_eq!(drag_border(&mut t, "t", true, 0, 50.0), 50.0);
        assert_eq!((at(&t, 1, 0).w, at(&t, 1, 1).x, at(&t, 1, 1).w), (350.0, 450.0, 250.0));
        assert_eq!(drag_border(&mut t, "t", true, 0, 1000.0), 250.0 - MIN_SIZE, "stops at the minimum width");
        assert_eq!(at(&t, 0, 1).x + at(&t, 0, 1).w, 700.0, "table width unchanged");
        assert_eq!(drag_border(&mut t, "t", false, 0, 20.0), 20.0);
        assert_eq!((at(&t, 0, 0).h, at(&t, 1, 0).y, at(&t, 2, 0).y), (120.0, 220.0, 320.0));
        assert_eq!(drag_border(&mut t, "t", false, 0, -500.0), MIN_SIZE - 120.0);
    }

    #[test]
    fn rows_and_columns_can_be_added_and_removed_and_stay_a_grid() {
        let mut t = table();
        let mut n = 100;
        let mut id = || { n += 1; format!("n{n}") };
        t[2].text = "keep".into(); // cell (1,0)
        insert_row(&mut t, "t", 0, true, &mut id);
        assert_eq!(dims(&t, "t"), (4, 2));
        assert_eq!(at(&t, 2, 0).text, "keep", "the old row moved down");
        assert_eq!(at(&t, 2, 0).y, 300.0);
        assert!(!at(&t, 1, 0).style.bold, "a row under the header is a body row");
        insert_col(&mut t, "t", 1, true, &mut id);
        assert_eq!(dims(&t, "t"), (4, 3));
        assert_eq!(at(&t, 0, 2).x, 700.0);
        delete_row(&mut t, "t", 1);
        assert_eq!(dims(&t, "t"), (3, 3));
        assert_eq!(at(&t, 1, 0).text, "keep");
        assert_eq!(at(&t, 1, 0).y, 200.0, "rows below closed the gap");
        delete_col(&mut t, "t", 0);
        assert_eq!(dims(&t, "t"), (3, 2));
        assert_eq!(at(&t, 0, 0).x, 100.0);
        // The last row and column cannot be removed.
        while dims(&t, "t").0 > 1 { delete_row(&mut t, "t", 0); }
        delete_row(&mut t, "t", 0);
        assert_eq!(dims(&t, "t").0, 1);
        let mut ids: Vec<&str> = t.iter().map(|o| o.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), t.len(), "ids stay unique");
    }

    #[test]
    fn cells_merge_split_and_survive_row_and_column_edits() {
        let mut t = table(); // 3 rows x 2 columns: columns 100..400..700, rows 100..200..300..400
        let mut n = 100;
        let mut id = || { n += 1; format!("n{n}") };
        for o in t.iter_mut() { o.text = format!("{},{}", o.table.unwrap().0, o.table.unwrap().1); }
        assert_eq!(merge(&mut t, "t", (1, 0), (2, 0)), Some((1, 0)));
        assert_eq!(t.len(), 5);
        let m = at(&t, 1, 0);
        assert_eq!((m.y, m.h, m.table_span), (200.0, 200.0, Some((2, 1))));
        assert_eq!(m.text, "1,0\n2,0", "text of merged cells is kept");
        assert_eq!(dims(&t, "t"), (3, 2));
        assert_eq!(merge(&mut t, "t", (0, 0), (0, 0)), None, "a single cell cannot merge");
        // A range that cuts the merged cell grows to include it.
        assert_eq!(expand_range(&t, "t", (0, 0), (1, 1)), (0, 0, 2, 1));
        // A row inserted through the merged cell makes it taller instead of adding a cell to it.
        insert_row(&mut t, "t", 1, true, &mut id);
        assert_eq!(dims(&t, "t"), (4, 2));
        assert_eq!(at(&t, 1, 0).table_span, Some((3, 1)));
        assert_eq!(at(&t, 1, 0).h, 300.0);
        assert_eq!(t.len(), 6, "only the column without a merged cell got a new cell");
        delete_row(&mut t, "t", 2);
        assert_eq!(at(&t, 1, 0).table_span, Some((2, 1)));
        assert_eq!(at(&t, 1, 0).h, 200.0);
        // Dragging the border through a merged block keeps it covering both rows.
        drag_border(&mut t, "t", false, 1, 10.0);
        assert_eq!(at(&t, 1, 0).h, 210.0);
        // Splitting gives the block back as separate cells.
        assert!(split(&mut t, "t", (1, 0), &mut id));
        assert_eq!(dims(&t, "t"), (3, 2));
        assert_eq!(t.len(), 6);
        assert_eq!(at(&t, 1, 0).text, "1,0\n2,0");
        assert_eq!(at(&t, 2, 0).text, "");
        assert_eq!(at(&t, 2, 0).y, at(&t, 1, 0).y + at(&t, 1, 0).h);
        // Deleting a column through a wide merged cell shrinks it; a 1-column span goes back to a plain cell.
        merge(&mut t, "t", (0, 0), (0, 1));
        assert_eq!(at(&t, 0, 0).w, 600.0);
        delete_col(&mut t, "t", 1);
        assert_eq!(at(&t, 0, 0).table_span, None);
        assert_eq!(at(&t, 0, 0).w, 300.0);
    }

    #[test]
    fn styles_recolor_cells_and_keep_text_and_merges() {
        let mut t = table();
        t[0].text = "keep".into();
        merge(&mut t, "t", (1, 0), (1, 1));
        let theme = Theme::default();
        apply_style(&mut t, "t", TableStyle::Dark, StyleOptions::default(), &theme);
        assert_eq!(at(&t, 0, 0).fill, look(TableStyle::Dark, &theme).header);
        assert_eq!(at(&t, 0, 0).text, "keep");
        assert_eq!(at(&t, 1, 0).table_span, Some((1, 2)));
        assert_eq!(at(&t, 1, 0).style.color, Rgba(255, 255, 255, 255));
        apply_style(&mut t, "t", TableStyle::Grid, StyleOptions { header_row: false, banded_rows: false, first_column: true }, &theme);
        assert!(at(&t, 0, 0).fill.is_none() && at(&t, 0, 0).style.bold, "first column bold, no header fill");
        assert!(!at(&t, 2, 1).style.bold);
        assert!(at(&t, 2, 1).stroke.is_some());
    }

    #[test]
    fn borders_can_be_set_per_side_and_agree_across_a_shared_edge() {
        let mut t = table(); // 3 rows x 2 columns, white 2 px lines
        let black = Some(Stroke { color: Rgba(0, 0, 0, 255), width: 4.0 });
        let white = Some(Stroke { color: Rgba(255, 255, 255, 255), width: 2.0 });
        set_borders(&mut t, "t", (0, 0, 0, 1), BorderSet::Bottom, black);
        assert_eq!(sides(at(&t, 0, 0)), [white, white, white, black]);
        assert_eq!(sides(at(&t, 1, 1))[2], black, "the cell below shares that line");
        assert_eq!(sides(at(&t, 2, 0)), [white; 4], "other cells are untouched");
        // Everything on one cell: the four sides become the plain outline again.
        set_borders(&mut t, "t", (2, 0, 2, 0), BorderSet::All, black);
        assert!(at(&t, 2, 0).table_borders.is_none() && at(&t, 2, 0).stroke == black);
        assert_eq!(sides(at(&t, 1, 0))[3], black, "its neighbour above follows");
        // Outside of a block leaves the lines between its cells alone; Inside only changes those.
        set_borders(&mut t, "t", (1, 0, 2, 1), BorderSet::Outside, None);
        assert_eq!(sides(at(&t, 1, 0))[1], white, "inner line between (1,0) and (1,1)");
        assert_eq!(sides(at(&t, 1, 0))[0], None);
        assert_eq!(sides(at(&t, 0, 0))[3], None, "the row above shares the block's top edge");
        set_borders(&mut t, "t", (1, 0, 2, 1), BorderSet::Inside, black);
        assert_eq!(sides(at(&t, 1, 0))[1], black);
        assert_eq!(sides(at(&t, 1, 0))[0], None);
        // Merging keeps the far corner's right and bottom borders; restyling resets everything.
        merge(&mut t, "t", (1, 0), (1, 1));
        assert_eq!(sides(at(&t, 1, 0))[1], None);
        apply_style(&mut t, "t", TableStyle::Light, StyleOptions::default(), &Theme::default());
        assert!(t.iter().all(|o| o.table_borders.is_none()));
    }
}
