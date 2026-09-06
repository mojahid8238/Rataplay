use super::actions;
use super::{App, AppState, InputMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

mod downloads;
mod overlays;
mod results;
mod settings;

pub fn handle_mouse_event(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let x = mouse.column;
            let y = mouse.row;
            let double_click = is_double_click(app, x, y);

            // Handle Overlays first
            match app.state {
                AppState::ActionMenu => {
                    overlays::handle_action_menu_mouse(app, x, y, double_click);
                    return;
                }
                AppState::FormatSelection => {
                    overlays::handle_format_selection_mouse(app, x, y, double_click);
                    return;
                }
                AppState::DownloadDialog => {
                    downloads::handle_download_dialog_mouse(app, x, y, double_click);
                    return;
                }
                AppState::Settings => {
                    settings::handle_settings_mouse(app, x, y, double_click);
                    return;
                }
                _ => {}
            }

            // Playback Bar
            if let Some(area) = app.playback_bar_area
                && is_in_rect(x, y, area)
            {
                actions::toggle_pause(app);
                return;
            }

            // Search Bar
            if is_in_rect(x, y, app.search_bar_area) {
                app.input_mode = InputMode::Editing;
                return;
            }

            // Downloads Panel
            if app.show_downloads_panel
                && let Some(area) = app.downloads_area
                && is_in_rect(x, y, area)
            {
                if app.state != AppState::Downloads {
                    app.previous_app_state = app.state;
                    app.state = AppState::Downloads;
                    actions::refresh_local_files(app);
                }

                // Hit testing for Downloads
                let has_active = !app.download_manager.task_order.is_empty();
                let active_height = if has_active {
                    (area.height as f64 * 0.4).round() as u16
                } else {
                    0
                };

                if has_active && y < area.y + active_height {
                    // Active downloads section
                    let list_start_y = area.y + 1 + 1; // Border + Header
                    if y >= list_start_y {
                        let relative_y = y - list_start_y;
                        let idx = app.downloads_active_state.offset() + relative_y as usize;
                        if idx < app.download_manager.task_order.len() {
                            app.selected_download_index = Some(idx);
                            app.selected_local_file_index = None;
                            if double_click {
                                app.previous_app_state = app.state;
                                app.state = AppState::ActionMenu;
                            }
                        }
                    }
                } else {
                    // Local files section
                    let local_start_y = if has_active {
                        area.y + active_height + 1 + 1 // + Border + Header of local block
                    } else {
                        area.y + 1 + 1 // Border + Header
                    };

                    if y >= local_start_y {
                        let relative_y = y - local_start_y;
                        let idx = app.downloads_local_state.offset() + relative_y as usize;
                        if idx < app.local_files.len() {
                            app.selected_local_file_index = Some(idx);
                            app.selected_download_index = None;
                            if double_click {
                                app.previous_app_state = app.state;
                                app.state = AppState::ActionMenu;
                            }
                        }
                    }
                }
                return;
            }

            // Main Content
            if is_in_rect(x, y, app.main_content_area) {
                if app.state == AppState::Downloads {
                    app.state = AppState::Results;
                }

                // Hit testing for Main List
                // Items are 2 lines tall.
                let list_start_y = app.main_content_area.y + 1; // Border
                if y >= list_start_y {
                    let relative_y = y - list_start_y;
                    let item_index = app.main_list_state.offset() + (relative_y / 2) as usize;

                    if item_index < app.search_results.len() {
                        app.selected_result_index = Some(item_index);
                        if double_click {
                            app.previous_app_state = app.state;
                            app.state = AppState::ActionMenu;
                        }
                    } else if (!app.is_url_mode || app.is_playlist_mode)
                        && item_index == app.search_results.len()
                    {
                        // Clicked "Load More" (approx)
                        actions::load_more(app);
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => handle_scroll(app, -1),
        MouseEventKind::ScrollDown => handle_scroll(app, 1),
        _ => {}
    }
}

fn handle_scroll(app: &mut App, dir: i32) {
    match app.state {
        AppState::Results => results::handle_scroll(app, dir),
        AppState::Downloads => downloads::handle_scroll(app, dir),
        AppState::FormatSelection => overlays::handle_format_selection_scroll(app, dir),
        _ => {}
    }
}

fn is_double_click(app: &mut App, x: u16, y: u16) -> bool {
    let now = Instant::now();
    let threshold = Duration::from_millis(500);

    if let (Some(last_time), Some(last_pos)) = (app.last_click_time, app.last_click_pos)
        && now.duration_since(last_time) < threshold
        && last_pos == (x, y)
    {
        app.last_click_time = None;
        return true;
    }
    app.last_click_time = Some(now);
    app.last_click_pos = Some((x, y));
    false
}

fn is_in_rect(x: u16, y: u16, area: ratatui::layout::Rect) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    let code = match app.input_mode {
        InputMode::Editing => key.code,
        _ => match key.code {
            KeyCode::Char(c) => KeyCode::Char(c.to_lowercase().next().unwrap_or(c)),
            _ => key.code,
        },
    };

    log::debug!("Key event: {:?}, input_mode: {:?}", code, app.input_mode);

    match app.input_mode {
        InputMode::Normal => {
            if code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.change_theme();
                return;
            }

            if code == KeyCode::Char('a') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.toggle_animation();
                return;
            }

            if code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.toggle_live();
                return;
            }

            if code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.toggle_playlists();
                return;
            }

            if code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if app.state == AppState::Settings {
                    app.state = app.previous_app_state;
                } else {
                    app.previous_app_state = app.state;
                    app.state = AppState::Settings;
                    app.settings_state.select(Some(0));
                }
                return;
            }

            match app.state {
                AppState::Settings if app.date_filter_selecting_unit => {
                    settings::handle_date_filter_unit_key(app, code)
                }
                AppState::Settings => settings::handle_key(app, code),
                AppState::FormatSelection => overlays::handle_format_selection_key(app, code),
                AppState::Downloads => downloads::handle_key(app, code),
                AppState::DownloadDialog => downloads::handle_download_dialog_key(app, code),
                AppState::ActionMenu => overlays::handle_action_menu_key(app, code),
                _ => results::handle_key(app, code),
            }
        }
        InputMode::Editing => {
            let control = key.modifiers.contains(KeyModifiers::CONTROL);
            match key.code {
                KeyCode::Enter => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        let val = app.settings_input.clone();
                        match app.settings_editing_item {
                            Some(crate::tui::components::settings::SettingItem::SearchLimit) => {
                                if let Ok(n) = val.parse::<u32>() {
                                    app.search_limit = n;
                                    app.status_message = Some(format!("Search Limit set to {}", n));
                                    app.save_config();
                                    app.reload_config();
                                }
                            }
                            Some(crate::tui::components::settings::SettingItem::PlaylistLimit) => {
                                if let Ok(n) = val.parse::<u32>() {
                                    app.playlist_limit = n;
                                    app.status_message =
                                        Some(format!("Playlist Limit set to {}", n));
                                    app.save_config();
                                    app.reload_config();
                                }
                            }
                            Some(
                                crate::tui::components::settings::SettingItem::DownloadDirectory,
                            ) => {
                                app.download_directory = val;
                                app.status_message = Some(format!(
                                    "Download Directory set to {}",
                                    app.download_directory
                                ));
                                app.save_config();
                                app.reload_config();
                            }
                            Some(crate::tui::components::settings::SettingItem::ProgressStyle) => {
                                app.progress_style = val;
                                app.status_message = Some(format!(
                                    "Progress Style set to \"{}\"",
                                    app.progress_style
                                ));
                                app.save_config();
                                app.reload_config();
                            }
                            Some(crate::tui::components::settings::SettingItem::DateFilter) => {
                                if let Ok(n) = val.parse::<u32>() {
                                    if n == 0 {
                                        app.status_message =
                                            Some("Value must be at least 1".to_string());
                                        app.settings_input = "1".to_string();
                                        app.settings_cursor_position = 1;
                                        return;
                                    }
                                    let unit_name = match app.date_filter_unit {
                                        crate::model::DateFilterUnit::Day(_) => "Day",
                                        crate::model::DateFilterUnit::Week(_) => "Week",
                                        crate::model::DateFilterUnit::Month(_) => "Month",
                                        crate::model::DateFilterUnit::Off => "Off",
                                    };
                                    app.date_filter_unit = match app.date_filter_unit {
                                        crate::model::DateFilterUnit::Day(_) => {
                                            crate::model::DateFilterUnit::Day(n)
                                        }
                                        crate::model::DateFilterUnit::Week(_) => {
                                            crate::model::DateFilterUnit::Week(n)
                                        }
                                        crate::model::DateFilterUnit::Month(_) => {
                                            crate::model::DateFilterUnit::Month(n)
                                        }
                                        crate::model::DateFilterUnit::Off => {
                                            crate::model::DateFilterUnit::Off
                                        }
                                    };
                                    app.status_message =
                                        Some(format!("Filter Search: {} ({})", unit_name, n));
                                    app.save_config();
                                    app.reload_config();
                                    if app.state == AppState::Results && !app.is_url_mode {
                                        crate::app::actions::perform_search(app);
                                    }
                                } else {
                                    app.status_message = Some("Invalid number".to_string());
                                    app.settings_input.clear();
                                    app.settings_cursor_position = 0;
                                }
                            }
                            _ => {}
                        }
                        app.settings_input.clear();
                        app.settings_cursor_position = 0;
                        app.settings_editing_item = None;
                        app.input_mode = InputMode::Normal;
                    } else {
                        actions::perform_search(app);
                    }
                }
                KeyCode::Char(c) => {
                    if control {
                        match c {
                            'u' => {
                                if app.state == AppState::Settings
                                    && app.settings_editing_item.is_some()
                                {
                                    app.settings_input.drain(..app.settings_cursor_position);
                                    app.settings_cursor_position = 0;
                                } else {
                                    app.search_query.drain(..app.cursor_position);
                                    app.cursor_position = 0;
                                }
                            }
                            'k' => {
                                if app.state == AppState::Settings
                                    && app.settings_editing_item.is_some()
                                {
                                    app.settings_input.truncate(app.settings_cursor_position);
                                } else {
                                    app.search_query.truncate(app.cursor_position);
                                }
                            }
                            'w' | 'h' => {
                                delete_word_backwards(app);
                            }
                            'a' => {
                                if app.state == AppState::Settings
                                    && app.settings_editing_item.is_some()
                                {
                                    app.settings_cursor_position = 0;
                                } else {
                                    app.cursor_position = 0;
                                }
                            }
                            'e' => {
                                if app.state == AppState::Settings
                                    && app.settings_editing_item.is_some()
                                {
                                    app.settings_cursor_position = app.settings_input.len();
                                } else {
                                    app.cursor_position = app.search_query.len();
                                }
                            }
                            _ => {}
                        }
                    } else {
                        if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                            app.settings_input.insert(app.settings_cursor_position, c);
                            app.settings_cursor_position += c.len_utf8();
                        } else {
                            app.search_query.insert(app.cursor_position, c);
                            app.cursor_position += c.len_utf8();
                        }
                    }
                }
                KeyCode::Backspace => {
                    if control {
                        delete_word_backwards(app);
                    } else {
                        if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                            if app.settings_cursor_position > 0 {
                                // Find the start of the previous char
                                let mut prev_char_idx = 0;
                                for (idx, _) in app.settings_input.char_indices() {
                                    if idx >= app.settings_cursor_position {
                                        break;
                                    }
                                    prev_char_idx = idx;
                                }
                                app.settings_input.remove(prev_char_idx);
                                app.settings_cursor_position = prev_char_idx;
                            }
                        } else if app.cursor_position > 0 {
                            let mut prev_char_idx = 0;
                            for (idx, _) in app.search_query.char_indices() {
                                if idx >= app.cursor_position {
                                    break;
                                }
                                prev_char_idx = idx;
                            }
                            app.search_query.remove(prev_char_idx);
                            app.cursor_position = prev_char_idx;
                        }
                    }
                }
                KeyCode::Delete => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        if app.settings_cursor_position < app.settings_input.len() {
                            app.settings_input.remove(app.settings_cursor_position);
                        }
                    } else if app.cursor_position < app.search_query.len() {
                        app.search_query.remove(app.cursor_position);
                    }
                }
                KeyCode::Left => {
                    if control {
                        move_word_left(app);
                    } else if app.state == AppState::Settings && app.settings_editing_item.is_some()
                    {
                        if app.settings_cursor_position > 0 {
                            let mut prev_char_idx = 0;
                            for (idx, _) in app.settings_input.char_indices() {
                                if idx >= app.settings_cursor_position {
                                    break;
                                }
                                prev_char_idx = idx;
                            }
                            app.settings_cursor_position = prev_char_idx;
                        }
                    } else if app.cursor_position > 0 {
                        let mut prev_char_idx = 0;
                        for (idx, _) in app.search_query.char_indices() {
                            if idx >= app.cursor_position {
                                break;
                            }
                            prev_char_idx = idx;
                        }
                        app.cursor_position = prev_char_idx;
                    }
                }
                KeyCode::Right => {
                    if control {
                        move_word_right(app);
                    } else if app.state == AppState::Settings && app.settings_editing_item.is_some()
                    {
                        if app.settings_cursor_position < app.settings_input.len()
                            && let Some((_idx, c)) = app.settings_input
                                [app.settings_cursor_position..]
                                .char_indices()
                                .next()
                        {
                            app.settings_cursor_position += c.len_utf8();
                        }
                    } else if app.cursor_position < app.search_query.len()
                        && let Some((_idx, c)) = app.search_query[app.cursor_position..]
                            .char_indices()
                            .next()
                    {
                        app.cursor_position += c.len_utf8();
                    }
                }
                KeyCode::Home => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        app.settings_cursor_position = 0;
                    } else {
                        app.cursor_position = 0;
                    }
                }
                KeyCode::End => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        app.settings_cursor_position = app.settings_input.len();
                    } else {
                        app.cursor_position = app.search_query.len();
                    }
                }
                KeyCode::Esc | KeyCode::Tab => {
                    if app.state == AppState::Settings {
                        if let Some(crate::tui::components::settings::SettingItem::DateFilter) =
                            app.settings_editing_item
                        {
                            // Cancel date filter edit: revert to Off since no value was confirmed
                            app.date_filter_unit = crate::model::DateFilterUnit::Off;
                            app.status_message = Some("Filter Search: Off".to_string());
                        }
                        app.settings_input.clear();
                        app.settings_cursor_position = 0;
                        app.settings_editing_item = None;
                    }
                    app.input_mode = InputMode::Normal;
                }
                _ => {}
            }
        }
        InputMode::Loading => {
            if code == KeyCode::Esc || code == KeyCode::Char('x') {
                app.terminal_loading = false;
                app.terminal_loading_error = None;
                app.input_mode = InputMode::Normal;
            }
        }
    }
}

fn delete_word_backwards(app: &mut App) {
    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
        if app.settings_cursor_position == 0 {
            return;
        }

        let mut chars = app.settings_input[..app.settings_cursor_position]
            .char_indices()
            .rev()
            .peekable();

        for (_, c) in chars.by_ref() {
            if !c.is_whitespace() {
                break;
            }
        }

        let mut start_idx = 0;
        for (idx, c) in chars {
            if c.is_whitespace() {
                start_idx = idx + 1;
                break;
            }
        }

        app.settings_input
            .drain(start_idx..app.settings_cursor_position);
        app.settings_cursor_position = start_idx;
    } else {
        if app.cursor_position == 0 {
            return;
        }

        let mut chars = app.search_query[..app.cursor_position]
            .char_indices()
            .rev()
            .peekable();

        for (_, c) in chars.by_ref() {
            if !c.is_whitespace() {
                break;
            }
        }

        let mut start_idx = 0;
        for (idx, c) in chars {
            if c.is_whitespace() {
                start_idx = idx + 1;
                break;
            }
        }

        app.search_query.drain(start_idx..app.cursor_position);
        app.cursor_position = start_idx;
    }
}

fn move_word_left(app: &mut App) {
    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
        if app.settings_cursor_position == 0 {
            return;
        }

        let mut chars = app.settings_input[..app.settings_cursor_position]
            .char_indices()
            .rev()
            .peekable();

        while let Some((_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        while let Some((_, c)) = chars.peek() {
            if !c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        app.settings_cursor_position = chars.next().map(|(i, _)| i + 1).unwrap_or(0);
    } else {
        if app.cursor_position == 0 {
            return;
        }

        let mut chars = app.search_query[..app.cursor_position]
            .char_indices()
            .rev()
            .peekable();

        while let Some((_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        while let Some((_, c)) = chars.peek() {
            if !c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        app.cursor_position = chars.next().map(|(i, _)| i + 1).unwrap_or(0);
    }
}

fn move_word_right(app: &mut App) {
    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
        if app.settings_cursor_position >= app.settings_input.len() {
            return;
        }

        let mut pos = app.settings_cursor_position;
        let mut chars = app.settings_input[pos..].char_indices();

        for (_, c) in chars.by_ref() {
            pos += c.len_utf8();
            if !c.is_whitespace() {
                break;
            }
        }

        for (_, c) in chars {
            pos += c.len_utf8();
            if c.is_whitespace() {
                break;
            }
        }

        app.settings_cursor_position = pos;
    } else {
        if app.cursor_position >= app.search_query.len() {
            return;
        }

        let mut pos = app.cursor_position;
        let mut chars = app.search_query[pos..].char_indices();

        for (_, c) in chars.by_ref() {
            pos += c.len_utf8();
            if !c.is_whitespace() {
                break;
            }
        }

        for (_, c) in chars {
            pos += c.len_utf8();
            if c.is_whitespace() {
                break;
            }
        }

        app.cursor_position = pos;
    }
}
