use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget, Wrap},
};

pub enum ConnectionHint<'a> {
    SearchingWorkers {
        port_min: u16,
        port_max: u16,
    },
    WorkerStopped {
        file_name: &'a str,
        port_min: u16,
        port_max: u16,
    },
    AwaitingCursor {
        file_name: Option<&'a str>,
        editor_name: Option<&'a str>,
        other_worker_count: usize,
    },
    AwaitingGraph {
        file_name: &'a str,
        line_number: u64,
    },
}

enum KnownEditor {
    Helix,
    Neovim,
    VisualStudioCode,
    Zed,
    Emacs,
}

impl KnownEditor {
    fn from_client_name(client_name: &str) -> Option<Self> {
        let client_name = client_name.to_lowercase();
        if client_name.contains("helix") {
            Some(Self::Helix)
        } else if client_name.contains("neovim") || client_name.contains("nvim") {
            Some(Self::Neovim)
        } else if client_name.contains("code") || client_name.contains("codium") {
            Some(Self::VisualStudioCode)
        } else if client_name.contains("zed") {
            Some(Self::Zed)
        } else if client_name.contains("emacs") || client_name.contains("eglot") {
            Some(Self::Emacs)
        } else {
            None
        }
    }

    const fn cursor_request_instruction(&self) -> &'static str {
        match self {
            Self::Helix => {
                "Press `space k` (hover) in normal mode, or `ctrl-x` (completion) in insert mode, \
                 on the code you want to see. Moving the cursor alone sends nothing."
            }
            Self::Neovim => {
                "Press `K` (hover) on the code you want to see. With lean.nvim the infoview also \
                 updates the graph on every cursor move."
            }
            Self::VisualStudioCode => {
                "Move the cursor into a proof or definition: the Lean 4 extension asks for the \
                 goal at every cursor move."
            }
            Self::Zed => "Hover the mouse over the code you want to see.",
            Self::Emacs => {
                "Leave the cursor on the code you want to see: eldoc asks for hover documentation \
                 after a short delay."
            }
        }
    }
}

fn cursor_request_instruction(editor_name: Option<&str>) -> String {
    editor_name.map_or_else(
        || {
            "Ask your editor for hover documentation or completion on the code you want to see."
                .to_string()
        },
        |name| {
            KnownEditor::from_client_name(name).map_or_else(
                || {
                    format!(
                        "Ask {name} for hover documentation or completion on the code you want to \
                         see."
                    )
                },
                |editor| editor.cursor_request_instruction().to_string(),
            )
        },
    )
}

fn worker_start_instruction() -> &'static str {
    "Open a Lean file in an editor that runs `lean-prism serve` as its Lean language server, or in \
     a project that imports LeanPrism. Each open file gets its own worker."
}

impl ConnectionHint<'_> {
    fn title(&self) -> String {
        match self {
            Self::SearchingWorkers { .. } => " lean-tui: waiting for Lean-Prism ".to_string(),
            Self::WorkerStopped { file_name, .. } => format!(" lean-tui: {file_name} closed "),
            Self::AwaitingCursor {
                file_name: Some(file_name),
                ..
            }
            | Self::AwaitingGraph { file_name, .. } => format!(" lean-tui ({file_name}) "),
            Self::AwaitingCursor {
                file_name: None, ..
            } => " lean-tui ".to_string(),
        }
    }

    fn headline_and_details(&self) -> (String, Vec<String>) {
        match self {
            Self::SearchingWorkers { port_min, port_max } => (
                format!("No Lean-Prism worker on 127.0.0.1:{port_min}–{port_max} yet."),
                vec![worker_start_instruction().to_string()],
            ),
            Self::WorkerStopped {
                file_name,
                port_min,
                port_max,
            } => (
                format!("The worker for {file_name} stopped."),
                vec![
                    "The editor closed the file or restarted the Lean language server.".to_string(),
                    format!("Looking for workers on 127.0.0.1:{port_min}–{port_max}."),
                    worker_start_instruction().to_string(),
                ],
            ),
            Self::AwaitingCursor {
                file_name,
                editor_name,
                other_worker_count,
            } => {
                let headline = match (file_name, editor_name) {
                    (Some(file), Some(editor)) => {
                        format!("Connected to Lean-Prism for {file} in {editor}.")
                    }
                    (Some(file), None) => format!("Connected to Lean-Prism for {file}."),
                    (None, _) => "Connected to Lean-Prism.".to_string(),
                };
                let mut details = vec![cursor_request_instruction(*editor_name)];
                if *other_worker_count > 0 {
                    details.push(format!(
                        "{other_worker_count} other open file(s) have a worker too. Press `f` to \
                         pick one."
                    ));
                }
                (headline, details)
            }
            Self::AwaitingGraph {
                file_name,
                line_number,
            } => (
                format!("Lean is still checking {file_name} up to line {line_number}."),
                vec![
                    "The graph appears once Lean has elaborated the code at the cursor."
                        .to_string(),
                ],
            ),
        }
    }
}

impl Widget for ConnectionHint<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title(self.title())
            .title_alignment(Alignment::Center)
            .border_style(Style::new().fg(Color::Reset));
        let inner = block.inner(area);
        block.render(area, buf);

        let (headline, details) = self.headline_and_details();
        let mut lines = vec![
            Line::styled(headline, Style::new().add_modifier(Modifier::BOLD)),
            Line::default(),
        ];
        for detail in details {
            lines.push(Line::styled(detail, Style::new().fg(Color::DarkGray)));
            lines.push(Line::default());
        }

        Paragraph::new(Text::from(lines))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

    use super::ConnectionHint;

    fn rendered_text(hint: ConnectionHint<'_>) -> String {
        let area = Rect::new(0, 0, 200, 20);
        let mut buffer = Buffer::empty(area);
        hint.render(area, &mut buffer);
        (0..area.height)
            .map(|row| {
                (0..area.width)
                    .map(|column| buffer[(column, row)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn searching_workers_names_port_range_and_serve_command() {
        let text = rendered_text(ConnectionHint::SearchingWorkers {
            port_min: 9742,
            port_max: 9842,
        });
        assert!(text.contains("127.0.0.1:9742–9842"));
        assert!(text.contains("lean-prism serve"));
    }

    #[test]
    fn stopped_worker_names_its_file() {
        let text = rendered_text(ConnectionHint::WorkerStopped {
            file_name: "Basic.lean",
            port_min: 9742,
            port_max: 9842,
        });
        assert!(text.contains("The worker for Basic.lean stopped."));
    }

    #[test]
    fn helix_gets_hover_and_completion_keys() {
        let text = rendered_text(ConnectionHint::AwaitingCursor {
            file_name: Some("Basic.lean"),
            editor_name: Some("helix"),
            other_worker_count: 2,
        });
        assert!(text.contains("Connected to Lean-Prism for Basic.lean in helix."));
        assert!(text.contains("space k"));
        assert!(text.contains("2 other open file(s)"));
    }

    #[test]
    fn unknown_editor_is_named_in_generic_instruction() {
        let text = rendered_text(ConnectionHint::AwaitingCursor {
            file_name: Some("Basic.lean"),
            editor_name: Some("kakoune-lsp"),
            other_worker_count: 0,
        });
        assert!(text.contains("Ask kakoune-lsp for hover documentation"));
        assert!(!text.contains("other open file"));
    }

    #[test]
    fn awaiting_graph_shows_one_based_line() {
        let text = rendered_text(ConnectionHint::AwaitingGraph {
            file_name: "Basic.lean",
            line_number: 7,
        });
        assert!(text.contains("Lean is still checking Basic.lean up to line 7."));
    }
}
