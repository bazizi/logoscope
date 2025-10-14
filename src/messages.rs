use core::f32;

use crate::app::{LogLoadError, LogLoadSuccess, LogoscopeApp, SCROLL_END};
use crate::parser::LogEntryIndices;
use crate::tab::{Tab, TabType};
use crate::theme::AppTheme;
use copypasta::{ClipboardContext, ClipboardProvider};
use iced::Task;
use iced::event::Event;
use iced::keyboard::key;
use iced::{keyboard, mouse};
use log::info;

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ToggleMultiselect(bool),
    ToggleTail(bool),
    TailUpdate(Tab),
    FilterChanged(String),
    FilterUpdate(Option<Tab>),
    PrevSearch,
    SearchChanged(String),
    NextSearch,
    FileOpen,
    FileOpened(Result<Vec<LogLoadSuccess>, LogLoadError>),
    Scrolled(f32),
    RowClicked(usize),
    TabChanged(usize),
    TabClosed(usize),
    Event(Event),
    PrevTheme,
    ThemeChanged(AppTheme),
    NextTheme,
    PrevSession,
    SessionChanged(usize),
    NextSession,
    TextSizeIncrease,
    TextSizeDecrease,
    CombinedTabReloaded(Tab),
}

fn do_search(range: Vec<usize>, app: &mut LogoscopeApp) -> Task<Message> {
    let mut search_kws = app.search.clone();
    let Some(current_session) = app.get_current_session_mut() else {
        return Task::none();
    };

    for i in range {
        let Some(row) = current_session.rows.get(i) else {
            continue;
        };
        for search_kw in &mut search_kws {
            let mut row_column = LogEntryIndices::Log as usize;
            let sanitized_kw = if search_kw.to_lowercase().starts_with("s:") {
                row_column = LogEntryIndices::Level as usize;
                search_kw[2..].to_owned()
            } else if search_kw.to_lowercase().starts_with("d:") {
                row_column = LogEntryIndices::Date as usize;
                search_kw[2..].to_owned()
            } else {
                search_kw.clone()
            };
            if row[row_column]
                .to_lowercase()
                .contains(&sanitized_kw.to_lowercase())
            {
                current_session.scroll_pos = current_session
                    .rows
                    .len()
                    .saturating_sub(i)
                    .clamp(0, current_session.rows.len())
                    as f32;
                log::debug!("Searching: Index {} matched!", i);
                return Task::none();
            } else {
                log::debug!("Searching: Index {} did not match", i);
            }
        }
    }
    Task::none()
}

impl Message {
    pub fn handle(self, app: &mut LogoscopeApp) -> Task<Message> {
        match self {
            Message::NextSession => {
                let Some(current_tab) = app.get_current_tab_mut() else {
                    return Task::none();
                };
                current_tab.current_session = current_tab
                    .current_session
                    .saturating_add(1)
                    .clamp(0, current_tab.sessions.len().saturating_sub(1))
            }
            Message::PrevSession => {
                let Some(current_tab) = app.get_current_tab_mut() else {
                    return Task::none();
                };
                current_tab.current_session = current_tab
                    .current_session
                    .saturating_sub(1)
                    .clamp(0, current_tab.sessions.len().saturating_sub(1))
            }
            Message::SessionChanged(curr_session) => {
                let Some(current_tab) = app.get_current_tab_mut() else {
                    return Task::none();
                };
                current_tab.current_session = curr_session;
            }
            Message::PrevTheme => {
                app.theme = (app.theme.clone() as usize)
                    .saturating_sub(1)
                    .clamp(0, AppTheme::ALL.len() - 1)
                    .into();
            }
            Message::ThemeChanged(theme) => {
                app.theme = theme;
            }
            Message::NextTheme => {
                app.theme = (app.theme.clone() as usize)
                    .saturating_add(1)
                    .clamp(0, AppTheme::ALL.len() - 1)
                    .into();
            }
            Message::ToggleTail(tail_enabled) => {
                app.tail_enabled = tail_enabled;
                if app.tail_enabled {
                    let mut tail_tasks = Vec::new();
                    for tab in &app.tabs {
                        if let TabType::Combined = tab.tab_type {
                            // Skip the combined tab
                            continue;
                        }
                        tail_tasks.push(Task::perform(
                            Tab::tail(tab.clone(), app.filters.clone()),
                            Message::TailUpdate,
                        ));
                    }

                    return tail_tasks
                        .into_iter()
                        .reduce(|acc, task| acc.chain(task))
                        .unwrap_or(Task::none());
                }
            }
            Message::CombinedTabReloaded(tab) => {
                if app.tabs.is_empty() {
                    return Task::none();
                }

                if TabType::Combined != app.tabs.front().unwrap().tab_type {
                    // The combined tab isn't added yet
                    let mut combined_tab = Tab::default();
                    combined_tab.tab_type = TabType::Combined;
                    app.tabs.push_front(combined_tab);
                }
                if let Some(combined_tab) = app.tabs.front_mut() {
                    combined_tab.sessions = tab.sessions;
                }
                return Task::none();
            }
            Message::TailUpdate(new_tab) => {
                if !app.tail_enabled {
                    return Task::none();
                }

                for tab in &mut app.tabs {
                    if let TabType::Combined = tab.tab_type {
                        continue;
                    }

                    if tab.file_size != new_tab.file_size && tab.file == new_tab.file {
                        *tab = new_tab.clone();
                    }
                }

                return iced::Task::batch([
                    iced::Task::perform(
                        LogoscopeApp::reload_combined_tab(app.tabs.clone()),
                        Message::CombinedTabReloaded,
                    ),
                    Task::perform(
                        Tab::tail(new_tab.clone(), app.filters.clone()),
                        Message::TailUpdate,
                    ),
                ]);
            }
            Message::ToggleMultiselect(multiselect_enabled) => {
                app.multiselect_enabled = multiselect_enabled;
            }
            Message::FilterChanged(filters) => {
                app.filters = filters.split(',').map(|str| str.to_owned()).collect();
                let mut tab_reload_tasks = Vec::new();
                for tab in &mut app.tabs {
                    tab_reload_tasks.push(Task::perform(
                        Tab::apply_filter(tab.clone(), app.filters.clone()),
                        Message::FilterUpdate,
                    ));
                }

                return tab_reload_tasks
                    .into_iter()
                    .reduce(|acc, task| acc.chain(task))
                    .unwrap_or(Task::none());
            }
            Message::FilterUpdate(maybe_update) => {
                let Some(tab_update) = maybe_update else {
                    return Task::none();
                };

                for tab in &mut app.tabs {
                    if let TabType::Combined = tab.tab_type {
                        continue;
                    }

                    if tab.file == tab_update.file {
                        *tab = tab_update;
                        if let Some(last_session) = tab.sessions.last_mut() {
                            log::info!("Reloaded tab {:?}", tab.file);
                            if app.tail_enabled {
                                last_session.scroll_pos = SCROLL_END;
                            } else {
                                last_session.scroll_pos =
                                    last_session.rows.len().saturating_sub(SCROLL_END as usize)
                                        as f32;
                            }
                        }
                        break;
                    }
                }

                return iced::Task::perform(
                    LogoscopeApp::reload_combined_tab(app.tabs.clone()),
                    Message::CombinedTabReloaded,
                );
            }
            Message::PrevSearch => {
                let Some(current_session) = app.get_current_session_mut() else {
                    return Task::none();
                };

                // Have to invert the scroll position to reverse the slider rendering
                let scroll_pos = current_session
                    .rows
                    .len()
                    .saturating_sub(current_session.scroll_pos as usize)
                    .clamp(0, current_session.rows.len().saturating_sub(1));
                return do_search((0..scroll_pos).rev().collect(), app);
            }
            Message::SearchChanged(search) => {
                app.search = search.split(',').map(|str| str.to_owned()).collect()
            }
            Message::NextSearch => {
                let Some(current_session) = app.get_current_session_mut() else {
                    return Task::none();
                };

                // Have to invert the scroll position to reverse the slider rendering
                let scroll_pos = current_session
                    .rows
                    .len()
                    .saturating_sub(current_session.scroll_pos as usize)
                    .clamp(0, current_session.rows.len().saturating_sub(1));
                return do_search(
                    (scroll_pos.saturating_add(1)..current_session.rows.len()).collect(),
                    app,
                );
            }
            Message::FileOpen => {
                app.loading_in_progress = true;
                return Task::perform(LogoscopeApp::open_file(), Message::FileOpened);
            }
            Message::FileOpened(file_load_result) => {
                app.loading_in_progress = false;
                let Ok(loaded_files) = file_load_result else {
                    log::error!("File open error: [{:?}]", file_load_result);
                    return iced::Task::perform(
                        LogoscopeApp::handle_incoming_requests(app.tcp_listener.clone().unwrap()),
                        Message::FileOpened,
                    );
                };

                if loaded_files.is_empty() {
                    return iced::Task::perform(
                        LogoscopeApp::handle_incoming_requests(app.tcp_listener.clone().unwrap()),
                        Message::FileOpened,
                    );
                }

                log::info!(
                    "Files opened {:?}",
                    loaded_files
                        .iter()
                        .map(|file| { &file.file_path })
                        .collect::<Vec<_>>()
                );

                let mut tab_reload_tasks = Vec::new();

                for loaded_file in loaded_files {
                    app.tabs.push_back(Tab {
                        table: loaded_file.table,
                        file: loaded_file.file_path,
                        ..Tab::default()
                    });
                    app.current_tab = app.tabs.len().saturating_sub(1);
                    for tab in &mut app.tabs {
                        tab_reload_tasks.push(Task::perform(
                            Tab::apply_filter(tab.clone(), app.filters.clone()),
                            Message::FilterUpdate,
                        ));
                        tab.current_session = tab.sessions.len().saturating_sub(1);
                        if let Some(last_session) = tab.sessions.last_mut() {
                            last_session.scroll_pos = last_session.rows.len() as f32;
                        }
                    }
                }

                if app.tail_enabled {
                    for tab in &app.tabs {
                        tab_reload_tasks.push(Task::perform(
                            Tab::tail(tab.clone(), app.filters.clone()),
                            Message::TailUpdate,
                        ));
                    }
                }

                tab_reload_tasks.push(iced::Task::perform(
                    LogoscopeApp::handle_incoming_requests(app.tcp_listener.clone().unwrap()),
                    Message::FileOpened,
                ));

                return iced::Task::batch([
                    tab_reload_tasks
                        .into_iter()
                        .reduce(|acc, task| acc.chain(task))
                        .unwrap_or(Task::none()),
                    iced::Task::perform(
                        LogoscopeApp::reload_combined_tab(app.tabs.clone()),
                        Message::CombinedTabReloaded,
                    ),
                ]);
            }
            Message::Scrolled(pos) => {
                log::info!("Scrolled to {}", pos);
                let Some(current_session) = app.get_current_session_mut() else {
                    return Task::none();
                };
                current_session.scroll_pos = pos;
            }
            Message::RowClicked(row_num) => {
                log::info!("Row clicked {}", row_num);
                let multiselect_enabled = app.multiselect_enabled;
                let Some(current_tab) = app.get_current_tab_mut() else {
                    return Task::none();
                };
                if current_tab.selected_rows.contains(&row_num) {
                    current_tab.selected_rows.remove(&row_num);
                    return Task::none();
                }

                if !multiselect_enabled {
                    current_tab.selected_rows.clear();
                }
                current_tab.selected_rows.insert(row_num);
            }
            Message::TabChanged(i) => {
                app.current_tab = i;
            }
            Message::TabClosed(i) => app.close_tab(i),
            Message::Event(event) => {
                // Known keys pressed
                if let Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(identifier),
                    modifiers,
                    ..
                }) = event
                {
                    if modifiers.control() {
                        match identifier {
                            key::Named::Tab => {
                                if modifiers.shift() {
                                    app.prev_tab();
                                    return Task::none();
                                }
                                app.next_tab();
                            }
                            key::Named::PageDown => {
                                app.next_tab();
                            }
                            key::Named::PageUp => {
                                app.prev_tab();
                            }
                            _ => {}
                        }
                        return Task::none();
                    }

                    match identifier {
                        key::Named::ArrowDown => {
                            app.move_cursor(1);
                        }
                        key::Named::ArrowUp => {
                            app.move_cursor(-1);
                        }
                        key::Named::ArrowLeft => {
                            let Some(current_tab) = app.get_current_tab_mut() else {
                                return Task::none();
                            };
                            current_tab.current_session =
                                current_tab.current_session.saturating_sub(1);
                        }
                        key::Named::ArrowRight => {
                            let Some(current_tab) = app.get_current_tab_mut() else {
                                return Task::none();
                            };
                            let last_session = current_tab.sessions.len().saturating_sub(1);
                            current_tab.current_session = current_tab
                                .current_session
                                .saturating_add(1)
                                .clamp(0, last_session);
                        }
                        key::Named::PageDown => {
                            app.scroll(-1.);
                        }
                        key::Named::PageUp => {
                            app.scroll(1.);
                        }
                        key::Named::Home => {
                            let Some(current_session) = app.get_current_session_mut() else {
                                return Task::none();
                            };
                            current_session.scroll_pos = current_session.rows.len() as f32;
                        }
                        key::Named::End => {
                            let Some(current_session) = app.get_current_session_mut() else {
                                return Task::none();
                            };
                            current_session.scroll_pos = SCROLL_END;
                        }
                        _ => {}
                    }
                }
                // character keys
                else if let Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Character(key_char),
                    modifiers,
                    ..
                }) = event
                {
                    if key_char.to_lowercase() == "o" {
                        let _ = iced::Task::perform(LogoscopeApp::open_file(), Message::FileOpened);
                    } else if modifiers.control() {
                        if key_char.to_lowercase() == "w" {
                            if app.tabs.is_empty() {
                                return Task::none();
                            }
                            app.close_tab(app.current_tab);
                        } else if key_char.to_lowercase() == "c" {
                            // TODO
                            let Ok(mut clipboard_ctx) = ClipboardContext::new() else {
                                return Task::none();
                            };
                            let Some(current_tab) = app.get_current_tab() else {
                                return Task::none();
                            };
                            let Some(current_session) = app.get_current_session() else {
                                return Task::none();
                            };

                            let mut selected_text = String::new();
                            for row_index in &current_tab.selected_rows {
                                let Some(row) = current_session.rows.get(*row_index) else {
                                    continue;
                                };
                                let Some(text) = row
                                    .clone()
                                    .into_iter()
                                    .reduce(|acc, col| acc.to_owned() + " " + &col)
                                else {
                                    continue;
                                };
                                selected_text += &(text + "\n");
                            }
                            let Ok(_) = clipboard_ctx.set_contents(selected_text.clone()) else {
                                log::error!("Failed to copy to clipboard: [{}]", selected_text);
                                return Task::none();
                            };
                        } else if key_char.to_lowercase() == "=" {
                            app.increase_text_size();
                        } else if key_char.to_lowercase() == "-" {
                            app.decrease_text_size();
                        }
                    }
                } else if let Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines { y, .. },
                }) = event
                {
                    app.scroll(y);
                }
            }
            Message::TextSizeIncrease => {
                app.increase_text_size();
            }
            Message::TextSizeDecrease => {
                app.decrease_text_size();
            }
            Message::None => {}
        }

        Task::none()
    }
}
