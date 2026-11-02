//! What the timeline's object names share with the rest of the editor: the icon for each kind, and moving an
//! object to a new z-position. (The names themselves are drawn in `timeline_panel`.)
use super::*;

pub(super) fn kind_icon(o: &Object) -> char {
    match &o.kind {
        Kind::Text => ribbon::TEXT_T,
        Kind::Image => ribbon::IMAGE,
        Kind::Video => ribbon::VIDEO,
        Kind::Audio => ribbon::SPEAKER,
        Kind::Ellipse => ribbon::CIRCLE,
        Kind::Line | Kind::LineUp => ribbon::LINE,
        Kind::Hotspot => ribbon::CLICK,
        Kind::Path(_) => ribbon::SHAPES,
        Kind::Slider(_) => ribbon::SLIDERS,
        Kind::Web(_) => ribbon::GLOBE,
        Kind::Pano(_) => ribbon::PANORAMA,
        Kind::Input(_) => ribbon::TEXTBOX,
        _ => if o.check.is_some() { ribbon::CHECK } else if o.has_button_states() { ribbon::CLICK } else { ribbon::RECT },
    }
}

impl App {
    /// Move object `src` of layer `li` to z-position `dst`.
    pub(super) fn reorder_object(&mut self, li: usize, src: usize, dst: usize) {
        let Some(layer) = self.project.as_ref().and_then(|p| p.slides.get(self.slide)).and_then(|s| s.layers.get(li)) else { return };
        if src >= layer.objects.len() || dst >= layer.objects.len() || src == dst { return; }
        self.snapshot();
        let layer = &mut self.project.as_mut().unwrap().slides[self.slide].layers[li];
        let o = layer.objects.remove(src);
        let id = o.id.clone();
        layer.objects.insert(dst, o);
        self.selected = Some((li, layer.objects.iter().position(|x| x.id == id).unwrap_or(dst)));
        self.multi.clear();
        self.dirty = true;
    }
}
