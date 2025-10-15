use crate::app::{LogoscopeApp, ROW_BUFFER_SIZE, SCROLL_END};
use crate::messages::Message;
use crate::parser::LogEntryIndices;
use crate::tab::{Tab, TabType};
use crate::theme::AppTheme;

use iced::widget::toggler;
use iced::widget::vertical_slider;
use iced::widget::{pick_list, rich_text};
use iced::{Background, alignment};
use iced::{
    Element, Font, Length,
    widget::{
        button, column, horizontal_space, keyed_column, row, span, text, text_input, vertical_space,
    },
};

use crate::table::{self, Table};

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
        button(text("<")).on_press(Message::PrevSearch),
        text_input("Comma-separated keywords...", &app.search.join(","))
            .width(Length::FillPortion(20))
            .on_input(Message::SearchChanged),
        button(text(">")).on_press(Message::NextSearch),
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
    row(app.tabs.iter().enumerate().filter_map(|(i, tab)| {
        let is_combined_tab = if let TabType::Combined = tab.tab_type {
            true
        } else {
            false
        };

        let file_name = if is_combined_tab {
            "Combined"
        } else {
            tab.file
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
        };

        Some(
            row![
                button(file_name)
                    .style(move |theme: &iced::Theme, status| {
                        if app.get_current_tab().is_some() && tab == app.get_current_tab().unwrap()
                        {
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
            .into(),
        )
    }))
    .into()
}

fn highlight_search_matches<'a>(
    text: &'a str,
    keywords: &Vec<String>,
    text_size: f32,
) -> Element<'a, Message> {
    // A mutable vector to hold all start and end positions of found keywords.
    let mut keyword_positions_in_text = vec![];

    // Search for all occurrences of each keyword, not just the first.
    // The search logic is now inside a loop that iterates for each keyword.
    for keyword in keywords {
        if keyword.is_empty() {
            continue;
        }

        let mut last_keyword_end_absolute = 0;
        let keyword_lowercase = keyword.to_lowercase();
        let text_lowercase = text.to_lowercase();

        // Use a while loop to find all instances of the current keyword.
        // The search starts from the last found position to avoid infinite loops and find all matches.
        while let Some(keyword_begin_relative) =
            text_lowercase[last_keyword_end_absolute..].find(&keyword_lowercase)
        {
            let keyword_begin_absolute = last_keyword_end_absolute + keyword_begin_relative;
            let keyword_end_absolute = keyword_begin_absolute + keyword.len();
            keyword_positions_in_text.push((keyword_begin_absolute, keyword_end_absolute));
            last_keyword_end_absolute = keyword_end_absolute;
        }
    }

    // Sort the matches by their starting position to ensure correct processing.
    keyword_positions_in_text.sort_by_key(|(start, _)| *start);

    // Merge overlapping or adjacent matches to create clean highlighted spans.
    let mut merged_positions = vec![];
    if let Some(&(mut current_start, mut current_end)) = keyword_positions_in_text.first() {
        for &(next_start, next_end) in keyword_positions_in_text.iter().skip(1) {
            if next_start <= current_end {
                // If the next match starts before or at the end of the current one, merge them.
                current_end = current_end.max(next_end);
            } else {
                // Otherwise, push the current merged span and start a new one.
                merged_positions.push((current_start, current_end));
                current_start = next_start;
                current_end = next_end;
            }
        }
        // Don't forget to push the last merged span.
        merged_positions.push((current_start, current_end));
    }

    let mut text_spans = vec![];
    let mut prev_span_end = 0;

    // Create the spans for the rich text element based on the merged positions.
    for (keyword_start, keyword_end) in merged_positions {
        // Add the plain text before the highlighted match.
        if prev_span_end < keyword_start {
            text_spans.push(span(&text[prev_span_end..keyword_start]));
        }
        // Add the highlighted text.
        text_spans.push(span(&text[keyword_start..keyword_end]).font(Font {
            weight: iced::font::Weight::Bold,
            ..Font::default()
        }));
        // Update the end of the last processed span.
        prev_span_end = keyword_end;
    }

    // Add any remaining text after the last highlighted match.
    if prev_span_end < text.len() {
        text_spans.push(span(&text[prev_span_end..]));
    }

    rich_text(text_spans)
        .wrapping(text::Wrapping::None)
        .size(text_size)
        .into()
}

fn view_data_rows(app: &LogoscopeApp) -> Element<Message> {
    let Some(current_session) = app.get_current_session() else {
        return horizontal_space().into();
    };

    // Have to invert the scroll position to reverse the slider rendering
    let scroll_pos = current_session
        .rows
        .len()
        .saturating_sub(current_session.scroll_pos as usize);
    let Some(current_tab) = app.get_current_tab() else {
        return horizontal_space().into();
    };

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
                        text(format!(
                            "{:<30}",
                            if let TabType::Combined = current_tab.tab_type {
                                &row[LogEntryIndices::FileName as usize]
                            } else {
                                ""
                            }
                        ))
                        .wrapping(text::Wrapping::None)
                        .size(app.text_size),
                        text(format!("{:<30}", &row[LogEntryIndices::Date as usize]).to_owned())
                            .wrapping(text::Wrapping::None)
                            .size(app.text_size),
                        text(format!("{:<10}", &row[LogEntryIndices::Level as usize]).to_owned())
                            .wrapping(text::Wrapping::None)
                            .size(app.text_size)
                            .width(Length::Fixed(100.)),
                        iced::widget::mouse_area(highlight_search_matches(
                            &row[LogEntryIndices::Log as usize],
                            &app.search,
                            app.text_size,
                        ))
                        .on_press(Message::RowClicked(i))
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
