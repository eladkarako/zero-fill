#![cfg_attr(
    not(any(target_os = "linux", target_os = "android")),
    allow(dead_code)
)]

#[cfg(not(any(target_os = "linux", target_os = "android")))]
compile_error!("This program only supports Linux and Android");

mod allocator;
mod cli;
mod filesystem;

use std::io;

use allocator::ProgressEvent;
use cli::Config;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = Config::parse()?;

    if config.dry_run {
        let available = filesystem::available_space(&config.path)?;

        println!("Path: {}", config.path.display());
        println!("Available: {}", indicatif::HumanBytes(available));
        println!(
            "Reserve: {}",
            indicatif::HumanBytes(config.reserve)
        );

        if available > config.reserve {
            println!(
                "Allocatable: {}",
                indicatif::HumanBytes(available - config.reserve)
            );
        } else {
            println!("Allocatable: 0 B");
        }

        return Ok(());
    }

    let initial_free = filesystem::available_space(&config.path)?;

    if initial_free <= config.reserve {
        return Err(io::Error::other(format!(
            "available space ({}) is not greater than reserve ({})",
            indicatif::HumanBytes(initial_free),
            indicatif::HumanBytes(config.reserve)
        )));
    }

    let initial_target = config
        .max_size
        .unwrap_or(initial_free - config.reserve)
        .min(initial_free - config.reserve);

    let progress_bar = if config.progress_bar {
        let bar = ProgressBar::new(initial_target);

        let style = ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] \
             [{bar:40.cyan/blue}] {bytes}/{total_bytes} \
             ({percent}%) {msg}",
        )
            .map_err(io::Error::other)?
            .progress_chars("##-");

        bar.set_style(style);
        Some(bar)
    } else {
        None
    };

    let (sender, mut receiver) = mpsc::channel::<ProgressEvent>(16);
    let worker_config = config.clone();

    let worker = tokio::task::spawn_blocking(move || {
        allocator::run(worker_config, sender)
    });

    while let Some(event) = receiver.recv().await {
        if let Some(bar) = &progress_bar {
            bar.set_position(event.allocated);
            bar.set_message(format!(
                "{} free",
                indicatif::HumanBytes(event.free_space)
            ));
        } else {
            println!(
                "allocated={} free={}",
                event.allocated, event.free_space
            );
        }
    }

    let allocated = worker.await.map_err(|error| {
        io::Error::other(format!("worker failed: {error}"))
    })??;

    if let Some(bar) = progress_bar {
        bar.finish_with_message("temporary file removed");
    }

    println!(
        "Allocated and removed {}.",
        indicatif::HumanBytes(allocated)
    );

    Ok(())
}
