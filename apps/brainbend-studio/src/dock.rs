//! The layout of the panels: tab groups.
//!
//! The editor has two panel edges, the right and the bottom. An edge holds lanes, and a lane holds groups, and a group holds tabs.
//! On the right edge the lanes are columns side by side and the groups in a lane are rows stacked top to bottom; on the bottom
//! edge the lanes are rows stacked top to bottom and the groups in a lane are columns side by side. A tab can also be a window
//! of its own, outside the layout. Tabs are numbers (see `main.rs`: the panels are 0 to 2 and 20 to 22, Slide Layers is `LAYERS`,
//! a tool is 10 and up).
//!
//! Nothing here draws anything: it is the plan the panels are drawn from, and where the person has moved things to.
use std::collections::HashMap;

/// The Slide Layers panel's number.
pub const LAYERS: usize = 30;

/// The slide list's number.
pub const SLIDES: usize = 31;

/// The Project settings panel's number: it shows beside the Scene View and the Table of Contents.
pub const PROJECT: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Edge { Right, Bottom, Left, Top }

impl Edge {
    pub const ALL: [Edge; 4] = [Edge::Right, Edge::Bottom, Edge::Left, Edge::Top];
    /// At the left and right edges the lanes are columns side by side and the groups in a lane are rows; at the top and bottom it is the other way round.
    pub fn columns(self) -> bool { matches!(self, Edge::Right | Edge::Left) }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Zone { Center, Left, Right, Above, Below }

/// Where a dragged tab is let go.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Spot {
    /// Beside or in a group: the middle adds the tab to it, an edge makes a new group or lane next to it.
    At { group: u32, zone: Zone },
    /// The outer end of an edge: a new lane there (a new column at the far right, a new row at the very bottom).
    EdgeEnd(Edge),
}

impl Default for Spot { fn default() -> Self { Spot::EdgeEnd(Edge::Right) } }

#[derive(Clone, Debug, PartialEq)]
pub struct Group { pub id: u32, pub tabs: Vec<usize>, pub current: usize, pub share: f32 }

#[derive(Clone, Debug, PartialEq)]
pub struct Lane { pub id: u32, pub groups: Vec<Group>, pub share: f32 }

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub right: Vec<Lane>,
    pub bottom: Vec<Lane>,
    pub left: Vec<Lane>,
    pub top: Vec<Lane>,
    /// The tabs that are windows of their own.
    pub window: Vec<usize>,
    next: u32,
}

impl Default for Layout {
    /// Properties, Triggers and AI at the right with Slide Layers under them; Timeline, Slide Notes and States at the bottom.
    fn default() -> Self {
        let mut l = Layout { right: vec![], bottom: vec![], left: vec![], top: vec![], window: vec![], next: 1 };
        l.left = l.slide_list();
        let main = l.group(vec![0, 1, 2], 0.772);
        let layers = l.group(vec![LAYERS], 0.228);
        let lane = l.lane(vec![main, layers]);
        l.right.push(lane);
        let low = l.group(vec![20, 21, 22], 1.0);
        let lane = l.lane(vec![low]);
        l.bottom.push(lane);
        l
    }
}

/// A ready-made arrangement for a kind of screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Preset {
    /// Properties at the right with Slide Layers under it, the Timeline at the bottom, the slide list at the left.
    Standard,
    /// For wide screens: two columns at the right (Properties and Slide Layers, then the Timeline over Slide Notes and States), nothing at the bottom.
    Wide,
    /// For tall screens: nothing at the right, and three columns at the bottom (Properties, the Timeline with Notes and States, Slide Layers).
    Tall,
}

impl Preset {
    pub const ALL: [Preset; 3] = [Preset::Standard, Preset::Wide, Preset::Tall];
    pub fn name(self) -> &'static str { match self { Preset::Standard => "Standard", Preset::Wide => "Wide screen", Preset::Tall => "Tall screen" } }
    pub fn about(self) -> &'static str {
        match self {
            Preset::Standard => "Properties at the right with Slide Layers under it, the Timeline at the bottom",
            Preset::Wide => "Two columns at the right, so the slide keeps its height: Properties and Slide Layers, then the Timeline, Slide Notes and States",
            Preset::Tall => "Everything in rows of columns at the bottom, so the slide keeps its width: Properties, the Timeline, Slide Layers",
        }
    }
}

impl Layout {
    /// The layout of a preset (with no windows; the caller adds the tools back).
    pub fn preset(p: Preset) -> Layout {
        match p {
            Preset::Standard => Layout::default(),
            Preset::Wide => {
                let mut l = Layout { right: vec![], bottom: vec![], left: vec![], top: vec![], window: vec![], next: 1 };
                let main = l.group(vec![0, 1, 2], 0.7);
                let layers = l.group(vec![LAYERS], 0.3);
                let first = l.lane(vec![main, layers]);
                let time = l.group(vec![20], 0.5);
                let low = l.group(vec![21, 22], 0.5);
                let second = l.lane(vec![time, low]);
                l.right = vec![first, second];
                l.left = l.slide_list();
                l
            }
            Preset::Tall => {
                let mut l = Layout { right: vec![], bottom: vec![], left: vec![], top: vec![], window: vec![], next: 1 };
                let main = l.group(vec![0, 1, 2], 0.3);
                let low = l.group(vec![20, 21, 22], 0.4);
                let layers = l.group(vec![LAYERS], 0.3);
                let lane = l.lane(vec![main, low, layers]);
                l.bottom = vec![lane];
                l.left = l.slide_list();
                l
            }
        }
    }

    /// The slide list on its own at the left, as it has always been.
    /// The left edge starts with the slide list, and the Project Settings behind it: they never show together (the slide list belongs to a slide's view,
    /// the Project Settings to the Scene View and the Table of Contents), so the one group serves both.
    fn slide_list(&mut self) -> Vec<Lane> { let g = self.group(vec![SLIDES, PROJECT], 1.0); vec![self.lane(vec![g])] }

    fn id(&mut self) -> u32 { self.next += 1; self.next }
    fn group(&mut self, tabs: Vec<usize>, share: f32) -> Group { let current = tabs.first().copied().unwrap_or(0); Group { id: self.id(), tabs, current, share } }
    fn lane(&mut self, groups: Vec<Group>) -> Lane { Lane { id: self.id(), groups, share: 1.0 } }

    pub fn lanes(&self, edge: Edge) -> &Vec<Lane> { match edge { Edge::Right => &self.right, Edge::Bottom => &self.bottom, Edge::Left => &self.left, Edge::Top => &self.top } }
    pub fn lanes_mut(&mut self, edge: Edge) -> &mut Vec<Lane> { match edge { Edge::Right => &mut self.right, Edge::Bottom => &mut self.bottom, Edge::Left => &mut self.left, Edge::Top => &mut self.top } }
    fn all_lanes_mut(&mut self) -> impl Iterator<Item = &mut Lane> { self.right.iter_mut().chain(self.bottom.iter_mut()).chain(self.left.iter_mut()).chain(self.top.iter_mut()) }

    /// Every group with the edge it is on, in drawing order.
    pub fn groups(&self) -> Vec<(Edge, &Group)> {
        Edge::ALL.into_iter().flat_map(|e| self.lanes(e).iter().flat_map(move |l| l.groups.iter().map(move |g| (e, g)))).collect()
    }

    /// The edge a tab is docked on, or `None` when it is a window (or not placed).
    pub fn edge_of(&self, tab: usize) -> Option<Edge> { self.groups().into_iter().find(|(_, g)| g.tabs.contains(&tab)).map(|(e, _)| e) }

    pub fn group_of(&self, tab: usize) -> Option<&Group> { self.groups().into_iter().map(|(_, g)| g).find(|g| g.tabs.contains(&tab)) }

    pub fn group_mut(&mut self, id: u32) -> Option<&mut Group> {
        self.all_lanes_mut().flat_map(|l| l.groups.iter_mut()).find(|g| g.id == id)
    }

    /// Make a tab the one showing in its group.
    pub fn show(&mut self, tab: usize) {
        if let Some(id) = self.group_of(tab).map(|g| g.id) { if let Some(g) = self.group_mut(id) { g.current = tab; } }
    }

    /// Take a tab out of wherever it is, and tidy up the groups and lanes that are left empty.
    pub fn remove(&mut self, tab: usize) {
        self.window.retain(|t| *t != tab);
        for lane in self.all_lanes_mut() {
            for g in lane.groups.iter_mut() {
                g.tabs.retain(|t| *t != tab);
                if g.current == tab { g.current = g.tabs.first().copied().unwrap_or(0); }
            }
        }
        self.tidy();
    }

    fn tidy(&mut self) {
        for edge in Edge::ALL {
            let lanes = self.lanes_mut(edge);
            for l in lanes.iter_mut() { l.groups.retain(|g| !g.tabs.is_empty()); }
            lanes.retain(|l| !l.groups.is_empty());
        }
    }

    /// Make the tab a window of its own.
    pub fn pop_out(&mut self, tab: usize) {
        self.remove(tab);
        self.window.push(tab);
    }

    /// Dock a tab at the end of an edge's first group (making one if the edge is empty).
    pub fn dock(&mut self, tab: usize, edge: Edge) {
        self.remove(tab);
        if self.lanes(edge).is_empty() {
            let g = self.group(vec![tab], 1.0);
            let lane = self.lane(vec![g]);
            self.lanes_mut(edge).push(lane);
            return;
        }
        let lane = &mut self.lanes_mut(edge)[0];
        let g = &mut lane.groups[0];
        g.tabs.push(tab);
        g.current = tab;
    }

    /// Put a tab where it was let go. Letting a group's only tab go beside that same group changes nothing.
    pub fn place(&mut self, tab: usize, spot: Spot) {
        match spot {
            Spot::EdgeEnd(edge) => {
                self.remove(tab);
                let g = self.group(vec![tab], 1.0);
                let lane = self.lane(vec![g]);
                self.lanes_mut(edge).push(lane);
            }
            Spot::At { group, zone } => {
                let Some((edge, li, gi)) = self.locate(group) else { return };
                let own = self.group_of(tab).map(|g| g.id);
                if own == Some(group) && (zone == Zone::Center || self.group_of(tab).is_some_and(|g| g.tabs.len() == 1)) { return; }
                if zone == Zone::Center {
                    self.remove(tab);
                    // The group may have moved if the tab's removal tidied lanes away.
                    if let Some((e, l, g)) = self.locate(group) { let g = &mut self.lanes_mut(e)[l].groups[g]; g.tabs.push(tab); g.current = tab; }
                    return;
                }
                // New groups beside a group in its lane; new lanes beside its lane. Which is which depends on the edge.
                let in_lane = if edge.columns() { matches!(zone, Zone::Above | Zone::Below) } else { matches!(zone, Zone::Left | Zone::Right) };
                let after = matches!(zone, Zone::Right | Zone::Below);
                let (lane_id, group_id) = (self.lanes(edge)[li].id, group);
                let _ = (gi, lane_id);
                let new = self.group(vec![tab], 1.0);
                if in_lane {
                    let lane = &mut self.lanes_mut(edge)[li];
                    let at = lane.groups.iter().position(|g| g.id == group_id).unwrap_or(0) + after as usize;
                    lane.groups.insert(at, new);
                } else {
                    let lane = self.lane(vec![new]);
                    let at = li + after as usize;
                    self.lanes_mut(edge).insert(at, lane);
                }
                // Now take the tab out of its old place (not the new one).
                let new_id = self.groups().into_iter().map(|(_, g)| g).find(|g| g.tabs == vec![tab] && Some(g.id) != own).map(|g| g.id);
                for lane in self.all_lanes_mut() {
                    for g in lane.groups.iter_mut() {
                        if Some(g.id) != new_id { g.tabs.retain(|t| *t != tab); if g.current == tab { g.current = g.tabs.first().copied().unwrap_or(0); } }
                    }
                }
                self.window.retain(|t| *t != tab);
                self.tidy();
            }
        }
    }

    /// Where a group is: its edge, its lane's place and its own place in the lane.
    pub fn locate(&self, group: u32) -> Option<(Edge, usize, usize)> {
        for edge in Edge::ALL {
            for (li, l) in self.lanes(edge).iter().enumerate() {
                if let Some(gi) = l.groups.iter().position(|g| g.id == group) { return Some((edge, li, gi)); }
            }
        }
        None
    }

    /// Make sure every tab in `all` is somewhere: the ones the layout has never heard of go where they start.
    pub fn include(&mut self, all: &[usize], start: &HashMap<usize, Edge>) {
        for &t in all {
            if self.window.contains(&t) || self.group_of(t).is_some() { continue; }
            match start.get(&t) { Some(&e) => self.dock(t, e), None => self.window.push(t) }
        }
    }

    // ----- saving -----

    pub fn to_json(&self) -> String {
        use serde_json::{json, Value};
        let lanes = |ls: &Vec<Lane>| -> Value {
            Value::Array(ls.iter().map(|l| json!({ "share": l.share, "groups": l.groups.iter().map(|g| json!({ "tabs": g.tabs, "current": g.current, "share": g.share })).collect::<Vec<_>>() })).collect())
        };
        json!({ "right": lanes(&self.right), "bottom": lanes(&self.bottom), "left": lanes(&self.left), "top": lanes(&self.top), "window": self.window }).to_string()
    }

    pub fn from_json(text: &str) -> Option<Layout> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let mut l = Layout { right: vec![], bottom: vec![], left: vec![], top: vec![], window: vec![], next: 1 };
        for (edge, key) in [(Edge::Right, "right"), (Edge::Bottom, "bottom"), (Edge::Left, "left"), (Edge::Top, "top")] {
            // Layouts saved before the left and top edges existed have no such keys.
            let empty = vec![];
            let lanes_in: &Vec<serde_json::Value> = if matches!(edge, Edge::Left | Edge::Top) { v.get(key).and_then(|k| k.as_array()).unwrap_or(&empty) } else { v.get(key)?.as_array()? };
            for lane in lanes_in {
                let mut groups = vec![];
                for g in lane.get("groups")?.as_array()? {
                    let tabs: Vec<usize> = g.get("tabs")?.as_array()?.iter().filter_map(|n| n.as_u64().map(|n| n as usize)).collect();
                    if tabs.is_empty() { continue; }
                    let mut gr = l.group(tabs, g.get("share").and_then(|s| s.as_f64()).unwrap_or(1.0) as f32);
                    if let Some(c) = g.get("current").and_then(|c| c.as_u64()).map(|c| c as usize) { if gr.tabs.contains(&c) { gr.current = c; } }
                    groups.push(gr);
                }
                if groups.is_empty() { continue; }
                let mut ln = l.lane(groups);
                ln.share = lane.get("share").and_then(|s| s.as_f64()).unwrap_or(1.0) as f32;
                l.lanes_mut(edge).push(ln);
            }
        }
        l.window = v.get("window")?.as_array()?.iter().filter_map(|n| n.as_u64().map(|n| n as usize)).collect();
        Some(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn right_groups(l: &Layout) -> Vec<Vec<Vec<usize>>> { l.right.iter().map(|ln| ln.groups.iter().map(|g| g.tabs.clone()).collect()).collect() }
    fn bottom_groups(l: &Layout) -> Vec<Vec<Vec<usize>>> { l.bottom.iter().map(|ln| ln.groups.iter().map(|g| g.tabs.clone()).collect()).collect() }

    #[test]
    fn the_default_is_the_layout_the_app_always_had() {
        let l = Layout::default();
        assert_eq!(right_groups(&l), vec![vec![vec![0, 1, 2], vec![LAYERS]]], "Properties, Triggers and AI, with Slide Layers under them");
        assert_eq!(bottom_groups(&l), vec![vec![vec![20, 21, 22]]]);
        assert_eq!(l.edge_of(1), Some(Edge::Right));
        assert_eq!(l.edge_of(21), Some(Edge::Bottom));
    }

    #[test]
    fn a_tab_dropped_beside_a_group_makes_a_row_or_a_column_depending_on_the_edge() {
        let mut l = Layout::default();
        let main = l.group_of(0).unwrap().id;
        // At the right edge, below a group is a new row in the same column.
        l.place(20, Spot::At { group: main, zone: Zone::Below });
        assert_eq!(right_groups(&l), vec![vec![vec![0, 1, 2], vec![20], vec![LAYERS]]]);
        assert_eq!(bottom_groups(&l), vec![vec![vec![21, 22]]]);
        // Beside it is a new column.
        l.place(21, Spot::At { group: main, zone: Zone::Right });
        assert_eq!(right_groups(&l), vec![vec![vec![0, 1, 2], vec![20], vec![LAYERS]], vec![vec![21]]]);
        // At the bottom edge it is the other way round: beside a group is a new column in the row, below it is a new row.
        let low = l.group_of(22).unwrap().id;
        l.place(2, Spot::At { group: low, zone: Zone::Left });
        assert_eq!(bottom_groups(&l), vec![vec![vec![2], vec![22]]]);
        l.place(1, Spot::At { group: low, zone: Zone::Below });
        assert_eq!(bottom_groups(&l), vec![vec![vec![2], vec![22]], vec![vec![1]]]);
        assert_eq!(right_groups(&l), vec![vec![vec![0], vec![20], vec![LAYERS]], vec![vec![21]]]);
    }

    #[test]
    fn the_middle_adds_a_tab_to_the_group_and_the_outer_end_of_an_edge_makes_a_lane() {
        let mut l = Layout::default();
        let layers = l.group_of(LAYERS).unwrap().id;
        l.place(21, Spot::At { group: layers, zone: Zone::Center });
        assert_eq!(l.group_of(21).unwrap().tabs, vec![LAYERS, 21]);
        assert_eq!(l.group_of(21).unwrap().current, 21, "the tab put there is the one showing");
        l.place(22, Spot::EdgeEnd(Edge::Right));
        assert_eq!(right_groups(&l), vec![vec![vec![0, 1, 2], vec![LAYERS, 21]], vec![vec![22]]]);
        // Empty groups and lanes go: moving the last tab out of the bottom leaves no bottom lane.
        l.place(20, Spot::EdgeEnd(Edge::Right));
        assert!(l.bottom.is_empty(), "nothing is left at the bottom");
        l.place(20, Spot::EdgeEnd(Edge::Bottom));
        assert_eq!(bottom_groups(&l), vec![vec![vec![20]]], "and it can be brought back");
    }

    #[test]
    fn a_groups_only_tab_let_go_beside_its_own_group_changes_nothing() {
        let mut l = Layout::default();
        let layers = l.group_of(LAYERS).unwrap().id;
        let before = l.clone();
        l.place(LAYERS, Spot::At { group: layers, zone: Zone::Left });
        assert_eq!(l, before);
        l.place(LAYERS, Spot::At { group: layers, zone: Zone::Center });
        assert_eq!(l, before);
        // A tab in a group of several can be split out of it into a new group beside it.
        let main = l.group_of(0).unwrap().id;
        l.place(2, Spot::At { group: main, zone: Zone::Above });
        assert_eq!(right_groups(&l), vec![vec![vec![2], vec![0, 1], vec![LAYERS]]]);
    }

    #[test]
    fn windows_are_remembered_and_new_tabs_go_where_they_start() {
        let mut l = Layout::default();
        l.pop_out(1);
        assert_eq!(l.window, vec![1]);
        assert_eq!(l.edge_of(1), None);
        l.dock(1, Edge::Bottom);
        assert_eq!(l.edge_of(1), Some(Edge::Bottom));
        assert!(l.window.is_empty());
        // A tab the layout has not heard of (from a newer version) goes where it starts; one with no start is a window.
        let mut start = HashMap::new();
        start.insert(41, Edge::Right);
        l.include(&[41, 40], &start);
        assert_eq!(l.edge_of(41), Some(Edge::Right));
        assert_eq!(l.window, vec![40]);
    }

    #[test]
    fn the_layout_survives_being_saved() {
        let mut l = Layout::default();
        let main = l.group_of(0).unwrap().id;
        l.place(20, Spot::At { group: main, zone: Zone::Right });
        l.pop_out(2);
        l.show(21);
        let back = Layout::from_json(&l.to_json()).expect("reads back");
        assert_eq!(right_groups(&back), right_groups(&l));
        assert_eq!(bottom_groups(&back), bottom_groups(&l));
        assert_eq!(back.window, l.window);
        assert_eq!(back.group_of(21).unwrap().current, 21);
        assert!(Layout::from_json("not json").is_none());
    }

    #[test]
    fn the_left_and_top_edges_work_like_the_right_and_bottom() {
        let mut l = Layout::default();
        l.place(1, Spot::EdgeEnd(Edge::Left));
        l.place(21, Spot::EdgeEnd(Edge::Top));
        assert_eq!(l.edge_of(1), Some(Edge::Left));
        assert_eq!(l.edge_of(21), Some(Edge::Top));
        assert_eq!(l.right.len(), 1, "the right edge keeps the others");
        // On the left the lanes are columns, so below a group is a new row; on the top they are rows, so beside a group is a new column.
        let one = l.group_of(1).unwrap().id;
        l.place(2, Spot::At { group: one, zone: Zone::Below });
        assert_eq!(l.left.len(), 2, "the slide list's column and the new one");
        assert_eq!(l.left[1].groups.iter().map(|g| g.tabs.clone()).collect::<Vec<_>>(), vec![vec![1], vec![2]]);
        let top = l.group_of(21).unwrap().id;
        l.place(22, Spot::At { group: top, zone: Zone::Right });
        assert_eq!(l.top.len(), 1);
        assert_eq!(l.top[0].groups.len(), 2);
        // Both come back after a restart, and a layout saved before these edges existed still loads.
        assert_eq!(Layout::from_json(&l.to_json()).unwrap().to_json(), l.to_json());
        let old = r#"{"right":[{"share":1.0,"groups":[{"tabs":[0],"current":0,"share":1.0}]}],"bottom":[],"window":[3]}"#;
        let o = Layout::from_json(old).unwrap();
        assert!(o.left.is_empty() && o.top.is_empty() && o.edge_of(0) == Some(Edge::Right));
        // Moving the last tab away tidies the edge.
        let mut l = Layout::default();
        l.place(1, Spot::EdgeEnd(Edge::Left));
        l.place(1, Spot::EdgeEnd(Edge::Right));
        assert_eq!(l.left.len(), 1, "only the slide list is left there");
    }
}
