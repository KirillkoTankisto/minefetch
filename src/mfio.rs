/*
 __  __ _            _____    _       _       ___ ___
|  \/  (_)_ __   ___|  ___|__| |_ ___| |__   |_ _/ _ \
| |\/| | | '_ \ / _ \ |_ / _ \ __/ __| '_ \   | | | | |
| |  | | | | | |  __/  _|  __/ || (__| | | |  | | |_| |
|_|  |_|_|_| |_|\___|_|  \___|\__\___|_| |_| |___\___/

*/

use std::num::ParseIntError;

// External crates
use console::{Key, Term};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader, stdin, stdout};

/// Reads user input and returns a String
pub async fn ainput(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Print the prompt
    let mut stdout = stdout();
    stdout.write_all(prompt.as_bytes()).await?;
    stdout.flush().await?;

    // Create a buffer
    let mut buffer = String::new();

    // Create a reader
    let mut reader = BufReader::new(stdin());

    // Read the user input
    reader.read_line(&mut buffer).await?;

    // Return the reformatted input
    Ok(buffer.trim().to_string())
}

/// Parses String to Vec<usize>
pub fn parse_to_int(string: String) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    // Split the string
    let splitted_string = string.split(' ');

    // Create a numbers list
    let numbers: Vec<usize> = splitted_string
        .into_iter()
        .map(|character| character.parse::<usize>().map(|number| number - 1))
        .collect::<Result<Vec<usize>, ParseIntError>>()
        .unwrap_or_default(); // If failed then return an empty list
    Ok(numbers)
}

/// ANSI escape codes for formatting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MFText {
    // Style
    Bold,
    Underline,
    // Reset
    Reset,
}

/// Formatting for MFText
impl MFText {
    pub fn code(&self) -> &'static str {
        match self {
            // Style
            MFText::Bold => "\x1b[1m",
            MFText::Underline => "\x1b[4m",
            // Reset
            MFText::Reset => "\x1b[0m",
        }
    }
}

/// Display for MFText
impl std::fmt::Display for MFText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// A replacement for inquire. Works better
pub async fn select<S, T>(prompt: &str, options: Vec<(S, T)>) -> io::Result<T>
where
    S: AsRef<str>,
    T: Clone, // Makes function take any type of value which implements 'Clone' as an input
{
    if options.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No options provided",
        ));
    }

    let term = Term::stdout();
    let mut index = 0;
    let total = options.len();

    Term::hide_cursor(&term)?;

    loop {
        // Get terminal size a the moment
        let (rows, columns) = term.size();

        let (viewport_size, columns) = (rows as usize - 2, columns as usize);
        term.clear_screen()?;
        println!(":out: {prompt}");

        let start = if total <= viewport_size {
            0
        } else if index < viewport_size / 2 {
            0
        } else if index > total - (viewport_size / 2 + 1) {
            total - viewport_size
        } else {
            index - viewport_size / 2
        };

        let end = (start + viewport_size).min(total);
        for (i, (label, _)) in options.iter().enumerate().skip(start).take(end - start) {
            let max_width = columns as usize - 3;

            let raw_string = label.as_ref();

            let display = if raw_string.len() > max_width {
                format!("{}...", &raw_string[..max_width.saturating_sub(3)])
            } else {
                raw_string.to_string()
            };

            println!("{}{}", if i == index { ">> " } else { "   " }, display);
        }

        match term.read_key()? {
            Key::ArrowUp => index = (index + options.len() - 1) % options.len(),
            Key::ArrowDown => index = (index + 1) % options.len(),
            Key::Enter => {
                term.clear_last_lines(end - start)?;
                Term::show_cursor(&term)?;
                return Ok(options[index].1.clone());
            }
            Key::Escape | Key::Char('q') => {
                term.clear_last_lines(end - start)?;
                Term::show_cursor(&term)?;
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Selection cancelled",
                ));
            }
            _ => {}
        }

        term.clear_last_lines(end - start)?;
    }
}
