use alloc::vec;
use alloc::vec::Vec;

use src_desktop_types::{
    CursorDraw, DividerLayout, DrawCmd, EditorViewModel, FocusTarget,
    PanelLayout, PaneId, PaneKind, Rect, ScrollOffset, SplitDirection,
    TabInfo,
};

use crate::model::{collect_panes, Model, PanelNode, Pane};

const DIVIDER_SIZE: f32 = 4.0;

// ── Public entry point ────────────────────────────────────────────────────────

pub fn view(model: &Model) -> Vec<DrawCmd> {
    let mut cmds = Vec::new();
    let (w, h) = model.layout.window_size;
    let root_bounds = Rect { x: 0.0, y: 0.0, width: w as f32, height: h as f32 };

    // 1. Layout
    let mut panels: Vec<PanelLayout> = Vec::new();
    let mut dividers: Vec<DividerLayout> = Vec::new();
    collect_layout(&model.layout.root, root_bounds, &mut panels, &mut dividers);
    cmds.push(DrawCmd::SetLayout { panels: panels.clone(), dividers });

    // 2. Per-pane content
    let mut pane_vec: Vec<&Pane> = Vec::new();
    collect_panes(&model.layout.root, &mut pane_vec);

    for pane in pane_vec {
        // Find the computed bounds for this pane from the panel list
        let bounds = panels.iter()
            .find(|pl| pl.pane_id == pane.id)
            .map(|pl| pl.bounds)
            .unwrap_or(root_bounds);

        // Tab bar
        let tabs: Vec<TabInfo> = pane.tabs.iter().map(|t| TabInfo {
            doc_id: t.doc_id,
            title:  t.title.clone(),
            dirty:  t.dirty,
        }).collect();
        cmds.push(DrawCmd::SetTabBar {
            pane_id:    pane.id,
            tabs,
            active_tab: pane.active_tab,
        });

        // Content
        let active_doc = pane.tabs.get(pane.active_tab).map(|t| t.doc_id);
        match pane.kind {
            PaneKind::Editor => {
                if let Some(doc_id) = active_doc {
                    if let Some(vm) = model.workspace.editors.get(&doc_id) {
                        emit_editor_frame(&mut cmds, pane.id, bounds, vm);
                    }
                }
            }
            PaneKind::Preview => {
                if let Some(doc_id) = active_doc {
                    if let Some(ps) = model.workspace.previews.get(&doc_id) {
                        cmds.push(DrawCmd::PreviewMount {
                            pane_id: pane.id,
                            html:    ps.html.clone(),
                        });
                        cmds.push(DrawCmd::PreviewScroll {
                            pane_id:  pane.id,
                            offset_y: ps.scroll_offset,
                        });
                        let warnings: Vec<_> = ps.warnings.iter().map(|w| {
                            src_desktop_types::WarningInfo {
                                code:    w.code.clone(),
                                message: w.message.clone(),
                                line:    Some(w.line),
                            }
                        }).collect();
                        cmds.push(DrawCmd::SetWarnings { warnings });
                    }
                }
            }
            PaneKind::FileTree => {
                cmds.push(DrawCmd::SetFileTree {
                    entries:  model.file_tree.entries.clone(),
                    expanded: model.file_tree.expanded.clone(),
                });
            }
            PaneKind::PluginManager => {
                cmds.push(DrawCmd::SetPluginList {
                    plugins: model.plugin_manager.plugins.clone(),
                });
            }
        }
    }

    // 3. Status bar
    cmds.push(DrawCmd::SetStatusBar {
        left:          model.status.left.clone(),
        right:         model.status.right.clone(),
        warning_count: model.status.warning_count,
    });

    cmds
}

// ── Layout calculation ────────────────────────────────────────────────────────

fn collect_layout(
    node: &PanelNode,
    bounds: Rect,
    panels: &mut Vec<PanelLayout>,
    dividers: &mut Vec<DividerLayout>,
) {
    match node {
        PanelNode::Leaf(pane) => {
            panels.push(PanelLayout {
                pane_id: pane.id,
                bounds,
                kind:    pane.kind,
                visible: true,
            });
        }
        PanelNode::Split { direction, ratio, a, b } => {
            let (bounds_a, div_bounds, bounds_b) = split_bounds(bounds, *direction, *ratio);
            collect_layout(a, bounds_a, panels, dividers);
            collect_layout(b, bounds_b, panels, dividers);
            dividers.push(DividerLayout {
                bounds:     div_bounds,
                direction:  *direction,
                draggable:  true,
            });
        }
    }
}

fn split_bounds(bounds: Rect, direction: SplitDirection, ratio: f32) -> (Rect, Rect, Rect) {
    match direction {
        SplitDirection::Horizontal => {
            let split_x = bounds.x + bounds.width * ratio;
            let a = Rect { x: bounds.x, y: bounds.y, width: split_x - bounds.x, height: bounds.height };
            let div = Rect { x: split_x, y: bounds.y, width: DIVIDER_SIZE, height: bounds.height };
            let b = Rect { x: split_x + DIVIDER_SIZE, y: bounds.y,
                           width: bounds.width - (split_x - bounds.x) - DIVIDER_SIZE,
                           height: bounds.height };
            (a, div, b)
        }
        SplitDirection::Vertical => {
            let split_y = bounds.y + bounds.height * ratio;
            let a = Rect { x: bounds.x, y: bounds.y, width: bounds.width, height: split_y - bounds.y };
            let div = Rect { x: bounds.x, y: split_y, width: bounds.width, height: DIVIDER_SIZE };
            let b = Rect { x: bounds.x, y: split_y + DIVIDER_SIZE,
                           width: bounds.width,
                           height: bounds.height - (split_y - bounds.y) - DIVIDER_SIZE };
            (a, div, b)
        }
    }
}

// ── Editor frame ──────────────────────────────────────────────────────────────

fn emit_editor_frame(cmds: &mut Vec<DrawCmd>, pane_id: PaneId, bounds: Rect, vm: &EditorViewModel) {
    cmds.push(DrawCmd::EditorFrame {
        pane_id,
        bounds,
        lines:     vm.visible_lines.clone(),
        cursor:    CursorDraw { x: 0.0, y: 0.0, height: 16.0 },
        selection: None,
        preedit:   vm.preedit.clone(),
        scroll:    vm.scroll,
    });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use src_desktop_types::{DocId, DocMeta, Tab, VfsPath};
    use crate::model::PanelNode;

    #[test]
    fn view_empty_model_emits_set_layout() {
        let m = Model::new(800, 600);
        let cmds = view(&m);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::SetLayout { .. })));
    }

    #[test]
    fn view_emits_status_bar() {
        let m = Model::new(800, 600);
        let cmds = view(&m);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::SetStatusBar { .. })));
    }

    #[test]
    fn view_emits_tab_bar_for_each_pane() {
        let m = Model::new(800, 600);
        let cmds = view(&m);
        assert!(cmds.iter().any(|c| matches!(c, DrawCmd::SetTabBar { .. })));
    }

    #[test]
    fn horizontal_split_gives_two_panels() {
        use src_desktop_types::{PaneId, PaneKind, Rect, SplitDirection};
        use crate::model::Pane;
        use alloc::boxed::Box;
        let mut m = Model::new(1000, 600);
        m.layout.root = PanelNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            a: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(1), kind: PaneKind::Editor,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
            b: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(2), kind: PaneKind::Preview,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
        };
        let cmds = view(&m);
        let layout = cmds.iter().find_map(|c| {
            if let DrawCmd::SetLayout { panels, .. } = c { Some(panels) } else { None }
        });
        assert_eq!(layout.unwrap().len(), 2);
    }

    #[test]
    fn split_panel_bounds_sum_to_window_width() {
        use src_desktop_types::{PaneId, PaneKind, Rect, SplitDirection};
        use crate::model::Pane;
        use alloc::boxed::Box;
        let mut m = Model::new(1000, 600);
        m.layout.root = PanelNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            a: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(1), kind: PaneKind::Editor,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
            b: Box::new(PanelNode::Leaf(Pane {
                id: PaneId(2), kind: PaneKind::Preview,
                tabs: alloc::vec![], active_tab: 0,
                bounds: Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
            })),
        };
        let cmds = view(&m);
        let panels = cmds.iter().find_map(|c| {
            if let DrawCmd::SetLayout { panels, .. } = c { Some(panels) } else { None }
        }).unwrap();
        let total_panel_w: f32 = panels.iter().map(|p| p.bounds.width).sum();
        let total_div_w: f32 = cmds.iter().find_map(|c| {
            if let DrawCmd::SetLayout { dividers, .. } = c {
                Some(dividers.iter().map(|d| d.bounds.width).sum())
            } else { None }
        }).unwrap_or(0.0);
        assert!((total_panel_w + total_div_w - 1000.0_f32).abs() < 1.0);
    }
}
