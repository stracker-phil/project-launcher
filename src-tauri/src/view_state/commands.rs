//! The main window's page reads its sidebar filter at start and reports each change.

use tauri::State;

use crate::error::AppResult;
use crate::library::{self, Filter};
use crate::state::AppState;

/// The filter shown last, or All when none was saved or its sidebar row is gone.
#[tauri::command]
pub fn view_filter(state: State<'_, AppState>) -> Filter {
    let Some(filter) = state.view.filter() else {
        return Filter::All;
    };
    library::shown_or_all(filter, &state.data.projects(), &state.data.settings().tags)
}

#[tauri::command]
pub fn set_view_filter(state: State<'_, AppState>, filter: Filter) -> AppResult<()> {
    state.view.set_filter(filter)
}
