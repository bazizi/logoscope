use crate::parser::{LogEntryIndices, parse_log_by_path_async};
use crate::theme::AppTheme;
use crate::utils::get_config_dir_path;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const ROW_BUFFER_SIZE: usize = 50;
pub const SCROLL_MULTIPLIER: f32 = 4.;
pub const SCROLL_END: f32 = (ROW_BUFFER_SIZE / 4) as f32;
pub const CONFIGS_FILE_NAME: &str = "config.json";

use crate::messages::Message;
use crate::tab::Tab;
use crate::table::Table;

use iced::{Element, Subscription, Task, event};

#[derive(Debug, Clone)]
pub struct LogLoadError {}

#[derive(Debug, Clone)]
pub struct LogLoadSuccess {
    pub table: Table,
    pub file_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogoscopeApp {
    pub filters: Vec<String>,
    pub search: Vec<String>,
    pub tabs: Vec<Tab>,
    pub current_tab: usize,
    pub multiselect_enabled: bool,
    pub tail_enabled: bool,
    pub theme: AppTheme,
    pub loading_in_progress: bool,
    pub text_size: f32,
}

impl LogoscopeApp {
    pub fn new() -> (Self, iced::Task<Message>) {
        let mut all_tasks = Vec::new();
        let mut tail_tasks = Vec::new();
        let mut loaded_config = None;
        if let Ok(file_contents) = std::fs::read(get_config_dir_path().join(CONFIGS_FILE_NAME)) {
            if let Ok(mut logoscope_app) =
                serde_json::from_str::<LogoscopeApp>(&String::from_utf8(file_contents).unwrap())
            {
                log::info!("Loaded config: [{:?}", logoscope_app);
                let files_to_open: Vec<_> = logoscope_app
                    .tabs
                    .iter()
                    .map(|tab| tab.file.clone())
                    .collect();
                log::info!("Loading files from last session: [{:?}]", files_to_open);
                logoscope_app.loading_in_progress = !files_to_open.is_empty();
                all_tasks.push(iced::Task::perform(
                    LogoscopeApp::open_files(files_to_open),
                    Message::FileOpened,
                ));

                for tab in &logoscope_app.tabs {
                    tail_tasks.push(Task::perform(
                        Tab::tail(tab.clone(), logoscope_app.filters.clone()),
                        Message::TailUpdate,
                    ));
                }
                logoscope_app.tabs.clear();
                loaded_config = Some(logoscope_app);
            }
        }

        log::info!("Loading app");
        let mut tasks = all_tasks
            .into_iter()
            .reduce(|acc, task| acc.chain(task))
            .unwrap_or(Task::none());
        if !tail_tasks.is_empty() {
            tasks = tasks.chain(
                tail_tasks
                    .into_iter()
                    .reduce(|acc, task| acc.chain(task))
                    .unwrap_or(Task::none()),
            );
        }

        if let Some(config) = loaded_config {
            return (config, tasks);
        }

        (
            Self {
                filters: Vec::new(),
                search: Vec::new(),
                tabs: Vec::<Tab>::new(),
                current_tab: 0,
                multiselect_enabled: false,
                tail_enabled: false,
                theme: AppTheme::TokyoNightStorm,
                loading_in_progress: false,
                text_size: 14.,
            },
            tasks,
        )
    }
    pub fn title(&self) -> String {
        "Logoscope".to_owned()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        event::listen().map(Message::Event)
    }

    pub fn get_current_tab(&self) -> Option<&Tab> {
        if self.tabs.is_empty() || self.tabs.len() <= self.current_tab {
            return None;
        }

        Some(&self.tabs[self.current_tab])
    }

    pub fn get_current_tab_mut(&mut self) -> Option<&mut Tab> {
        if self.tabs.is_empty() || self.tabs.len() <= self.current_tab {
            return None;
        }

        Some(&mut self.tabs[self.current_tab])
    }

    pub fn get_current_session(&self) -> Option<&Table> {
        let current_tab = self.get_current_tab()?;
        current_tab.sessions.get(current_tab.current_session)
    }

    pub fn get_current_session_mut(&mut self) -> Option<&mut Table> {
        self.get_current_tab_mut()?.get_current_session_mut()
    }

    async fn open_files(file_paths: Vec<PathBuf>) -> Result<Vec<LogLoadSuccess>, LogLoadError> {
        log::info!("Opening files: {:?}", file_paths);
        let mut tables = Vec::new();
        for file_path in file_paths {
            let Ok(log_data) = parse_log_by_path_async(&file_path).await else {
                log::error!("Failed to parse: [{:?}]", file_path);
                continue;
            };
            let num_rows = log_data.len();
            tables.push(LogLoadSuccess {
                table: Table {
                    identifier: log_data
                        .first()
                        .unwrap_or(&["".to_owned()].into_iter().collect::<Vec<_>>())
                        [LogEntryIndices::Date as usize]
                        .clone(),
                    rows: log_data.clone(),
                    scroll_pos: (num_rows as f32),
                },
                file_path,
            });
        }
        Ok(tables)
    }

    pub async fn open_file() -> Result<Vec<LogLoadSuccess>, LogLoadError> {
        let mut files_to_open = Vec::new();
        if let Some(files) = rfd::FileDialog::new()
            .add_filter("log files", &["log", "txt", "text"])
            .pick_files()
        {
            files_to_open = files;
        }
        LogoscopeApp::open_files(files_to_open).await
    }

    pub fn next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.current_tab = self.current_tab.saturating_add(1);
        self.current_tab %= self.tabs.len();
    }

    pub fn prev_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        if self.current_tab == 0 {
            self.current_tab = self.tabs.len() - 1;
            return;
        }
        self.current_tab = self.current_tab.saturating_sub(1);
    }

    pub fn scroll(&mut self, amount: f32) {
        let Some(current_session) = self.get_current_session_mut() else {
            return;
        };
        current_session.scroll_pos += amount * SCROLL_MULTIPLIER;
        current_session.scroll_pos = current_session.scroll_pos.clamp(
            std::cmp::min(SCROLL_END as usize, current_session.rows.len()) as f32,
            current_session.rows.len() as f32,
        );
    }

    pub fn move_cursor(&mut self, amount: isize) {
        if self.tabs[self.current_tab].selected_rows.is_empty() {
            return;
        }

        let first_selected_row = *self.tabs[self.current_tab]
            .selected_rows
            .iter()
            .next()
            .unwrap();
        self.tabs[self.current_tab].selected_rows.clear();

        let new_row_position = first_selected_row.saturating_add_signed(amount).clamp(
            0,
            self.tabs[self.current_tab].sessions[self.tabs[self.current_tab].current_session]
                .rows
                .len()
                .saturating_sub(1),
        );
        self.tabs[self.current_tab]
            .selected_rows
            .insert(new_row_position);
    }

    pub fn close_tab(&mut self, i: usize) {
        self.tabs.remove(i);
        self.current_tab = self.current_tab.clamp(0, self.tabs.len().saturating_sub(1));
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        message.handle(self)
    }

    pub fn view(&self) -> Element<Message> {
        crate::view::view(self)
    }

    pub fn theme(&self) -> iced::Theme {
        self.theme.clone().into()
    }

    pub fn increase_text_size(&mut self) {
        self.text_size = self.text_size + 1.;
        self.text_size = self.text_size.clamp(1., f32::INFINITY);
    }

    pub fn decrease_text_size(&mut self) {
        self.text_size = self.text_size - 1.;
        self.text_size = self.text_size.clamp(1., f32::INFINITY);
    }
}

impl Drop for LogoscopeApp {
    fn drop(&mut self) {
        log::info!("Shutting down");
        std::fs::write(
            get_config_dir_path().join(CONFIGS_FILE_NAME),
            serde_json::to_string(&self).unwrap(),
        )
        .unwrap();
    }
}
