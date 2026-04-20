use crate::terminal_pane::TerminalPane;
use gtk::prelude::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertPosition {
    Before,
    After,
}

#[derive(Debug, Clone)]
pub enum TileNode {
    Leaf(u64),
    Split {
        split_id: u64,
        axis: SplitAxis,
        ratio: f32,
        first: Box<TileNode>,
        second: Box<TileNode>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct TileTree {
    root: Option<TileNode>,
}

impl SplitAxis {
    pub fn to_orientation(self) -> gtk::Orientation {
        match self {
            SplitAxis::Horizontal => gtk::Orientation::Vertical,
            SplitAxis::Vertical => gtk::Orientation::Horizontal,
        }
    }
}

impl TileTree {
    pub fn set_root_leaf(&mut self, pane_id: u64) {
        self.root = Some(TileNode::Leaf(pane_id));
    }

    pub fn leaf_count(&self) -> usize {
        self.leaf_ids().len()
    }

    pub fn leaf_ids(&self) -> Vec<u64> {
        fn collect(node: &TileNode, output: &mut Vec<u64>) {
            match node {
                TileNode::Leaf(id) => output.push(*id),
                TileNode::Split { first, second, .. } => {
                    collect(first, output);
                    collect(second, output);
                }
            }
        }

        let mut ids = Vec::new();
        if let Some(root) = &self.root {
            collect(root, &mut ids);
        }
        ids
    }

    pub fn first_leaf(&self) -> Option<u64> {
        self.leaf_ids().into_iter().next()
    }

    pub fn split_leaf_with_position(
        &mut self,
        target: u64,
        new_pane: u64,
        split_id: u64,
        axis: SplitAxis,
        position: InsertPosition,
    ) {
        fn recurse(
            node: &mut TileNode,
            target: u64,
            new_pane: u64,
            split_id: u64,
            axis: SplitAxis,
            position: InsertPosition,
        ) {
            match node {
                TileNode::Leaf(current) if *current == target => {
                    let (first, second) = match position {
                        InsertPosition::Before => {
                            (TileNode::Leaf(new_pane), TileNode::Leaf(target))
                        }
                        InsertPosition::After => (TileNode::Leaf(target), TileNode::Leaf(new_pane)),
                    };
                    *node = TileNode::Split {
                        split_id,
                        axis,
                        ratio: 0.5,
                        first: Box::new(first),
                        second: Box::new(second),
                    };
                }
                TileNode::Split { first, second, .. } => {
                    if contains_leaf(first, target) {
                        recurse(first, target, new_pane, split_id, axis, position);
                    } else {
                        recurse(second, target, new_pane, split_id, axis, position);
                    }
                }
                _ => {}
            }
        }

        if self.root.is_none() {
            self.root = Some(TileNode::Leaf(new_pane));
            return;
        }

        if let Some(root) = self.root.as_mut() {
            recurse(root, target, new_pane, split_id, axis, position);
        }
    }

    pub fn leaf_depth(&self, target: u64) -> Option<usize> {
        let root = self.root.as_ref()?;
        let mut path = Vec::new();
        find_path(root, target, &mut path).then_some(path.len())
    }

    pub fn remove_leaf(&mut self, target: u64) -> bool {
        fn collapse(node: &mut TileNode, target: u64) -> bool {
            match node {
                TileNode::Split { first, second, .. } => {
                    if matches!(**first, TileNode::Leaf(id) if id == target) {
                        *node = (**second).clone();
                        return true;
                    }
                    if matches!(**second, TileNode::Leaf(id) if id == target) {
                        *node = (**first).clone();
                        return true;
                    }
                    if contains_leaf(first, target) {
                        return collapse(first, target);
                    }
                    if contains_leaf(second, target) {
                        return collapse(second, target);
                    }
                    false
                }
                _ => false,
            }
        }

        if matches!(self.root, Some(TileNode::Leaf(id)) if id == target) {
            self.root = None;
            return true;
        }

        self.root
            .as_mut()
            .is_some_and(|root| collapse(root, target))
    }

    pub fn swap_leaves(&mut self, first_id: u64, second_id: u64) -> bool {
        if first_id == second_id {
            return false;
        }

        let mut swapped = 0usize;
        fn recurse(node: &mut TileNode, first_id: u64, second_id: u64, swapped: &mut usize) {
            match node {
                TileNode::Leaf(id) if *id == first_id => {
                    *id = second_id;
                    *swapped += 1;
                }
                TileNode::Leaf(id) if *id == second_id => {
                    *id = first_id;
                    *swapped += 1;
                }
                TileNode::Split { first, second, .. } => {
                    recurse(first, first_id, second_id, swapped);
                    recurse(second, first_id, second_id, swapped);
                }
                TileNode::Leaf(_) => {}
            }
        }

        if let Some(root) = self.root.as_mut() {
            recurse(root, first_id, second_id, &mut swapped);
        }

        swapped == 2
    }

    pub fn update_split_ratio(&mut self, split_id: u64, ratio: f32) {
        fn recurse(node: &mut TileNode, split_id: u64, ratio: f32) -> bool {
            match node {
                TileNode::Split {
                    split_id: current,
                    ratio: current_ratio,
                    first,
                    second,
                    ..
                } => {
                    if *current == split_id {
                        *current_ratio = ratio.clamp(0.15, 0.85);
                        return true;
                    }
                    recurse(first, split_id, ratio) || recurse(second, split_id, ratio)
                }
                TileNode::Leaf(_) => false,
            }
        }

        if let Some(root) = self.root.as_mut() {
            let _ = recurse(root, split_id, ratio);
        }
    }

    pub fn resize_leaf(&mut self, target: u64, direction: Direction, step: f32) -> bool {
        let mut path = Vec::new();
        let Some(root) = &self.root else {
            return false;
        };

        if !find_path(root, target, &mut path) {
            return false;
        }

        for depth in (0..path.len()).rev() {
            let side = path[depth];
            if let Some((split_id, axis, ratio)) = split_meta_at_path(root, &path[..depth]) {
                let new_ratio = match (axis, side, direction) {
                    (SplitAxis::Vertical, ChildSide::Second, Direction::Left) => ratio - step,
                    (SplitAxis::Vertical, ChildSide::First, Direction::Right) => ratio + step,
                    (SplitAxis::Horizontal, ChildSide::Second, Direction::Up) => ratio - step,
                    (SplitAxis::Horizontal, ChildSide::First, Direction::Down) => ratio + step,
                    _ => continue,
                };
                self.update_split_ratio(split_id, new_ratio);
                return true;
            }
        }

        false
    }

    pub fn build_widget(
        &self,
        panes: &BTreeMap<u64, Rc<TerminalPane>>,
        on_ratio_changed: Rc<dyn Fn(u64, f32)>,
    ) -> Option<gtk::Widget> {
        fn build_node(
            node: &TileNode,
            panes: &BTreeMap<u64, Rc<TerminalPane>>,
            on_ratio_changed: Rc<dyn Fn(u64, f32)>,
        ) -> Option<gtk::Widget> {
            match node {
                TileNode::Leaf(id) => panes.get(id).map(|pane| pane.widget()),
                TileNode::Split {
                    split_id,
                    axis,
                    ratio,
                    first,
                    second,
                } => {
                    let start = build_node(first, panes, on_ratio_changed.clone())?;
                    let end = build_node(second, panes, on_ratio_changed.clone())?;
                    let paned = gtk::Paned::new(axis.to_orientation());
                    paned.set_wide_handle(true);
                    paned.add_css_class("tile-paned");
                    paned.set_start_child(Some(&start));
                    paned.set_end_child(Some(&end));

                    let split = *split_id;
                    let orientation = *axis;
                    let change_callback = on_ratio_changed.clone();
                    paned.connect_position_notify(move |widget| {
                        let total = match orientation {
                            SplitAxis::Vertical => widget.width(),
                            SplitAxis::Horizontal => widget.height(),
                        };

                        if total > 0 {
                            change_callback(split, widget.position() as f32 / total as f32);
                        }
                    });

                    let target = paned.clone();
                    let position_ratio = *ratio;
                    let frames_left = Cell::new(18u8);
                    let stable_frames = Cell::new(0u8);
                    target.add_tick_callback(move |widget, _| {
                        let total = match orientation {
                            SplitAxis::Vertical => widget.width(),
                            SplitAxis::Horizontal => widget.height(),
                        };

                        if total <= 0 {
                            let remaining = frames_left.get();
                            if remaining == 0 {
                                return gtk::glib::ControlFlow::Break;
                            }
                            frames_left.set(remaining.saturating_sub(1));
                            return gtk::glib::ControlFlow::Continue;
                        }

                        let desired = (total as f32 * position_ratio).round() as i32;
                        let delta = (widget.position() - desired).abs();
                        if delta > 1 {
                            widget.set_position(desired);
                            stable_frames.set(0);
                        } else {
                            stable_frames.set(stable_frames.get().saturating_add(1));
                        }

                        let remaining = frames_left.get();
                        if remaining == 0 || stable_frames.get() >= 2 {
                            gtk::glib::ControlFlow::Break
                        } else {
                            frames_left.set(remaining.saturating_sub(1));
                            gtk::glib::ControlFlow::Continue
                        }
                    });

                    Some(paned.upcast())
                }
            }
        }

        self.root
            .as_ref()
            .and_then(|root| build_node(root, panes, on_ratio_changed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildSide {
    First,
    Second,
}

fn contains_leaf(node: &TileNode, target: u64) -> bool {
    match node {
        TileNode::Leaf(id) => *id == target,
        TileNode::Split { first, second, .. } => {
            contains_leaf(first, target) || contains_leaf(second, target)
        }
    }
}

fn find_path(node: &TileNode, target: u64, path: &mut Vec<ChildSide>) -> bool {
    match node {
        TileNode::Leaf(id) => *id == target,
        TileNode::Split { first, second, .. } => {
            path.push(ChildSide::First);
            if find_path(first, target, path) {
                return true;
            }
            path.pop();

            path.push(ChildSide::Second);
            if find_path(second, target, path) {
                return true;
            }
            path.pop();

            false
        }
    }
}

fn split_meta_at_path(node: &TileNode, path: &[ChildSide]) -> Option<(u64, SplitAxis, f32)> {
    if path.is_empty() {
        if let TileNode::Split {
            split_id,
            axis,
            ratio,
            ..
        } = node
        {
            return Some((*split_id, *axis, *ratio));
        }
        return None;
    }

    let TileNode::Split { first, second, .. } = node else {
        return None;
    };

    match path[0] {
        ChildSide::First => split_meta_at_path(first, &path[1..]),
        ChildSide::Second => split_meta_at_path(second, &path[1..]),
    }
}
