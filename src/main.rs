mod build;
mod cli;
mod config;
mod detect;
mod error;
mod jlink;
mod jvm;
mod pack;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::HumanBytes;

use cli::{Cli, Command};
use config::{BuildConfig, Target};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("clj_pack=info".parse().unwrap()),
        )
        .with_target(false)
        .without_time()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Build {
            input,
            output,
            java_version,
            target,
            jvm_args,
        } => {
            let target = match target {
                Some(t) => Target::from_str(&t)
                    .context(format!("invalid target: {t}. Use: linux-x64, linux-aarch64, macos-x64, macos-aarch64"))?,
                None => Target::current(),
            };

            let config = BuildConfig {
                input: std::fs::canonicalize(&input)
                    .unwrap_or_else(|_| PathBuf::from(&input)),
                output: PathBuf::from(&output),
                java_version,
                target,
                jvm_args,
            };

            run_build(config).await?;
        }
        Command::Clean => {
            run_clean()?;
        }
        Command::Info => {
            run_info()?;
        }
    }

    Ok(())
}

async fn run_build(config: BuildConfig) -> Result<()> {
    let jar_path = if config.input.extension().map_or(false, |e| e == "jar") {
        tracing::info!("using pre-built JAR: {}", config.input.display());
        config.input.clone()
    } else {
        tracing::info!("detecting build system in {}", config.input.display());
        let system = detect::detect_build_system(&config.input)?;
        tracing::info!("building uberjar with {:?}", system);
        build::build_uberjar(&config.input, system)?
    };

    tracing::info!("uberjar: {}", jar_path.display());

    let jdk_path = jvm::ensure_jdk(config.java_version, &config.target).await?;
    tracing::info!("JDK path: {}", jdk_path.display());

    let temp_dir = tempfile::tempdir()?;
    let modules = jlink::detect_modules(&jdk_path, &jar_path)?;
    tracing::info!("modules: {modules}");

    let runtime_path = jlink::create_runtime(&jdk_path, &modules, temp_dir.path())?;
    tracing::info!("runtime created at {}", runtime_path.display());

    pack::create_binary(&runtime_path, &jar_path, &config.output, &config.jvm_args)?;

    let size = std::fs::metadata(&config.output)?.len();
    eprintln!("\n  Binary: {}", config.output.display());
    eprintln!("  Size:   {}", HumanBytes(size));
    eprintln!("  Ready to run!\n");

    Ok(())
}

fn run_clean() -> Result<()> {
    let cache_dir = BuildConfig::cache_dir();
    if cache_dir.exists() {
        let size = dir_size(&cache_dir);
        std::fs::remove_dir_all(&cache_dir)?;
        eprintln!("Cleaned {} of cached data", HumanBytes(size));
    } else {
        eprintln!("Cache is already empty");
    }
    Ok(())
}

fn run_info() -> Result<()> {
    let cache_dir = BuildConfig::cache_dir();
    eprintln!("Cache directory: {}", cache_dir.display());

    if cache_dir.exists() {
        let size = dir_size(&cache_dir);
        eprintln!("Cache size:      {}", HumanBytes(size));

        let entries: Vec<_> = std::fs::read_dir(&cache_dir)?
            .filter_map(|e| e.ok())
            .collect();
        eprintln!("Cached items:    {}", entries.len());

        for entry in &entries {
            let name = entry.file_name();
            let entry_size = dir_size(&entry.path());
            eprintln!("  {} ({})", name.to_string_lossy(), HumanBytes(entry_size));
        }
    } else {
        eprintln!("Cache is empty");
    }

    eprintln!("\nCurrent platform: {:?}", Target::current());
    Ok(())
}

fn dir_size(path: &std::path::Path) -> u64 {
    walkdir(path)
}

fn walkdir(path: &std::path::Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                size += walkdir(&p);
            } else if let Ok(meta) = p.metadata() {
                size += meta.len();
            }
        }
    }
    size
}
