use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let parser = Parser::new(text);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut list_starts: Vec<Option<u64>> = Vec::new();
    let mut list_counters: Vec<u64> = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Paragraph) => {
                flush_line(&mut lines, &mut current_spans);
            }
            Event::End(TagEnd::Paragraph) => {
                flush_line(&mut lines, &mut current_spans);
                lines.push(Line::from(""));
            }
            Event::Start(Tag::Strong) => {
                let s = current_style(&style_stack).add_modifier(Modifier::BOLD);
                style_stack.push(s);
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                let s = current_style(&style_stack).add_modifier(Modifier::ITALIC);
                style_stack.push(s);
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }
            Event::Code(code) => {
                let s = current_style(&style_stack).fg(Color::Yellow);
                current_spans.push(Span::styled(code.to_string(), s));
            }
            Event::Text(text) => {
                let s = current_style(&style_stack);
                current_spans.push(Span::styled(text.to_string(), s));
            }
            Event::SoftBreak => {
                let s = current_style(&style_stack);
                current_spans.push(Span::styled(" ", s));
            }
            Event::HardBreak => {
                flush_line(&mut lines, &mut current_spans);
            }
            Event::Start(Tag::List(start_num)) => {
                flush_line(&mut lines, &mut current_spans);
                list_starts.push(start_num);
                list_counters.push(start_num.unwrap_or(0));
            }
            Event::End(TagEnd::List(_)) => {
                list_starts.pop();
                list_counters.pop();
                flush_line(&mut lines, &mut current_spans);
            }
            Event::Start(Tag::Item) => {
                flush_line(&mut lines, &mut current_spans);
                let depth = list_starts.len().saturating_sub(1);
                let indent = "  ".repeat(depth);
                let prefix = match list_starts.last() {
                    Some(Some(_)) => {
                        let n = list_counters.last().copied().unwrap_or(0);
                        if let Some(counter) = list_counters.last_mut() {
                            *counter += 1;
                        }
                        format!("{}{}. ", indent, n)
                    }
                    _ => format!("{}• ", indent),
                };
                current_spans.push(Span::raw(prefix));
            }
            Event::End(TagEnd::Item) => {
                flush_line(&mut lines, &mut current_spans);
            }
            _ => {}
        }
    }
    flush_line(&mut lines, &mut current_spans);

    lines
}

fn current_style(stack: &[Style]) -> Style {
    stack.last().copied().unwrap_or_default()
}

fn flush_line(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(std::mem::take(spans)));
    }
}
