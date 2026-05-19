use crate::pipeline;
use crate::llm::Message;

use rustyline::{DefaultEditor, history};
use rustyline::error::ReadlineError;

const MAX_HISTORY_MESSAGES: usize = 20;

pub async fn run() -> anyhow::Result<()> {
    println!("╭─ Perplexed ─────────────────────
│
│  Ask anything. /exit to quit.
│");

    let mut rl = DefaultEditor::new()?;

    let mut history: Vec<Message> = Vec::new();

    loop {
        let line = rl.readline("> ");

        match line {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() { continue; }
                if line == "/exit" || line == "/quit" { break; }
                if line == "/reset" {
                    history.clear();
                    println!("History Cleared");
                    continue;
                }
                let _ = rl.add_history_entry(line);

                match pipeline::answer(line, &history).await {
                    Ok(response) => {
                        history.push(Message { role: "user".to_string(), content: line.to_string() });
                        history.push(Message { role: "assistant".to_string(), content: response });

                        if history.len() > MAX_HISTORY_MESSAGES {
                            let excess = history.len() - MAX_HISTORY_MESSAGES;
                            history.drain(0..excess);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {:#}", e);
                    }
                }
            },
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => { eprintln!("Error: {:#}", e); break; }
        }
    }
    Ok(())
}