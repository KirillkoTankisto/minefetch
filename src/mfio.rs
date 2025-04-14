/*
 __  __ _            _____    _       _       ___ ___
|  \/  (_)_ __   ___|  ___|__| |_ ___| |__   |_ _/ _ \
| |\/| | | '_ \ / _ \ |_ / _ \ __/ __| '_ \   | | | | |
| |  | | | | | |  __/  _|  __/ || (__| | | |  | | |_| |
|_|  |_|_|_| |_|\___|_|  \___|\__\___|_| |_| |___\___/

*/

// External crates
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, BufReader};

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
pub async fn press_enter() -> Result<(), tokio::io::Error> {
    let mut stdin = io::stdin();

    let mut buffer = [0u8; 1];

    stdin.read_exact(&mut buffer).await?;

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
