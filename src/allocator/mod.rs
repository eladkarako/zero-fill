use std::{fs::File, io};

use tempfile::Builder;
use tokio::sync::mpsc;

use crate::{cli::Config, filesystem};

#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub allocated: u64,
    pub free_space: u64,
}

pub fn run(config: Config, progress_sender: mpsc::Sender<ProgressEvent>) -> io::Result<u64> {
    let initial_free = filesystem::available_space(&config.path)?;

    if initial_free <= config.reserve {
        return Err(io::Error::other(
            "free space is at or below the configured reserve",
        ));
    }

    let temporary_file = Builder::new()
        .prefix(&config.prefix)
        .tempfile_in(&config.path)?;

    let file: &File = temporary_file.as_file();
    let mut allocated = 0u64;

    let allocation_result = allocate_loop(file, &config, &progress_sender, &mut allocated);

    match allocation_result {
        Ok(()) => {
            file.sync_all()?;

            if config.keep_file {
                let path = temporary_file.into_temp_path().keep()?;

                println!("Keeping temporary file: {}", path.display());
            } else {
                temporary_file.close()?;
            }

            Ok(allocated)
        }

        Err(error) => {
            // Dropping NamedTempFile attempts cleanup.
            drop(temporary_file);
            Err(error)
        }
    }
}

fn allocate_loop(
    file: &File,
    config: &Config,
    progress_sender: &mpsc::Sender<ProgressEvent>,
    allocated: &mut u64,
) -> io::Result<()> {
    loop {
        let free_space = filesystem::available_space(&config.path)?;

        if free_space <= config.reserve {
            break;
        }

        let reserve_limited = free_space - config.reserve;

        let max_remaining = config
            .max_size
            .map(|maximum| maximum.saturating_sub(*allocated))
            .unwrap_or(u64::MAX);

        if max_remaining == 0 {
            break;
        }

        let amount = config.chunk_size.min(reserve_limited).min(max_remaining);

        if amount == 0 {
            break;
        }

        match filesystem::fallocate(file, *allocated, amount) {
            Ok(()) => {
                *allocated = allocated
                    .checked_add(amount)
                    .ok_or_else(|| io::Error::other("allocation size overflow"))?;

                progress_sender
                    .blocking_send(ProgressEvent {
                        allocated: *allocated,
                        free_space: free_space.saturating_sub(amount),
                    })
                    .map_err(|_| {
                        io::Error::new(io::ErrorKind::BrokenPipe, "progress receiver was closed")
                    })?;
            }

            Err(error) if filesystem::is_interrupted(&error) => {
                continue;
            }

            Err(error) if filesystem::is_no_space(&error) => {
                break;
            }

            Err(error) => return Err(error),
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
