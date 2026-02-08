use std::collections::HashSet;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};

use crate::{
    deduction_tree::ProofContextHypothesis,
    protocol::NodeId,
    shared::{
        aligned_table::AlignedRow,
        diff_text::{diff_style, DiffState, RichTextExt},
        theme::Theme,
        Region, Selection,
    },
};

/// Context for rendering a hypothesis layer.
pub struct HypLayerRenderContext<'a> {
    pub selected: Option<Selection>,
    pub base_y: u16,
    pub area: Rect,
    pub depends_on: &'a HashSet<String>,
}

/// A layer of hypotheses.
#[derive(Debug, Clone)]
pub struct HypLayer {
    /// Node ID for creating selections (from DAG).
    node_id: Option<NodeId>,
    pub hypotheses: Vec<(usize, ProofContextHypothesis)>, // (hyp_idx, hyp)
}

impl Default for HypLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl HypLayer {
    pub const fn new() -> Self {
        Self {
            node_id: None,
            hypotheses: Vec::new(),
        }
    }

    pub const fn set_node_id(&mut self, node_id: Option<NodeId>) {
        self.node_id = node_id;
    }

    /// Add a hypothesis from `ProofContextHypothesis`.
    pub fn add_from_info(&mut self, hyp_idx: usize, info: &ProofContextHypothesis) {
        self.hypotheses.push((hyp_idx, info.clone()));
    }

    pub const fn len(&self) -> usize {
        self.hypotheses.len()
    }

    pub fn render(
        &self,
        ctx: &HypLayerRenderContext<'_>,
        click_regions: &mut Vec<Region>,
    ) -> Vec<AlignedRow> {
        if let Some(node_id) = self.node_id {
            self.track_click_regions(click_regions, node_id, ctx);
        }

        self.hypotheses
            .iter()
            .map(|(hyp_idx, hyp)| {
                let is_selected = matches!(
                    ctx.selected,
                    Some(Selection::Hyp { hyp_idx: hi, .. }) if hi == *hyp_idx
                );
                let is_dependency = ctx.depends_on.contains(&hyp.name);
                render_hyp_row(hyp, is_selected, is_dependency)
            })
            .collect()
    }

    fn track_click_regions(
        &self,
        click_regions: &mut Vec<Region>,
        node_id: NodeId,
        ctx: &HypLayerRenderContext<'_>,
    ) {
        click_regions.extend(
            self.hypotheses
                .iter()
                .enumerate()
                .map(|(i, (hyp_idx, _))| (ctx.base_y + i as u16, *hyp_idx))
                .take_while(|&(y, _)| y < ctx.area.y + ctx.area.height)
                .map(|(y, hyp_idx)| {
                    Region::from_rect(
                        Rect::new(ctx.area.x, y, ctx.area.width, 1),
                        Selection::Hyp { node_id, hyp_idx },
                    )
                }),
        );
    }
}

fn render_hyp_row(
    hyp: &ProofContextHypothesis,
    is_selected: bool,
    is_dependency: bool,
) -> AlignedRow {
    let state = DiffState {
        is_inserted: false,
        is_removed: hyp.is_removed,
    };
    let diff = diff_style(&state, is_selected, Color::White);

    let name_style = if is_dependency {
        Style::new()
            .fg(Theme::HYP_NAME)
            .add_modifier(Modifier::BOLD)
    } else if is_selected {
        Style::new().fg(Theme::HYP_NAME).bg(Theme::SELECTION_BG)
    } else {
        Style::new().fg(Theme::HYP_NAME)
    };

    AlignedRow::with_styled_type(
        hyp.name.clone(),
        name_style,
        ":",
        Style::new().fg(Color::DarkGray),
        hyp.type_.to_spans(diff.style),
    )
}
