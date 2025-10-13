use crate::app::SCROLL_END;
use crate::parser::LogEntryIndices;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

pub const SESSION_IDENTIFIERS: [&str; 3] = [
    "client version:",         // Steam - client version:
    "Startup - updater built", // Steam - bootstrap_log
    "[startup]",               // EA app
];

use crate::table::Table;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum TabType {
    Combined, //
    Normal,
}

impl Default for TabType {
    fn default() -> Self {
        TabType::Normal
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Tab {
    #[serde(skip_serializing, skip_deserializing)]
    pub table: Table, // All row data for this tab

    #[serde(skip_serializing, skip_deserializing)]
    pub sessions: Vec<Table>, // Copy of All data for this tab broken into multiple sessions
    pub tab_type: TabType,
    pub file: PathBuf,
    pub file_size: u64,
    pub selected_rows: std::collections::HashSet<usize>,
    pub current_session: usize,
}

impl Tab {
    pub fn get_current_session_mut(&mut self) -> Option<&mut Table> {
        if self.sessions.is_empty() || self.sessions.len() < self.current_session {
            return None;
        }

        Some(&mut self.sessions[self.current_session])
    }

    pub async fn tail(mut tab: Tab, filters: Vec<String>) -> Tab {
        async_std::task::sleep(Duration::from_secs(1)).await;
        let Ok(metadata) = std::fs::metadata(&tab.file) else {
            return tab;
        };

        if tab.file_size == metadata.len() {
            return tab;
        }

        log::info!(
            "Tail update {:?}: {}!={}",
            tab.file,
            metadata.len(),
            tab.file_size
        );
        tab.file_size = metadata.len();
        let Ok(log_data) = crate::parser::parse_log_by_path_async(&tab.file).await else {
            return tab;
        };

        tab.table.rows = log_data;
        let Some(mut tab) = Tab::apply_filter(tab.clone(), filters).await else {
            return tab;
        };
        tab.current_session = tab.sessions.len().saturating_sub(1);
        if let Some(current_session) = tab.get_current_session_mut() {
            current_session.scroll_pos = SCROLL_END;
        }

        tab
    }

    pub async fn apply_filter(mut tab: Tab, filters: Vec<String>) -> Option<Tab> {
        log::info!("applying filters {:?}", filters);
        tab.sessions = [].into();
        let mut session = Table::default();

        // Add the ALL session
        tab.sessions.push(Table {
            identifier: "ALL".into(),
            rows: tab.table.rows.clone(),
            scroll_pos: tab.table.rows.len() as f32,
        });

        for row in &tab.table.rows {
            // Break the table into sessions
            for session_identifier in SESSION_IDENTIFIERS {
                if row[LogEntryIndices::Log as usize]
                    .to_lowercase()
                    .contains(&session_identifier.to_lowercase())
                {
                    session.scroll_pos = session.rows.len() as f32;
                    session.identifier =
                        session
                            .rows
                            .first()
                            .unwrap_or(&vec!["".to_owned(); LogEntryIndices::Date as usize + 1])
                            [LogEntryIndices::Date as usize]
                            .clone();
                    tab.sessions.push(session);
                    session = Table::default();
                    break;
                }
            }
            session.rows.push(row.clone());
        }

        // add the last session
        session.identifier = session
            .rows
            .first()
            .unwrap_or(&["".to_owned()].into_iter().collect::<Vec<_>>())
            [LogEntryIndices::Date as usize]
            .clone();
        tab.sessions.push(session);

        for session in &mut tab.sessions {
            session.rows = session
                .rows
                .clone()
                .into_iter()
                .filter(|row| {
                    for col in row {
                        if filters.is_empty() {
                            return true;
                        }
                        for filter in &filters {
                            if filter.is_empty()
                                || col.to_lowercase().contains(filter.to_lowercase().as_str())
                            {
                                return true;
                            }
                        }
                    }
                    false
                })
                .collect();
        }

        let current_session = tab.get_current_session_mut()?;

        current_session.scroll_pos = current_session.rows.len() as f32;
        tab.selected_rows.clear();

        Some(tab)
    }
}
