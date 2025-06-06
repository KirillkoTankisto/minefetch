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
use tokio::io::{self, AsyncBufReadExt, BufReader};

/// Macro for async std output (with \n)
#[macro_export]
macro_rules! async_println {
    ($($arg:tt)*) => {{
        async {
            // Set the output to async stdout
            let mut stdout = tokio::io::BufWriter::new(tokio::io::stdout());

            // Write the input into stdout
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                &mut stdout,
                format!($($arg)*).as_bytes()
            ).await {
                eprintln!("Error writing to stdout: {}", e);
            }

            // Write newline into stdout
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                &mut stdout,
                b"\n"
            ).await {
                eprintln!("Error writing newline to stdout: {}", e);
            }

            /*
                Flush the text to make sure that
                everything has been written into stdout
            */
            if let Err(e) = tokio::io::AsyncWriteExt::flush(&mut stdout).await {
                eprintln!("Error flushing stdout: {}", e);
            }
        }
    }}
}

/// Macro for async stderr output (with \n)
#[macro_export]
macro_rules! async_eprintln {
    ($($arg:tt)*) => {{
        async {
            // Set the output to async stderr
            let mut stderr = tokio::io::BufWriter::new(tokio::io::stderr());

            // Write the input into the stderr
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut stderr, format!($($arg)*).as_bytes()).await {
                eprintln!(":err: Error writing to stderr: {}", e);
            }

            // Write newline into the stderr
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut stderr, b"\n").await {
                eprintln!(":err: Error writing newline to stderr: {}", e);
            }

            /*
                Flush the text to make sure that
                everything has been written into stderr
            */
            if let Err(e) = tokio::io::AsyncWriteExt::flush(&mut stderr).await {
                eprintln!(":err: Error flushing stderr: {}", e);
            }
        }
    }}
}

/// Macro for async std output (without \n)
#[macro_export]
macro_rules! async_print {
    ($($arg:tt)*) => {{
        async {
            // Set the output to async stdout
            let mut stdout = tokio::io::BufWriter::new(tokio::io::stdout());

            // Write the input into stdout
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                &mut stdout,
                format!($($arg)*).as_bytes()
            ).await {
                eprintln!(":err: Error writing to stdout: {}", e);
            }

            /*
                Flush the text to make sure that
                everything has been written into stdout
            */
            if let Err(e) = tokio::io::AsyncWriteExt::flush(&mut stdout).await {
                eprintln!(":err: Error flushing stdout: {}", e);
            }
        }
    }}
}

/// Press enter to continue functionality
pub async fn press_enter() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Set the stdout (output)
    let term = Term::stdout();

    // The start of the loop
    loop {
        // Read a key
        let key = term.read_key()?;

        // Check which key is pressed
        match key {
            // If user pressed 'enter' then stop
            Key::Enter => break,

            // If user pressed 'q' then stop with the error
            Key::Char('q') => return Err("The operation has been cancelled".into()),

            // If user pressed something else then continue
            _ => (),
        }
    }

    // Success
    Ok(())
}

/// Reads user input and returns a String
pub async fn ainput(prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Create a buffer
    let mut buffer = String::new();

    // Create a reader
    let mut reader = BufReader::new(tokio::io::stdin());

    // Print the prompt
    async_print!("{}", prompt).await;

    // Read the user input
    reader.read_line(&mut buffer).await?;

    // Return the reformatted input
    Ok(buffer.trim().to_string())
}

/// Parses String to Vec<usize>
pub fn parse_to_int(
    string: String,
) -> Result<Vec<usize>, Box<dyn std::error::Error + Send + Sync>> {
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
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MFText {
    // Foreground colors
    FgBlack,
    FgRed,
    FgGreen,
    FgYellow,
    FgBlue,
    FgMagenta,
    FgCyan,
    FgWhite,
    // Background colors
    BgBlack,
    BgRed,
    BgGreen,
    BgYellow,
    BgBlue,
    BgMagenta,
    BgCyan,
    BgWhite,
    // Text styles
    Bold,
    Italic,
    Underline,
    Blink,
    // Reset formatting
    Reset,
}

/// Formatting for MFText
impl MFText {
    pub fn code(&self) -> &'static str {
        match self {
            // Foreground colors
            MFText::FgBlack => "\x1b[30m",
            MFText::FgRed => "\x1b[31m",
            MFText::FgGreen => "\x1b[32m",
            MFText::FgYellow => "\x1b[33m",
            MFText::FgBlue => "\x1b[34m",
            MFText::FgMagenta => "\x1b[35m",
            MFText::FgCyan => "\x1b[36m",
            MFText::FgWhite => "\x1b[37m",
            // Background colors
            MFText::BgBlack => "\x1b[40m",
            MFText::BgRed => "\x1b[41m",
            MFText::BgGreen => "\x1b[42m",
            MFText::BgYellow => "\x1b[43m",
            MFText::BgBlue => "\x1b[44m",
            MFText::BgMagenta => "\x1b[45m",
            MFText::BgCyan => "\x1b[46m",
            MFText::BgWhite => "\x1b[47m",
            // Text styles
            MFText::Bold => "\x1b[1m",
            MFText::Italic => "\x1b[3m",
            MFText::Underline => "\x1b[4m",
            MFText::Blink => "\x1b[5m",
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
