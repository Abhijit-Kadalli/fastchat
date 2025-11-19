use crate::app::{App, InputMode};
use crate::types::Role;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

// Everforest Dark Palette
const BG_HARD: Color = Color::Rgb(39, 46, 51);   // #272e33
const BG_MEDIUM: Color = Color::Rgb(45, 53, 59); // #2d353b
const FG: Color = Color::Rgb(211, 198, 170);     // #d3c6aa
const GREEN: Color = Color::Rgb(167, 192, 128);  // #a7c080
const YELLOW: Color = Color::Rgb(219, 188, 127); // #dbbc7f
const BLUE: Color = Color::Rgb(127, 187, 179);   // #7fbbb3
const PURPLE: Color = Color::Rgb(214, 153, 182); // #d699b6
const AQUA: Color = Color::Rgb(131, 192, 146);   // #83c092
const GRAY: Color = Color::Rgb(133, 146, 137);   // #859289

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header / Tabline
            Constraint::Min(1),    // Messages
            Constraint::Length(3), // Input / Statusline
        ])
        .split(f.area());

    // Main background
    let main_block = Block::default().style(Style::default().bg(BG_HARD));
    f.render_widget(main_block, f.area());

    draw_header(f, app, chunks[0]);
    draw_messages(f, app, chunks[1]);
    draw_input(f, app, chunks[2]);

    if app.show_shortcuts {
        draw_shortcuts(f);
    }
    
    if app.show_stats {
        draw_stats(f, app);
    }

    if app.show_backend_selection {
        draw_backend_selection(f, app);
    }

    if app.show_url_edit {
        draw_url_input(f, app);
    }

    if app.show_history {
        draw_history_panel(f, app);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let backend_name = &app.config.active_backend;
    let model_name = app.config.get_active_backend().map(|b| b.model.as_str()).unwrap_or("Unknown");

    // LazyVim style: minimal, colored blocks
    let title = Line::from(vec![
        Span::styled("  FASTCHAT ", Style::default().fg(BG_HARD).bg(BLUE).add_modifier(Modifier::BOLD)),
        Span::styled("", Style::default().fg(BLUE).bg(BG_MEDIUM)),
        Span::styled(format!(" {} ", backend_name.to_uppercase()), Style::default().fg(FG).bg(BG_MEDIUM)),
        Span::styled("", Style::default().fg(BG_MEDIUM).bg(BG_HARD)),
        Span::styled(format!(" {} ", model_name), Style::default().fg(GRAY).bg(BG_HARD)),
    ]);

    let paragraph = Paragraph::new(title);
    f.render_widget(paragraph, area);
}

fn draw_messages(f: &mut Frame, app: &mut App, area: Rect) {
    let mut text_lines = Vec::new();
    
    for m in &app.messages {
        let (role_style, icon) = match m.role {
            Role::User => (Style::default().fg(BLUE), ""),
            Role::Assistant => (Style::default().fg(GREEN), ""),
            Role::System => (Style::default().fg(YELLOW), ""),
        };

        let header = Line::from(vec![
            Span::styled(format!("{} ", icon), role_style),
            Span::styled(format!("{:?}", m.role).to_uppercase(), role_style.add_modifier(Modifier::BOLD)),
        ]);
        
        text_lines.push(header);
        text_lines.extend(parse_markdown(&m.content));
        text_lines.push(Line::from("")); // Spacer
    }

    let paragraph = Paragraph::new(text_lines)
        .block(Block::default().borders(Borders::NONE).style(Style::default().bg(BG_HARD)))
        .style(Style::default().bg(BG_HARD))
        .wrap(Wrap { trim: true })
        .scroll((app.scroll, 0));

    f.render_widget(paragraph, area);
}

fn parse_markdown(content: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line in content.lines() {
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            lines.push(Line::from(Span::styled(line, Style::default().fg(GRAY))));
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(line, Style::default().fg(YELLOW).bg(BG_MEDIUM))));
        } else {
            // Basic inline parsing (bold, code)
            let mut spans = Vec::new();
            let mut current_text = String::new();
            let mut chars = line.chars().peekable();
            
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                    // Flush current text
                    if !current_text.is_empty() {
                        spans.push(Span::styled(current_text.clone(), Style::default().fg(FG)));
                        current_text.clear();
                    }
                    
                    // Read until next **
                    let mut bold_text = String::new();
                    while let Some(bc) = chars.next() {
                        if bc == '*' && chars.peek() == Some(&'*') {
                            chars.next();
                            break;
                        }
                        bold_text.push(bc);
                    }
                    spans.push(Span::styled(bold_text, Style::default().fg(FG).add_modifier(Modifier::BOLD)));
                } else if c == '`' {
                    // Flush current text
                    if !current_text.is_empty() {
                        spans.push(Span::styled(current_text.clone(), Style::default().fg(FG)));
                        current_text.clear();
                    }
                    
                    // Read until next `
                    let mut code_text = String::new();
                    while let Some(cc) = chars.next() {
                        if cc == '`' {
                            break;
                        }
                        code_text.push(cc);
                    }
                    spans.push(Span::styled(code_text, Style::default().fg(AQUA)));
                } else {
                    current_text.push(c);
                }
            }
            
            if !current_text.is_empty() {
                spans.push(Span::styled(current_text, Style::default().fg(FG)));
            }
            
            lines.push(Line::from(spans));
        }
    }
    
    lines
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if let InputMode::Editing = app.input_mode {
        AQUA
    } else {
        GRAY
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(" Input ", Style::default().fg(border_color)));

    let input_text = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(FG).bg(BG_HARD))
        .block(input_block)
        .wrap(Wrap { trim: false });

    f.render_widget(input_text, area);

    if let InputMode::Editing = app.input_mode {
        f.set_cursor_position((
            area.x + app.input.len() as u16 + 1,
            area.y + 1,
        ));
    }
}

use ratatui::widgets::{Table, Row, Cell};

fn draw_shortcuts(f: &mut Frame) {
    // Position at the bottom, full width (LazyVim style)
    let area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(12), // Height of the shortcuts menu
        ])
        .split(f.area())[1];

    let block = Block::default()
        .title(" Shortcuts ")
        .borders(Borders::TOP)
        .border_style(Style::default().fg(BLUE))
        .style(Style::default().bg(BG_MEDIUM).fg(FG));
    
    let rows = vec![
        Row::new(vec![
            Cell::from(Span::styled("Space", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))),
            Cell::from("Toggle this menu"),
        ]),
        Row::new(vec![
            Cell::from(Span::styled("Space+e", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))),
            Cell::from("Toggle Chat History"),
        ]),
        Row::new(vec![
            Cell::from(Span::styled("i", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))),
            Cell::from("Enter Input Mode"),
        ]),
        Row::new(vec![
            Cell::from(Span::styled("n", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))),
            Cell::from("New Chat"),
        ]),
        Row::new(vec![
            Cell::from(Span::styled("s", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))),
            Cell::from("Toggle Stats"),
        ]),
        Row::new(vec![
            Cell::from(Span::styled("c", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))),
            Cell::from("Clear Current Chat"),
        ]),
        Row::new(vec![
            Cell::from(Span::styled("q", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))),
            Cell::from("Quit"),
        ]),
        Row::new(vec![
            Cell::from(Span::styled("b", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))),
            Cell::from("Backend Selection"),
        ]),
        Row::new(vec![
            Cell::from(Span::styled("Esc", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))),
            Cell::from("Back / Normal Mode"),
        ]),
    ];
    
    let table = Table::new(rows, [Constraint::Length(10), Constraint::Min(1)])
        .block(block)
        .column_spacing(2);
        
    f.render_widget(ratatui::widgets::Clear, area); // Clear background
    f.render_widget(table, area);
}

fn draw_stats(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 25, f.area());
    let block = Block::default()
        .title(" Statistics ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .style(Style::default().bg(BG_MEDIUM).fg(FG));
    
    let duration = app.session_start.elapsed();
    let duration_str = format!("{:02}:{:02}:{:02}", 
        duration.as_secs() / 3600,
        (duration.as_secs() % 3600) / 60,
        duration.as_secs() % 60
    );

    let text = vec![
        Line::from(vec![Span::styled("Session Duration: ", Style::default().fg(BLUE)), Span::raw(duration_str)]),
        Line::from(""),
        Line::from(vec![Span::styled("User Messages:    ", Style::default().fg(GREEN)), Span::raw(app.user_msg_count.to_string())]),
        Line::from(vec![Span::styled("Assistant Msgs:   ", Style::default().fg(GREEN)), Span::raw(app.assistant_msg_count.to_string())]),
        Line::from(""),
        Line::from(vec![Span::styled("Active Backend:   ", Style::default().fg(PURPLE)), Span::raw(app.config.active_backend.clone())]),
    ];
    
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: true });
        
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_backend_selection(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 40, f.area());
    let block = Block::default()
        .title(" Select Backend ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .style(Style::default().bg(BG_MEDIUM).fg(FG));
    
    let mut items = Vec::new();
    for (i, backend) in app.available_backends.iter().enumerate() {
        let style = if i == app.backend_selection_index {
            Style::default().fg(BG_HARD).bg(BLUE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG)
        };
        
        let prefix = if i == app.backend_selection_index { "> " } else { "  " };
        items.push(Line::from(vec![
            Span::styled(format!("{}{}", prefix, backend), style)
        ]));
    }
    
    let paragraph = Paragraph::new(items)
        .block(block)
        .alignment(ratatui::layout::Alignment::Left);
        
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(paragraph, area);
}

fn draw_url_input(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    let block = Block::default()
        .title(" Edit Backend URL ")
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(AQUA))
        .style(Style::default().bg(BG_MEDIUM).fg(FG));
    
    let text = vec![
        Line::from(Span::styled("Enter new URL:", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled(&app.url_input, Style::default().fg(FG).bg(BG_HARD))),
    ];
    
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: false });
        
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(paragraph, area);
    
    // Draw cursor
    // Calculate cursor position relative to the popup
    // This is a simplified cursor positioning
    let input_area = area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    // The input text is on the 3rd line (index 2) of the paragraph
    // But paragraph rendering is complex. 
    // For simplicity, let's just render the cursor at the end of the input string manually if possible
    // or rely on the user seeing the text update.
    // A proper implementation would calculate the exact screen coordinates.
    
    f.set_cursor_position((
        input_area.x + app.url_input.len() as u16,
        input_area.y + 2,
    ));
}

fn draw_history_panel(f: &mut Frame, app: &App) {
    // Create a left-side panel, similar to LazyVim's file explorer
    let area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // History panel takes 30% of width
            Constraint::Percentage(70), // Rest for main content
        ])
        .split(f.area())[0];

    // Create title with keybindings hint
    let title = Line::from(vec![
        Span::styled(" Chat History ", Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)),
        Span::styled("(n)ew ", Style::default().fg(GRAY)),
        Span::styled("(d)elete ", Style::default().fg(GRAY)),
        Span::styled("(r)efresh ", Style::default().fg(GRAY)),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(PURPLE))
        .style(Style::default().bg(BG_MEDIUM).fg(FG));

    if app.history.is_empty() {
        let empty_text = Paragraph::new("No chat history yet.\nStart a new conversation!")
            .block(block)
            .style(Style::default().fg(GRAY))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(ratatui::widgets::Clear, area);
        f.render_widget(empty_text, area);
        return;
    }

    let mut items = Vec::new();
    for (i, session) in app.history.iter().enumerate() {
        let is_selected = i == app.selected_chat_index;

        // Format timestamp
        let timestamp = session.created_at.format("%m/%d %H:%M").to_string();

        // Get session name (truncate if too long)
        let name = if session.name.len() > 35 {
            format!("{}...", &session.name[..32])
        } else {
            session.name.clone()
        };

        // Message count
        let msg_count = session.messages.len();

        if is_selected {
            items.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(AQUA).add_modifier(Modifier::BOLD)),
                Span::styled(name.clone(), Style::default().fg(BG_HARD).bg(AQUA).add_modifier(Modifier::BOLD)),
            ]));
            items.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("  {} msgs  {}", msg_count, timestamp),
                    Style::default().fg(GRAY)),
            ]));
        } else {
            items.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(name.clone(), Style::default().fg(FG)),
            ]));
            items.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("  {} msgs  {}", msg_count, timestamp),
                    Style::default().fg(GRAY)),
            ]));
        }

        // Add a blank line between items
        if i < app.history.len() - 1 {
            items.push(Line::from(""));
        }
    }

    // Add footer with directory path
    let chats_dir = crate::storage::get_chats_dir();
    let footer_text = format!(" {} ", chats_dir.display());
    let block_with_footer = block.title_bottom(
        Line::from(Span::styled(footer_text, Style::default().fg(GRAY)))
    );

    let paragraph = Paragraph::new(items)
        .block(block_with_footer)
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(paragraph, area);
}
