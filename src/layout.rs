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

    /// Returns the axis of the split that directly contains `target`.
    ///
    /// This is useful for choosing a balanced automatic split when a widget
    /// has not been allocated yet (for example, immediately after unzooming).
    pub fn parent_split_axis(&self, target: u64) -> Option<SplitAxis> {
        fn recurse(node: &TileNode, target: u64) -> Option<SplitAxis> {
            let TileNode::Split {
                axis,
                first,
                second,
                ..
            } = node
            else {
                return None;
            };

            if matches!(**first, TileNode::Leaf(id) if id == target)
                || matches!(**second, TileNode::Leaf(id) if id == target)
            {
                return Some(*axis);
            }

            recurse(first, target).or_else(|| recurse(second, target))
        }

        self.root.as_ref().and_then(|root| recurse(root, target))
    }

    /// Flips the axis of the smallest split containing `target`.
    ///
    /// The leaves and their order are preserved, so this is a safe way to
    /// turn a row into a column (or the reverse) without moving sessions.
    pub fn toggle_parent_split_axis(&mut self, target: u64) -> bool {
        fn recurse(node: &mut TileNode, target: u64) -> bool {
            let TileNode::Split {
                axis,
                first,
                second,
                ..
            } = node
            else {
                return false;
            };

            if matches!(**first, TileNode::Leaf(id) if id == target)
                || matches!(**second, TileNode::Leaf(id) if id == target)
            {
                *axis = match *axis {
                    SplitAxis::Horizontal => SplitAxis::Vertical,
                    SplitAxis::Vertical => SplitAxis::Horizontal,
                };
                return true;
            }

            recurse(first, target) || recurse(second, target)
        }

        self.root.as_mut().is_some_and(|root| recurse(root, target))
    }

    /// Restores every divider to an even 50/50 split while keeping the tree
    /// topology and all panes intact.
    pub fn balance_ratios(&mut self) -> bool {
        fn recurse(node: &mut TileNode, changed: &mut bool) {
            if let TileNode::Split {
                ratio,
                first,
                second,
                ..
            } = node
            {
                *changed |= (*ratio - 0.5).abs() > f32::EPSILON;
                *ratio = 0.5;
                recurse(first, changed);
                recurse(second, changed);
            }
        }

        let mut changed = false;
        if let Some(root) = self.root.as_mut() {
            recurse(root, &mut changed);
        }
        changed
    }

    pub fn leaf_edge_direction(&self, target: u64) -> Option<Direction> {
        let root = self.root.as_ref()?;
        let mut path = Vec::new();
        if !find_path(root, target, &mut path) {
            return None;
        }

        let side = *path.last()?;
        let parent_path = &path[..path.len().saturating_sub(1)];
        let (_, axis, _) = split_meta_at_path(root, parent_path)?;

        match (axis, side) {
            (SplitAxis::Vertical, ChildSide::First) => Some(Direction::Left),
            (SplitAxis::Vertical, ChildSide::Second) => Some(Direction::Right),
            (SplitAxis::Horizontal, ChildSide::First) => Some(Direction::Up),
            (SplitAxis::Horizontal, ChildSide::Second) => Some(Direction::Down),
        }
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
                    // A newly-created Paned reports one or more temporary
                    // positions before it has received its real allocation.
                    // Do not persist those values: otherwise a rebuild can
                    // turn a 50/50 split into a narrow (clamped) 15/85 one.
                    let accepts_ratio_updates = Rc::new(Cell::new(false));
                    let change_callback = on_ratio_changed.clone();
                    let accepts_updates = accepts_ratio_updates.clone();
                    paned.connect_position_notify(move |widget| {
                        if !accepts_updates.get() {
                            return;
                        }

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
                    let accepts_updates = accepts_ratio_updates.clone();
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
                            accepts_updates.set(true);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn three_pane_tree() -> TileTree {
        let mut tree = TileTree::default();
        tree.set_root_leaf(1);
        tree.split_leaf_with_position(1, 2, 10, SplitAxis::Vertical, InsertPosition::After);
        tree.split_leaf_with_position(2, 3, 11, SplitAxis::Horizontal, InsertPosition::After);
        tree
    }

    fn ratios(node: &TileNode, output: &mut Vec<f32>) {
        if let TileNode::Split {
            ratio,
            first,
            second,
            ..
        } = node
        {
            output.push(*ratio);
            ratios(first, output);
            ratios(second, output);
        }
    }

    #[test]
    fn conserva_el_orden_al_insertar_antes_y_despues() {
        let mut tree = TileTree::default();
        tree.set_root_leaf(2);
        tree.split_leaf_with_position(2, 1, 10, SplitAxis::Vertical, InsertPosition::Before);
        tree.split_leaf_with_position(2, 3, 11, SplitAxis::Horizontal, InsertPosition::After);

        assert_eq!(tree.leaf_ids(), vec![1, 2, 3]);
        assert_eq!(tree.leaf_count(), 3);
    }

    #[test]
    fn rota_solo_el_contenedor_directo_del_panel() {
        let mut tree = three_pane_tree();

        assert_eq!(tree.parent_split_axis(3), Some(SplitAxis::Horizontal));
        assert!(tree.toggle_parent_split_axis(3));
        assert_eq!(tree.parent_split_axis(3), Some(SplitAxis::Vertical));
        assert_eq!(tree.parent_split_axis(1), Some(SplitAxis::Vertical));
        assert_eq!(tree.leaf_ids(), vec![1, 2, 3]);
        assert!(!tree.toggle_parent_split_axis(999));
    }

    #[test]
    fn elimina_hojas_y_colapsa_sin_perder_las_restantes() {
        let mut tree = three_pane_tree();

        assert!(tree.remove_leaf(2));
        assert_eq!(tree.leaf_ids(), vec![1, 3]);
        assert_eq!(tree.parent_split_axis(3), Some(SplitAxis::Vertical));
        assert!(tree.remove_leaf(1));
        assert_eq!(tree.leaf_ids(), vec![3]);
        assert!(!tree.remove_leaf(99));
    }

    #[test]
    fn intercambia_solo_las_hojas_solicitadas() {
        let mut tree = three_pane_tree();

        assert!(tree.swap_leaves(1, 3));
        assert_eq!(tree.leaf_ids(), vec![3, 2, 1]);
        assert!(!tree.swap_leaves(1, 1));
        assert!(!tree.swap_leaves(1, 99));
    }

    #[test]
    fn redimensiona_el_divisor_adecuado_y_respeta_limites() {
        let mut tree = three_pane_tree();

        assert!(tree.resize_leaf(3, Direction::Up, 0.2));
        assert_eq!(tree.parent_split_axis(3), Some(SplitAxis::Horizontal));
        assert!(tree.resize_leaf(1, Direction::Right, 10.0));

        let mut current_ratios = Vec::new();
        ratios(tree.root.as_ref().unwrap(), &mut current_ratios);
        assert!(
            current_ratios
                .iter()
                .all(|ratio| (0.15..=0.85).contains(ratio))
        );
        assert!(!tree.resize_leaf(1, Direction::Up, 0.1));
    }

    #[test]
    fn equilibrar_restablece_todos_los_divisores() {
        let mut tree = three_pane_tree();
        tree.update_split_ratio(10, 0.25);
        tree.update_split_ratio(11, 0.75);

        assert!(tree.balance_ratios());
        assert!(!tree.balance_ratios());
        let mut current_ratios = Vec::new();
        ratios(tree.root.as_ref().unwrap(), &mut current_ratios);
        assert_eq!(current_ratios, vec![0.5, 0.5]);
    }
}
