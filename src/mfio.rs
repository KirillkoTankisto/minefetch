/*
 __  __ _            _____    _       _       ___ ___
|  \/  (_)_ __   ___|  ___|__| |_ ___| |__   |_ _/ _ \
| |\/| | | '_ \ / _ \ |_ / _ \ __/ __| '_ \   | | | | |
| |  | | | | | |  __/  _|  __/ || (__| | | |  | | |_| |
|_|  |_|_|_| |_|\___|_|  \___|\__\___|_| |_| |___\___/

*/

// External crates
use console::{Key, Term};
use tokio::io::{self, AsyncBufReadExt, BufReader};

/// Macro for async std output (with \n)
#[macro_export]
macro_rules! async_println {
    ($($arg:tt)*) => {{
        async {
            let mut stdout = tokio::io::BufWriter::new(tokio::io::stdout());
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                &mut stdout,
                format!($($arg)*).as_bytes()
            ).await {
                eprintln!("Error writing to stdout: {}", e);
            }
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                &mut stdout,
                b"\n"
            ).await {
                eprintln!("Error writing newline to stdout: {}", e);
            }
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
            let mut stderr = tokio::io::BufWriter::new(tokio::io::stderr());
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut stderr, format!($($arg)*).as_bytes()).await {
                eprintln!(":err: Error writing to stderr: {}", e);
            }
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut stderr, b"\n").await {
                eprintln!(":err: Error writing newline to stderr: {}", e);
            }
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
            let mut stdout = tokio::io::BufWriter::new(tokio::io::stdout());
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                &mut stdout,
                format!($($arg)*).as_bytes()
            ).await {
                eprintln!(":err: Error writing to stdout: {}", e);
            }
            if let Err(e) = tokio::io::AsyncWriteExt::flush(&mut stdout).await {
                eprintln!(":err: Error flushing stdout: {}", e);
            }
        }
    }}
}

/// Press enter to continue functionality
pub async fn press_enter() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let term = Term::stdout();

    loop {
        let key = term.read_key()?;

        match key {
            Key::Enter => break,
            Key::Char('q') => return Err("The operation has been cancelled".into()),
            _ => (),
        }
    }

    Ok(())
}

/// Reads user input and returns a String
pub async fn ainput(prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = String::new();
    let mut reader = BufReader::new(tokio::io::stdin());

    async_print!("{}", prompt).await;
    reader.read_line(&mut buffer).await?;

    Ok(buffer.trim().to_string())
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

impl std::fmt::Display for MFText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Size of view in select menu
const VIEWPORT_SIZE: usize = 7;

/// A replacement for inquire. Works better
pub async fn select<T>(prompt: &str, options: Vec<(String, T)>) -> io::Result<T>
where
    T: Clone,
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
        term.clear_screen()?;
        async_println!(":out: {prompt}").await;

        let (_rows, columns) = term.size();

        let start = if total <= VIEWPORT_SIZE {
            0
        } else if index < VIEWPORT_SIZE / 2 {
            0
        } else if index > total - (VIEWPORT_SIZE / 2 + 1) {
            total - VIEWPORT_SIZE
        } else {
            index - VIEWPORT_SIZE / 2
        };

        let end = (start + VIEWPORT_SIZE).min(total);
        for (i, (label, _)) in options.iter().enumerate().skip(start).take(end - start) {
            let max_width = columns as usize - 3;

            let label = if label.len() > max_width {
                &format!("{}...", label[0..max_width - 3].to_string())
            } else {
                label
            };

            if i == index {
                async_println!(">> {label}").await;
            } else {
                async_println!("   {label}").await;
            }
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
