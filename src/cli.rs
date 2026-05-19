use crate::{context::Citation, pipeline};
use crate::llm::Message;

use std::io::stdout;
use std::time::Duration;
use rustyline::history;
use tokio::sync::mpsc::{UnboundedReceiver, error::TryRecvError};
use crate::pipeline::{PipelineEvent};

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

struct App {
    input_buffer: String,
    history: Vec<Message>,
    conversation: Vec<(String, String)>,
    sources: Vec<Citation>,

    current_query: Option<String>,
    current_response: String
}

impl App {
    fn new() -> Self {
        Self {
            input_buffer: String::new(),
            history: Vec::new(),
            conversation: Vec::new(),
            sources: Vec::new(),

            current_query: None,
            current_response: String::new()
        }
    }
}

fn handle_pipe_event(app: &mut App, event: PipelineEvent) -> bool {
    match event {
        PipelineEvent::Sources(citations) => {
            app.sources = citations;
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
                    Constraint::Min(1),    // conversation
                    Constraint::Length(8), // sources
                    Constraint::Length(3), // input
                ])
                .split(frame.area());

            // conversation pane
            let mut lines: Vec<Line> = Vec::new();
            if app.conversation.is_empty() {
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
                            "❯ ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(query.as_str()),
                    ]));
                    lines.push(Line::from(""));
                    for response_line in response.lines() {
                        lines.push(Line::from(response_line.to_string()));
                    }
                    lines.push(Line::from(""));
                }
            }

            if let Some(query) = &app.current_query {
                lines.push(Line::from(vec![
                    Span::styled(
                        "❯ ",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(query.as_str()),
                ]));
                lines.push(Line::from(""));

                if app.current_response.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "▍ generating...",
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                    )));
                } else {
                    for line in app.current_response.lines() {
                        lines.push(Line::from(line.to_string()));
                    } 
                }
                lines.push(Line::from(""));
            }

            let conversation_widget = Paragraph::new(Text::from(lines))
                .block(Block::default().borders(Borders::ALL).title("Perplexed"))
                .wrap(Wrap { trim: false });
            frame.render_widget(conversation_widget, chunks[0]);

            // sources pane
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
            let sources_widget = Paragraph::new(Text::from(source_lines))
                .block(Block::default().borders(Borders::ALL).title("Sources"))
                .wrap(Wrap { trim: false });
            frame.render_widget(sources_widget, chunks[1]);

            // input pane
            let input = Paragraph::new(app.input_buffer.as_str())
                .block(Block::default().borders(Borders::ALL).title("Ask"));
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
                    KeyCode::Char(c) => app.input_buffer.push(c),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Enter => {
                        if app.current_query.is_some() { continue; }

                        let trimmed = app.input_buffer.trim().to_string();
                        app.input_buffer.clear();
                        if trimmed.is_empty() { continue; }

                        app.current_query = Some(trimmed.clone());
                        app.current_response.clear();

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
