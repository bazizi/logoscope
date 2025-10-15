use crate::parser::{LogEntryIndices, parse_log_by_path_async};
use crate::theme::AppTheme;
use crate::utils::get_config_dir_path;
use async_std::task::sleep;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::env::args;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub const ROW_BUFFER_SIZE: usize = 50;
pub const SCROLL_MULTIPLIER: f32 = 4.;
pub const SCROLL_END: f32 = (ROW_BUFFER_SIZE / 4) as f32;
pub const CONFIGS_FILE_NAME: &str = "config.json";
pub const LOCALHOST_IPV4: &str = "127.0.0.1";
const PORT_FILE: &str = "PORT";

use crate::messages::Message;
use crate::tab::{Tab, TabType};
use crate::table::Table;
use crate::table::TableRow;

use iced::{Element, Subscription, event};

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
    pub tabs: VecDeque<Tab>,
    pub current_tab: usize,
    pub multiselect_enabled: bool,
    pub tail_enabled: bool,
    pub theme: AppTheme,
    pub loading_in_progress: bool,
    pub tcp_port: Option<u16>,
    pub text_size: f32,
    #[serde(skip_serializing, skip_deserializing)]
    pub tcp_listener: Option<Arc<std::net::TcpListener>>,
}

impl LogoscopeApp {
    fn make_tcp_listener() -> std::net::TcpListener {
        let tcp_listener = std::net::TcpListener::bind(format!("{}:0", LOCALHOST_IPV4)).unwrap();
        std::fs::write(
            get_config_dir_path().join(PORT_FILE),
            tcp_listener.local_addr().unwrap().port().to_string(),
        )
        .unwrap();
        tcp_listener.set_nonblocking(true).unwrap();
        log::info!(
            "Listening on TCP port [{}]",
            tcp_listener.local_addr().unwrap().port()
        );
        tcp_listener
    }

    pub fn init_net(&mut self) {
        if let Some(port_num) = self.tcp_port {
            log::info!("Attempting connection to port [{}]", port_num);
            if let Ok(mut conn) =
                std::net::TcpStream::connect(format!("{}:{}", LOCALHOST_IPV4, port_num))
            {
                log::info!("Successfully connected to port {}", port_num);
                if let Some(file_to_open) = args().nth(1) {
                    conn.write_all(file_to_open.as_bytes()).unwrap();
                }
                conn.flush().unwrap();
                conn.shutdown(Shutdown::Both).unwrap(); // On Windows this is necessary
                log::info!("Redirected to running instance");
                std::process::exit(0);
            }
        }
        self.tcp_listener = Some(LogoscopeApp::make_tcp_listener().into());
        self.tcp_port = Some(
            self.tcp_listener
                .as_ref()
                .unwrap()
                .local_addr()
                .unwrap()
                .port(),
        );
        self.serialize();
    }

    pub async fn handle_incoming_requests(
        listener: Arc<std::net::TcpListener>,
    ) -> Result<Vec<LogLoadSuccess>, LogLoadError> {
        sleep(Duration::from_millis(300)).await;
        let stream = listener.incoming().next().unwrap();
        match stream {
            Ok(mut stream) => {
                log::info!("reading from stream: {}", stream.peer_addr().unwrap());

                // do something with the TcpStream
                stream
                    .set_read_timeout(Some(Duration::from_millis(300)))
                    .unwrap();
                let mut file_path = String::new();

                let read_result = stream.read_to_string(&mut file_path);
                if let Ok(bytes) = read_result {
                    log::info!("Read [{}] bytes from the stream", bytes);
                } else {
                    log::error!("Failed to read from the TCP stream: [{:?}]", read_result);
                    return Ok(Vec::new());
                }

                return LogoscopeApp::open_files(vec![PathBuf::from_str(&file_path).unwrap()])
                    .await;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("encountered IO error: {e}"),
        }

        Ok(Vec::new())
    }

    pub fn new() -> (Self, iced::Task<Message>) {
        let tabs_to_add = args().skip(1).collect::<Vec<_>>();
        if let Ok(file_contents) = std::fs::read(get_config_dir_path().join(CONFIGS_FILE_NAME)) {
            if let Ok(mut logoscope_app) =
                serde_json::from_str::<LogoscopeApp>(&String::from_utf8(file_contents).unwrap())
            {
                logoscope_app.init_net();
                for file_path in &tabs_to_add {
                    logoscope_app.tabs.push_back(Tab {
                        file: std::path::PathBuf::from_str(file_path).unwrap(),
                        ..Tab::default()
                    });
                }
                log::info!("Loaded config: [{:?}", logoscope_app);

                let files_to_open: Vec<_> = logoscope_app
                    .tabs
                    .iter()
                    .map(|tab| tab.file.clone())
                    .collect();
                log::info!("Loading files from last session: [{:?}]", files_to_open);
                // Clear tabs as they'll be reopened fresh
                logoscope_app.tabs.clear();
                logoscope_app.loading_in_progress = !files_to_open.is_empty();

                return (
                    logoscope_app,
                    iced::Task::perform(
                        LogoscopeApp::open_files(files_to_open),
                        Message::FileOpened,
                    ),
                );
            }
        }

        log::info!("Loading app");
        let mut logoscope_app = Self {
            filters: Vec::new(),
            search: Vec::new(),
            tabs: tabs_to_add
                .into_iter()
                .map(|file_path| Tab {
                    file: std::path::PathBuf::from_str(&file_path).unwrap(),
                    ..Tab::default()
                })
                .collect(),
            current_tab: 0,
            multiselect_enabled: false,
            tail_enabled: false,
            theme: AppTheme::TokyoNightStorm,
            loading_in_progress: false,
            tcp_port: None,
            text_size: 14.,
            tcp_listener: None,
        };

        logoscope_app.init_net();
        (logoscope_app, iced::Task::none())
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

    pub async fn reload_combined_tab(tabs: VecDeque<Tab>) -> Tab {
        let mut combined_tab_data: Vec<TableRow> = Vec::new();
        for tab in tabs {
            if let TabType::Combined = tab.tab_type {
                // Skip the combined tab if it pre-exiests
                continue;
            }
            for session in &tab.sessions {
                combined_tab_data.append(&mut session.rows.clone());
            }
        }

        combined_tab_data.sort_by(|row1, row2| {
            return row1[LogEntryIndices::Date as usize].cmp(&row2[LogEntryIndices::Date as usize]);
        });

        let mut combined_tab = Tab::default();
        let mut table = Table::default();
        table.rows = combined_tab_data;
        table.scroll_pos = table.rows.len() as f32;
        table.identifier = "All".to_owned();
        combined_tab.table = table.clone();
        combined_tab.sessions.push(table);
        combined_tab.current_session = 0;
        combined_tab.tab_type = TabType::Combined;
        combined_tab
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

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        message.handle(self)
    }

    pub fn view(&self) -> Element<Message> {
        crate::view::view(self)
    }

    pub fn theme(&self) -> iced::Theme {
        self.theme.clone().into()
    }

    pub fn increase_text_size(&mut self) {
        self.text_size += 1.;
        self.text_size = self.text_size.clamp(1., f32::INFINITY);
    }

    pub fn decrease_text_size(&mut self) {
        self.text_size -= 1.;
        self.text_size = self.text_size.clamp(1., f32::INFINITY);
    }

    pub fn serialize(&mut self) {
        std::fs::write(
            get_config_dir_path().join(CONFIGS_FILE_NAME),
            serde_json::to_string(&self).unwrap(),
        )
        .unwrap();
    }
}

impl Drop for LogoscopeApp {
    fn drop(&mut self) {
        log::info!("Shutting down");
        self.serialize();
    }
}
