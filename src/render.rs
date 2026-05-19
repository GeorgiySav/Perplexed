use std::{io::Write};

use crossterm::style::{Color, Stylize};

use crate::context::Citation;

#[derive(PartialEq)]
enum State {
    Normal,
    InBrackets
}

pub struct CitationGuard {
    max_n: usize,
    state: State,
    digits: String
}

impl CitationGuard {
    pub fn new(max_n: usize) -> Self {
        CitationGuard{
            max_n,
            state: State::Normal,
            digits: String::new()
        }
    }

    fn feed_char(&mut self, ch: char) {
        if self.state == State::InBrackets {
            if ch.is_ascii_digit() {
                self.digits.push(ch);
            }
            else if ch == ']' {
                if !self.digits.is_empty() {
                    let n = self.digits.parse::<usize>().unwrap();
                    if 1 <= n && n <= self.max_n {
                        print!("[{}]", n);
                    }
                    else {
                        print!("[?]");
                    }
                }
                self.state = State::Normal;
                self.digits.clear();
            }
            else {
                print!("[{}", self.digits);
                self.state = State::Normal;
                self.digits.clear();
                self.feed_char(ch);
            }
        }
        else if self.state == State::Normal {
            if ch == '[' {
                self.state = State::InBrackets;
            }
            else {
                print!("{}", ch);
            }
        }
    }

    pub fn feed(&mut self, token: &str) {
        token.chars().for_each(|c| self.feed_char(c));
        let _ = std::io::stdout().flush();
    }

    pub fn flush(&mut self) {
        if self.state == State::InBrackets {
            print!("[{}", self.digits);
            self.state = State::Normal;
            self.digits.clear();
        }
        let _ = std::io::stdout().flush();
    }
}

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