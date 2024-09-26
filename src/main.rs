#![feature(let_chains)]

mod merge;
mod test;

use merge::*;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use core::fmt;
use log::info;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(clap::ValueEnum, Clone, Copy, Default, Debug)]
enum SubPosition {
    BottomLeft,
    #[default]
    BottomCenter,
    BottomRight,
    MiddleLeft,
    MiddleCenter,
    MiddleRight,
    TopLeft,
    TopCenter,
    TopRight,
}

#[derive(clap::ValueEnum, Clone, Copy, Default, Debug)]
enum LogLevel {
    Error = 1,
    #[default]
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl From<log::Level> for LogLevel {
    fn from(item: log::Level) -> Self {
        match item {
            log::Level::Error => LogLevel::Error,
            log::Level::Warn => LogLevel::Warn,
            log::Level::Info => LogLevel::Info,
            log::Level::Debug => LogLevel::Debug,
            log::Level::Trace => LogLevel::Trace,
        }
    }
}

impl From<LogLevel> for log::Level {
    fn from(val: LogLevel) -> Self {
        match val {
            LogLevel::Error => log::Level::Error,
            LogLevel::Warn => log::Level::Warn,
            LogLevel::Info => log::Level::Info,
            LogLevel::Debug => log::Level::Debug,
            LogLevel::Trace => log::Level::Trace,
        }
    }
}

impl fmt::Display for SubPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let printable = match self {
            SubPosition::BottomLeft => "{\\an1}",
            SubPosition::BottomCenter => "{\\an2}",
            SubPosition::BottomRight => "{\\an3}",
            SubPosition::MiddleLeft => "{\\an4}",
            SubPosition::MiddleCenter => "{\\an5}",
            SubPosition::MiddleRight => "{\\an6}",
            SubPosition::TopLeft => "{\\an7}",
            SubPosition::TopCenter => "{\\an8}",
            SubPosition::TopRight => "{\\an9}",
        };
        write!(f, "{}", printable)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Subcommands for the tool
    #[command(subcommand)]
    subcommand: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Simple CLI interface for merging two srt files
    Simple {
        /// Path to the first subtitle file
        #[arg(required = true)]
        sub1: PathBuf,

        /// Path to the first subtitle file
        #[arg(required = true)]
        sub2: PathBuf,

        /// Output file where the merged subtitles will be saved
        #[arg(required = true)]
        out: PathBuf,

        /// Sets the color for the second subtitle track
        #[arg(short, long)]
        color: Option<String>,

        /// Sets the position of the second subtitle track
        #[arg(short, long, default_value = "top-center")]
        position: SubPosition,

        /// Sets the level of logging
        #[arg(short, long, default_value = "warn")]
        log_level: LogLevel,
    },
    /// Recursively merge srt files matching the given specification
    ///
    /// By default, this will search the directory for any files matching
    /// the languages given ("en", "ja", "da", etc), but also match hearing
    /// impaired subs ("en.hi", "ja.hi", "da.hi", etc) if no normal subs are found
    Recursive {
        /// Language code for the first subtitle file (e.g., `en` for English)
        #[arg(required = true)]
        sub1_lang: String,

        /// Language code for the second subtitle file (e.g., `ja` for Japanese)
        #[arg(required = true)]
        sub2_lang: String,

        /// Root directory to recursively search for subtitle files
        #[arg(required = true)]
        path: PathBuf,

        /// The file extension for the output file (e.g. `file.en.srt` -> `file.merged.srt` if set to `merged.srt`)
        #[arg(short, long, default_value = "srt")]
        out_ext: String,

        /// Also match and convert VTT files. Note, this will not output VTT files, only SRT is supported as output.
        #[arg(short, long, default_value = "true")]
        vtt: bool,

        /// Sets the color for the second subtitle track
        #[arg(short, long)]
        color: Option<String>,

        /// Sets the position of the second subtitle track
        #[arg(short, long, default_value = "top-center")]
        position: SubPosition,

        /// Sets the level of logging
        #[arg(short, long, default_value = "warn")]
        log_level: LogLevel,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.subcommand {
        Commands::Simple {
            sub1,
            sub2,
            out,
            color,
            position,
            log_level,
        } => {
            simple_logger::init_with_level(log_level.into())?;

            let merged = merge(&load_sub(sub1)?, &load_sub(sub2)?, color, position);

            let mut file = File::create(&out)?;
            file.write_all(merged.render().as_bytes())?;

            info!("Successfully merged subtitles into {:?}", out);
        }
        Commands::Recursive {
            path,
            sub1_lang,
            sub2_lang,
            color,
            position,
            log_level,
            out_ext,
            vtt,
        } => {
            simple_logger::init_with_level(log_level.into())?;

            let matches = find_matching_subtitle_files(&path, &sub1_lang, &sub2_lang, vtt)?;

            for (dir, subs) in matches {
                for sub1 in &subs {
                    let mut l1 = None;
                    let mut l2 = None;

                    for sub2 in &subs {
                        if base_file_stem(&sub1.path)? == base_file_stem(&sub2.path)?
                            && sub1.lang == sub1_lang
                            && sub2.lang == sub2_lang
                        {
                            if !sub1.hi || l1.is_none() {
                                l1 = Some(sub1.clone())
                            }
                            if !sub2.hi || l2.is_none() {
                                l2 = Some(sub2.clone())
                            }
                        }
                    }

                    // If we have found lang each for a file, continue
                    if let Some(s1) = l1
                        && let Some(s2) = l2
                    {
                        let sub1 = load_sub(s1.path.clone())?;
                        let sub2 = load_sub(s2.path.clone())?;

                        // Create extension for new file, e.g. "enja"
                        let no_ext = base_file_stem(&s1.path)?;
                        let old_ext = s1
                            .path
                            .extension()
                            .context(format!("invalid extension on {:?}", s1.path))?
                            .to_str()
                            .context(format!("not valid unicode ({:?})", s1.path))?;
                        let new_ext = sub1_lang.clone() + sub2_lang.as_str() + "." + old_ext;
                        let out = dir.join(no_ext.with_extension(&out_ext));

                        info!("Writing subs to {:?}", out);

                        // Create extension for new file, e.g. "enja"
                        let merged = merge(&sub1, &sub2, color.clone(), position);
                        let mut file = File::create(&out)?;
                        file.write_all(merged.render().as_bytes())?;
                    }
                }
            }
        }
    }

    Ok(())
}
