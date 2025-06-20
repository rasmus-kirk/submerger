#![feature(let_chains)]

mod merge;
mod test;

use merge::*;

use anyhow::Result;
use clap::{Parser, Subcommand};
use core::fmt;
use log::info;
use std::io::Write;
use std::path::PathBuf;
use std::{fmt::Debug, fs::File};

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
        write!(f, "{printable}")
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

        /// Sets the color for the first subtitle track (HTML, ex. #fbf1c7)
        #[arg(long)]
        sub1_color: Option<String>,

        /// Sets the position of the first subtitle track
        #[arg(long, default_value = "bottom-center")]
        sub1_position: SubPosition,

        /// Sets the offset of the first subtitle track (seconds)
        #[clap(allow_hyphen_values = true)]
        #[arg(long, default_value_t = 0.0)]
        sub1_offset: f32,

        /// Path to the first subtitle file
        #[arg(required = true)]
        sub2: PathBuf,

        /// Sets the color for the second subtitle track (HTML, ex. #fbf1c7)
        #[arg(long)]
        sub2_color: Option<String>,

        /// Sets the position of the second subtitle track
        #[arg(long, default_value = "top-center")]
        sub2_position: SubPosition,

        /// Sets the offset of the second subtitle track (seconds)
        #[clap(allow_hyphen_values = true)]
        #[arg(long, default_value_t = 0.0)]
        sub2_offset: f32,

        /// Output file where the merged subtitles will be saved
        #[arg(required = true)]
        out: PathBuf,

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

        /// Sets the color for the first subtitle track (HTML, ex. #fbf1c7)
        #[arg(long)]
        sub1_color: Option<String>,

        /// Sets the position of the first subtitle track
        #[arg(long, default_value = "bottom-center")]
        sub1_position: SubPosition,

        /// Sets the offset of the first subtitle track (seconds)
        #[clap(allow_hyphen_values = true)]
        #[arg(long, default_value_t = 0.0)]
        sub1_offset: f32,

        /// Language code for the second subtitle file (e.g., `ja` for Japanese)
        #[arg(required = true)]
        sub2_lang: String,

        /// Sets the color for the first subtitle track (HTML, ex. #fbf1c7)
        #[arg(long)]
        sub2_color: Option<String>,

        /// Sets the position of the first subtitle track
        #[arg(long, default_value = "top-center")]
        sub2_position: SubPosition,

        /// Sets the offset of the second subtitle track (seconds)
        #[clap(allow_hyphen_values = true)]
        #[arg(long, default_value_t = 0.0)]
        sub2_offset: f32,

        /// Root directory to recursively search for subtitle files
        #[arg(required = true)]
        path: PathBuf,

        /// The file extension for the output file (e.g. `file.en.srt` -> `file.merged.srt` if set to `merged.srt`)
        #[arg(short, long, default_value = "srt")]
        out_ext: String,

        /// Also match and convert VTT files. Note, this will not output VTT files, only SRT is supported as output.
        #[arg(short, long, default_value = "true")]
        vtt: bool,

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
            sub1_color,
            sub1_position,
            sub1_offset,
            sub2,
            sub2_color,
            sub2_position,
            sub2_offset,
            out,
            log_level,
        } => {
            simple_logger::init_with_level(log_level.into())?;

            let mut srt1 = load_sub(&sub1)?;
            let mut srt2 = load_sub(&sub2)?;

            apply_sub_changes(&mut srt1, sub1_color, sub1_position, sub1_offset);
            apply_sub_changes(&mut srt2, sub2_color, sub2_position, sub2_offset);

            let merged = merge(srt1, srt2);

            let mut file = File::create(&out)?;
            file.write_all(format!("{merged}").as_bytes())?;

            info!("Successfully merged subtitles into {:?}", out);
        }
        Commands::Recursive {
            path,
            sub1_lang,
            sub1_color,
            sub1_position,
            sub1_offset,
            sub2_lang,
            sub2_color,
            sub2_position,
            sub2_offset,
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
                        let mut srt1 = load_sub(&s1.path.clone())?;
                        let mut srt2 = load_sub(&s2.path.clone())?;

                        apply_sub_changes(
                            &mut srt1,
                            sub1_color.clone(),
                            sub1_position,
                            sub1_offset,
                        );
                        apply_sub_changes(
                            &mut srt2,
                            sub2_color.clone(),
                            sub2_position,
                            sub2_offset,
                        );

                        // Create extension for new file, e.g. "enja"
                        let no_ext = base_file_stem(&s1.path)?;
                        let out = dir.join(no_ext.with_extension(&out_ext));

                        info!("Writing subs to {:?}", out);

                        let merged = merge(srt1, srt2);
                        let mut file = File::create(&out)?;
                        file.write_all(format!("{merged}").as_bytes())?;
                    }
                }
            }
        }
    }

    Ok(())
}
