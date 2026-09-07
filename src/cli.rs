use crate::context::Citation;
use crate::llm::Message;
use crate::pipeline::{self, PipelineEvent};
use crate::markdown;

use std::io::stdout;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, error::TryRecvError};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

const MAX_HISTORY_MESSAGES: usize = 20;

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Conversation,
    Sources,
}

struct App {
    input_buffer: String,
    history: Vec<Message>,
    conversation: Vec<(String, String)>,
    sources: Vec<Citation>,

    current_query: Option<String>,
    current_response: String,

    focus: Focus,
    conversation_scroll: u16,
    conversation_max_scroll: u16,
    conversation_locked_bottom: bool,
    sources_scroll: u16,
    sources_max_scroll: u16,
}

impl App {
    fn new() -> Self {
        Self {
            input_buffer: String::new(),
            history: Vec::new(),
            conversation: Vec::new(),
            sources: Vec::new(),

            current_query: None,
            current_response: String::new(),

            focus: Focus::Conversation,
            conversation_scroll: 0,
            conversation_max_scroll: 0,
            conversation_locked_bottom: true,
            sources_scroll: 0,
            sources_max_scroll: 0,
        }
    }
}

fn handle_pipe_event(app: &mut App, event: PipelineEvent) -> bool {
    match event {
        PipelineEvent::Sources(citations) => {
            app.sources = citations;
            app.sources_scroll = 0;
            false
        }
        PipelineEvent::Token(t) => {
            app.current_response.push_str(&t);
            false
        }
        PipelineEvent::Done => {
            if let Some(query) = app.current_query.take() {
                let response = std::mem::take(&mut app.current_response);
                app.conversation.push((query.clone(), response.clone()));
                app.history.push(Message { role: "user".to_string(), content: query });
                app.history.push(Message { role: "assistant".to_string(), content: response });
                if app.history.len() > MAX_HISTORY_MESSAGES {
                    let excess = app.history.len() - MAX_HISTORY_MESSAGES;
                    app.history.drain(0..excess);
                }
            }
            true
        }
        PipelineEvent::Error(msg) => {
            if let Some(query) = app.current_query.take() {
                app.conversation.push((query, format!("(error) {}", msg)));
            }
            app.current_response.clear();
            true
        }
    }
}

fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub async fn run() -> anyhow::Result<()> {
    std::panic::set_hook(Box::new(|panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        eprintln!("{panic_info}");
    }));

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut pipe_rx: Option<UnboundedReceiver<PipelineEvent>> = None;

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(15),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            // ─── Conversation pane ───
            let mut lines: Vec<Line> = Vec::new();
            if app.conversation.is_empty() && app.current_query.is_none() {
                lines.push(Line::from(Span::styled(
                    "Welcome to Perplexed",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )));
            } else {
                for (query, response) in &app.conversation {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "> ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(query.as_str()),
                    ]));
                    lines.push(Line::from(""));
                    lines.extend(markdown::render_markdown(response));
                }

                if let Some(query) = &app.current_query {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "> ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(query.as_str()),
                    ]));
                    lines.push(Line::from(""));

                    if app.current_response.is_empty() {
                        lines.push(Line::from(Span::styled(
                            " generating...",
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        )));
                    } else {
                        lines.extend(markdown::render_markdown(&app.current_response));
                    }
                    lines.push(Line::from(""));
                }
            }

            let conv_focused = app.focus == Focus::Conversation;
            let conversation_paragraph = Paragraph::new(Text::from(lines))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Perplexed")
                        .border_style(border_style(conv_focused)),
                )
                .wrap(Wrap { trim: false });

            // Scroll bookkeeping based on actual wrapped row count, not raw line count.
            let conv_inner_width = chunks[0].width.saturating_sub(2);
            let conv_visible = chunks[0].height.saturating_sub(2) as usize;
            let conv_total = conversation_paragraph.line_count(conv_inner_width);
            let conv_max = conv_total.saturating_sub(conv_visible) as u16;
            app.conversation_max_scroll = conv_max;
            if app.conversation_locked_bottom {
                app.conversation_scroll = conv_max;
            } else if app.conversation_scroll > conv_max {
                app.conversation_scroll = conv_max;
            }

            frame.render_widget(
                conversation_paragraph.scroll((app.conversation_scroll, 0)),
                chunks[0],
            );

            // ─── Sources pane ───
            let mut source_lines: Vec<Line> = Vec::new();
            if app.sources.is_empty() {
                source_lines.push(Line::from(Span::styled(
                    "(no sources yet)",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )));
            } else {
                for citation in &app.sources {
                    source_lines.push(Line::from(vec![
                        Span::styled(
                            format!("[{}] ", citation.n),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw(citation.title.as_str()),
                    ]));
                    source_lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            citation.url.as_str(),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
            }

            let src_focused = app.focus == Focus::Sources;
            let sources_paragraph = Paragraph::new(Text::from(source_lines))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Sources")
                        .border_style(border_style(src_focused)),
                )
                .wrap(Wrap { trim: false });

            let src_inner_width = chunks[1].width.saturating_sub(2);
            let src_visible = chunks[1].height.saturating_sub(2) as usize;
            let src_total = sources_paragraph.line_count(src_inner_width);
            let src_max = src_total.saturating_sub(src_visible) as u16;
            app.sources_max_scroll = src_max;
            if app.sources_scroll > src_max {
                app.sources_scroll = src_max;
            }

            frame.render_widget(
                sources_paragraph.scroll((app.sources_scroll, 0)),
                chunks[1],
            );

            // ─── Input pane ───
            let input = Paragraph::new(app.input_buffer.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Ask  ·  Tab: focus  ·  ↑↓: scroll  ·  Esc: quit"),
                );
            frame.render_widget(input, chunks[2]);
        })?;

        if let Some(rx) = pipe_rx.as_mut() {
            let mut done = false;
            loop {
                match rx.try_recv() {
                    Ok(event) => {
                        if handle_pipe_event(&mut app, event) {
                            done = true;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
            if done {
                pipe_rx = None;
            }
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != event::KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Esc => break,

                    KeyCode::Tab => {
                        app.focus = match app.focus {
                            Focus::Conversation => Focus::Sources,
                            Focus::Sources => Focus::Conversation,
                        };
                    }

                    KeyCode::Up => match app.focus {
                        Focus::Conversation => {
                            app.conversation_scroll = app.conversation_scroll.saturating_sub(1);
                            app.conversation_locked_bottom = false;
                        }
                        Focus::Sources => {
                            app.sources_scroll = app.sources_scroll.saturating_sub(1);
                        }
                    },
                    KeyCode::Down => match app.focus {
                        Focus::Conversation => {
                            app.conversation_scroll = (app.conversation_scroll + 1)
                                .min(app.conversation_max_scroll);
                            if app.conversation_scroll == app.conversation_max_scroll {
                                app.conversation_locked_bottom = true;
                            }
                        }
                        Focus::Sources => {
                            app.sources_scroll = (app.sources_scroll + 1)
                                .min(app.sources_max_scroll);
                        }
                    },

                    KeyCode::Char(c) => app.input_buffer.push(c),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Enter => {
                        if app.current_query.is_some() {
                            continue;
                        }

                        let trimmed = app.input_buffer.trim().to_string();
                        app.input_buffer.clear();
                        if trimmed.is_empty() {
                            continue;
                        }

                        app.current_query = Some(trimmed.clone());
                        app.current_response.clear();
                        app.conversation_locked_bottom = true;

                        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                        pipe_rx = Some(rx);

                        let history_snapshot = app.history.clone();
                        tokio::spawn(pipeline::answer_streaming(trimmed, history_snapshot, tx));
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}
