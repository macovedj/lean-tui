//! Interactive widget traits extending ratatui's `StatefulWidget`.

pub use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{layout::Rect, widgets::StatefulWidget, Frame};

use crate::{selection::Selection, widgets::legend::LegendSection};

/// Extension trait for `StatefulWidget` that adds state management and event
/// handling.
///
/// This trait extends ratatui's `StatefulWidget` with methods for updating
/// state and handling events, providing a complete interactive widget pattern.
///
/// # Example
///
/// ```ignore
/// struct MyWidgetState { value: i32 }
/// struct MyWidget;
///
/// impl StatefulWidget for MyWidget {
///     type State = MyWidgetState;
///     fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) { /* ... */ }
/// }
///
/// impl InteractiveStatefulWidget for MyWidget {
///     type Input = i32;
///     type Event = KeyEvent;
///
///     fn update_state(state: &mut Self::State, input: Self::Input) {
///         state.value = input;
///     }
///
///     fn handle_event(state: &mut Self::State, event: Self::Event) -> bool {
///         // handle event, return true if consumed
///         false
///     }
/// }
/// ```
pub trait InteractiveStatefulWidget: StatefulWidget {
    /// Input data type for updating state.
    type Input;
    /// Event type for handling (use `()` for non-interactive widgets).
    type Event;

    /// Update the widget's state with new input data.
    fn update_state(state: &mut Self::State, input: Self::Input);

    /// Handle an event. Returns true if the event was consumed.
    fn handle_event(_state: &mut Self::State, _event: Self::Event) -> bool {
        false
    }
}

/// Trait for display modes in the TUI.
///
/// Each mode manages its own state, handles key/mouse events, and renders
/// directly to a Frame.
pub trait Mode {
    type Dag;

    fn name(&self) -> &'static str;
    fn keybindings(&self) -> &'static [(&'static str, &'static str)];
    fn current_selection(&self) -> Option<Selection>;

    fn legend_sections(&self) -> Vec<LegendSection> {
        Vec::new()
    }

    fn take_clicked_error(&mut self) -> Option<String> {
        None
    }

    fn update(&mut self, dag: Option<&Self::Dag>, error: Option<String>);

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }

    fn handle_mouse(&mut self, _mouse: MouseEvent) -> bool {
        false
    }

    fn render(&mut self, frame: &mut Frame, area: Rect);
}
