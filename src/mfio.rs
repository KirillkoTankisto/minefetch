use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufWriter, BufReader};

/// Macro for async std output (with \n)
#[macro_export]
macro_rules! async_println {
    ($($arg:tt)*) => {{
        async {
            let mut stdout = BufWriter::new(io::stdout());
            if let Err(e) = stdout.write_all(format!($($arg)*).as_bytes()).await {
                eprintln!("Error writing to stdout: {}", e);
            }
            if let Err(e) = stdout.write_all(b"\n").await {
                eprintln!("Error writing newline to stdout: {}", e);
            }
            if let Err(e) = stdout.flush().await {
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
            let mut stderr = BufWriter::new(io::stderr());
            if let Err(e) = stderr.write_all(format!($($arg)*).as_bytes()).await {
                eprintln!("Error writing to stderr: {}", e);
            }
            if let Err(e) = stderr.write_all(b"\n").await {
                eprintln!("Error writing newline to stderr: {}", e);
            }
            if let Err(e) = stderr.flush().await {
                eprintln!("Error flushing stderr: {}", e);
            }
        }
    }}
}

/// Macro for async std output (without \n)
#[macro_export]
macro_rules! async_print {
    ($($arg:tt)*) => {{
        async {
            let mut stdout = BufWriter::new(io::stdout());
            if let Err(e) = stdout.write_all(format!($($arg)*).as_bytes()).await {
                eprintln!("Error writing to stdout: {}", e);
            }
            if let Err(e) = stdout.flush().await {
                eprintln!("Error flushing stdout: {}", e);
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

/// Reads user input and returns String
pub async fn ainput(prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = String::new();
    let mut reader = BufReader::new(tokio::io::stdin());

    async_print!("{}", prompt).await;
    reader.read_line(&mut buffer).await?;

    Ok(buffer.trim().to_string())
}