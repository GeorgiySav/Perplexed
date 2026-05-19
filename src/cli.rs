use crate::pipeline;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

pub async fn run() -> anyhow::Result<()> {
    println!("╭─ Perplexed ─────────────────────
│
│  Ask anything. /exit to quit.
│");

    let mut rl = DefaultEditor::new()?;

    loop {
        let line = rl.readline("> ");

        match line {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() { continue; }
                if line == "/exit" || line == "/quit" { break; }
                let _ = rl.add_history_entry(line);

                if let Err(e) = pipeline::answer(&line).await {
                    eprintln!("Error: {:#}", e);
                }
            },
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => { eprintln!("Error: {:#}", e); break; }
        }
    }
    Ok(())
}