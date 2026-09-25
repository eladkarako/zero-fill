use std::{
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

use clap::Parser;

#[derive(Clone, Debug)]
pub struct Config {
    pub path: PathBuf,
    pub reserve: u64,
    pub chunk_size: u64,
    pub max_size: Option<u64>,
    pub prefix: String,
    pub progress_bar: bool,
    pub dry_run: bool,
    pub keep_file: bool,
}

#[derive(Parser, Debug)]
#[command(
    name = "zero-fill",
    about = "Safely allocate filesystem space using Linux fallocate"
)]
struct Arguments {
    /// Directory on the target filesystem
    #[arg(long, default_value = ".")]
    path: PathBuf,

    /// Space to preserve, for example 2G, 512M, or 1T
    #[arg(long, default_value = "2G")]
    reserve: String,

    /// Allocation chunk size
    #[arg(long, default_value = "256M")]
    chunk_size: String,

    /// Maximum allocation size
    #[arg(long)]
    max_size: Option<String>,

    /// Temporary-file prefix
    #[arg(long, default_value = ".zero-fill-")]
    prefix: String,

    /// Disable the terminal progress bar
    #[arg(long)]
    no_progress: bool,

    /// Display information without allocating
    #[arg(long)]
    dry_run: bool,

    /// Keep the temporary file after successful allocation
    #[arg(long)]
    keep_file: bool,
}

impl Config {
    pub fn parse() -> io::Result<Self> {
        let args = Arguments::parse();

        validate_directory(&args.path)?;

        let reserve = parse_size(&args.reserve)?;
        let chunk_size = parse_size(&args.chunk_size)?;

        if reserve == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--reserve must be greater than zero",
            ));
        }

        if chunk_size == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--chunk-size must be greater than zero",
            ));
        }

        let max_size = args.max_size.as_deref().map(parse_size).transpose()?;

        if max_size == Some(0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--max-size must be greater than zero",
            ));
        }

        Ok(Self {
            path: args.path,
            reserve,
            chunk_size,
            max_size,
            prefix: args.prefix,
            progress_bar: !args.no_progress,
            dry_run: args.dry_run,
            keep_file: args.keep_file,
        })
    }
}

fn validate_directory(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("path does not exist: {}", path.display()),
        ));
    }

    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path is not a directory: {}", path.display()),
        ));
    }

    Ok(())
}

pub fn parse_size(value: &str) -> io::Result<u64> {
    let value = value.trim();

    if value.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "size cannot be empty",
        ));
    }

    let split_at = value
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(value.len());

    let number = &value[..split_at];
    let suffix = value[split_at..].trim().to_ascii_lowercase();

    let number = u64::from_str(number).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid size: {value}"),
        )
    })?;

    let multiplier = match suffix.as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1024,
        "m" | "mb" => 1024_u64.pow(2),
        "g" | "gb" => 1024_u64.pow(3),
        "t" | "tb" => 1024_u64.pow(4),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown size suffix: {suffix}"),
            ));
        }
    };

    number.checked_mul(multiplier).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("size is too large: {value}"),
        )
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
