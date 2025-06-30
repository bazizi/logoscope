use crate::app::{LogoscopeApp, ROW_BUFFER_SIZE, SCROLL_END};
use crate::messages::Message;
use crate::parser::LogEntryIndices;
use crate::theme::AppTheme;

use iced::widget::pick_list;
use iced::widget::toggler;
use iced::widget::vertical_slider;
use iced::{Background, alignment};
use iced::{
    Element, Length,
    widget::{
        button, column, horizontal_space, keyed_column, row, text, text_input, vertical_space,
    },
};

use crate::table::Table;

fn get_session_list(app: &LogoscopeApp) -> Vec<String> {
    let Some(current_tab) = app.get_current_tab() else {
        return Vec::new();
    };

    current_tab
        .sessions
        .iter()
        .map(|session| session.identifier.clone())
        .collect::<Vec<String>>()
}

fn view_top_navbar(app: &LogoscopeApp) -> Element<Message> {
    let session_list = get_session_list(app);

    let current_session_id = app
        .get_current_session()
        .map(|current_session| current_session.identifier.clone());

    row![
        // Multiselect
        toggler(app.multiselect_enabled)
            .label("Multiselect")
            .on_toggle(Message::ToggleMultiselect),
        horizontal_space().width(Length::FillPortion(5)),
        // Tail
        toggler(app.tail_enabled)
            .label("Tail")
            .on_toggle(Message::ToggleTail),
        horizontal_space().width(Length::FillPortion(5)),
        // Session selection
        text("Session: ").align_y(alignment::Vertical::Center),
        button(text("<")).on_press(Message::PrevSession),
        pick_list(session_list.clone(), current_session_id, move |selection| {
            let Some(current_tab) = app.get_current_tab() else {
                return Message::SessionChanged(session_list.len().saturating_sub(1));
            };
            for (i, session) in current_tab.sessions.iter().enumerate() {
                if session.identifier == selection {
                    return Message::SessionChanged(i);
                }
            }

            Message::SessionChanged(session_list.len().saturating_sub(1))
        }),
        button(text(">")).on_press(Message::NextSession),
        horizontal_space().width(Length::FillPortion(5)),
        // Filter
        text("Filter: "),
        text_input("Comma-separated keywords...", &app.filters.join(","))
            .width(Length::FillPortion(20))
            .on_input(Message::FilterChanged),
        horizontal_space().width(Length::FillPortion(5)),
        // Search
        text("Search: "),
        text_input("Comma-separated keywords...", &app.search.join(","))
            .width(Length::FillPortion(20))
            .on_input(Message::SearchChanged),
        horizontal_space().width(Length::FillPortion(5)),
        // Text size
        button("-").on_press(Message::TextSizeDecrease),
        text(" Zoom ").align_y(alignment::Vertical::Center),
        button("+").on_press(Message::TextSizeIncrease),
        horizontal_space().width(Length::FillPortion(5)),
        // Theme selection
        text("Theme: ").align_y(alignment::Vertical::Center),
        button(text("<")).on_press(Message::PrevTheme),
        pick_list(AppTheme::ALL, Some(&app.theme), Message::ThemeChanged),
        button(text(">")).on_press(Message::NextTheme),
    ]
    .into()
}

fn view_tabs(app: &LogoscopeApp) -> Element<Message> {
    row(app.tabs.iter().enumerate().map(|(i, tab)| {
        row![
            button(tab.file.file_name().unwrap().to_str().unwrap())
                .style(move |theme: &iced::Theme, status| {
                    if app.get_current_tab().is_some() && tab == app.get_current_tab().unwrap() {
                        button::primary(theme, status)
                    } else {
                        button::secondary(theme, status)
                    }
                })
                .on_press(Message::TabChanged(i)),
            button("X")
                .on_press(Message::TabClosed(i))
                .style(button::danger),
            horizontal_space().width(Length::Fixed(15.)),
        ]
        .into()
    }))
    .into()
}

fn view_data_rows(app: &LogoscopeApp) -> Element<Message> {
    let Some(current_session) = app.get_current_session() else {
        return horizontal_space().into();
    };

    // Have to invert the scoll position to reverse the slider rendering
    let scroll_pos = current_session
        .rows
        .len()
        .saturating_sub(current_session.scroll_pos as usize);
    let selected_rows = &app.get_current_tab().unwrap().selected_rows;
    let rows = &app.get_current_session().unwrap().rows;
    keyed_column(rows.iter().enumerate().filter_map(|(i, row)| {
        if i < scroll_pos || i > scroll_pos.saturating_add(ROW_BUFFER_SIZE) {
            return None;
        }

        Some((
            i,
            row![
                button("    ").style(move |theme: &iced::Theme, status| {
                    if row[LogEntryIndices::Level as usize]
                        .to_lowercase()
                        .contains("warn")
                    {
                        button::Style {
                            background: Some(Background::Color(iced::Color {
                                r: 255.,
                                g: 117.,
                                b: 24.,
                                a: 255.,
                            })),
                            ..button::Style::default()
                        }
                    } else if row[LogEntryIndices::Level as usize]
                        .to_lowercase()
                        .contains("err")
                    {
                        button::danger(theme, status)
                    } else if row[LogEntryIndices::Level as usize]
                        .to_lowercase()
                        .contains("deb")
                    {
                        button::Style {
                            background: Some(Background::Color(iced::Color {
                                r: 255.,
                                g: 255.,
                                a: 255.,
                                ..iced::Color::default()
                            })),
                            ..button::Style::default()
                        }
                    } else {
                        button::secondary(theme, status)
                    }
                }),
                button({
                    row![
                        text(format!("{:<30}", &row[LogEntryIndices::Date as usize]).to_owned())
                            .wrapping(text::Wrapping::None)
                            .size(app.text_size),
                        text(format!("{:<10}", &row[LogEntryIndices::Level as usize]).to_owned())
                            .wrapping(text::Wrapping::None)
                            .size(app.text_size)
                            .width(Length::Fixed(100.)),
                        text(&row[LogEntryIndices::Log as usize])
                            .size(app.text_size)
                            .wrapping(text::Wrapping::None)
                    ]
                })
                .style(move |theme: &iced::Theme, status| {
                    if selected_rows.contains(&i) {
                        return button::primary(theme, status);
                    }
                    button::secondary(theme, status)
                })
                .width(Length::Fill)
                .on_press(Message::RowClicked(i))
            ]
            .into(),
        ))
    }))
    .width(Length::Fill)
    .into()
}

pub fn view(app: &LogoscopeApp) -> Element<Message> {
    let new_tab_button = button(if !app.loading_in_progress {
        text("+")
    } else {
        text("Loading".to_owned())
    })
    .on_press(if !app.loading_in_progress {
        Message::FileOpen
    } else {
        Message::None
    })
    .style(button::success);

    let navbar_bottom_padding = vertical_space().height(Length::Fixed(3.));

    if app.tabs.is_empty() {
        return column![view_top_navbar(app), navbar_bottom_padding, new_tab_button].into();
    }

    let scroll_pos_max = if let Some(current_session) = app.get_current_session() {
        current_session.rows.len() as f32
    } else {
        SCROLL_END
    };

    column![
        view_top_navbar(app),
        navbar_bottom_padding,
        row![view_tabs(app), new_tab_button],
        row![
            view_data_rows(app),
            vertical_slider(
                SCROLL_END..=scroll_pos_max,
                app.get_current_session()
                    .unwrap_or(&Table::default())
                    .scroll_pos,
                Message::Scrolled
            )
        ]
    ]
    .into()
}
