use std::io::Write;

use crossterm::style::{Color, Stylize};

use crate::context::Citation;

pub fn print_token(token: &str) -> () {
    print!("{}", token);
    let _ = std::io::stdout().flush();
}

pub fn print_sources(citations: &[Citation]) -> () {
    println!("╭─ Sources ─");
    println!("│");

    for citation in citations.iter() {
        println!("│ [{}]  {}", format!("{}", citation.n).cyan(), citation.title);
        println!("│       {}", format!("{}", citation.url).blue().underline(Color::Blue));
        println!("│");
    }
}