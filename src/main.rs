#![feature(let_chains)]

mod merge;
mod test;

use chrono::Duration;
use merge::*;

use anyhow::Result;
use clap::{Parser, Subcommand};
use core::fmt;
use log::info;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use subtp::srt::{SrtTimestamp, SubRip};

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

fn f32_to_chrono(secs: f32) -> Duration {
    // Convert to milliseconds, rounding toward zero
    let millis = secs * 1000.0;
    Duration::milliseconds(millis as i64)
}

// Convert SrtTimestamp to chrono::Duration
pub fn srt_to_chrono(srt: SrtTimestamp) -> Duration {
    Duration::milliseconds(
        (srt.hours as i64 * 60 * 60 * 1000)
            + (srt.minutes as i64 * 60 * 1000)
            + (srt.seconds as i64 * 1000)
            + srt.milliseconds as i64,
    )
}

// Convert chrono::Duration to SrtTimestamp
pub fn chrono_to_srt(duration: Duration) -> SrtTimestamp {
    let total_millis = duration.num_milliseconds().abs();

    let hours = (total_millis / (60 * 60 * 1000)) as u8;
    let minutes = ((total_millis / (60 * 1000)) % 60) as u8;
    let seconds = ((total_millis / 1000) % 60) as u8;
    let milliseconds = (total_millis % 1000) as u16;

    SrtTimestamp {
        hours,
        minutes,
        seconds,
        milliseconds,
    }
}

fn apply_sub_changes(
    srt: &mut SubRip,
    color_opt: Option<String>,
    position: SubPosition,
    offset: f32,
) {
    let position = position.to_string();
    let (color_start, color_end) = if let Some(color) = color_opt {
        (format!("<font color=\"{color}\">"), "</font>".to_owned())
    } else {
        ("".to_owned(), "".to_owned())
    };

    for sub in &mut srt.subtitles {
        // Strip position from original sub
        sub.line_position = None;
        for text in sub.text.iter_mut() {
            if let Some(s) = text.strip_prefix(r"{\an8}") {
                *text = s.to_string();
            }
        }

        // Apply changes
        for txt in &mut sub.text {
            *txt = format!("{position} {color_start}{txt}{color_end}");
        }
        let offset = f32_to_chrono(offset);
        sub.start = chrono_to_srt(srt_to_chrono(sub.start) + offset);
        sub.end = chrono_to_srt(srt_to_chrono(sub.end) + offset);
    }
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

            let mut srt1 = load_sub(sub1)?;
            let mut srt2 = load_sub(sub2)?;

            apply_sub_changes(&mut srt1, sub1_color, sub1_position, sub1_offset);
            apply_sub_changes(&mut srt2, sub2_color, sub2_position, sub2_offset);

            let merged = merge(srt1, srt2);

            let mut file = File::create(&out)?;
            file.write_all(merged.render().as_bytes())?;

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
                        let mut srt1 = load_sub(s1.path.clone())?;
                        let mut srt2 = load_sub(s2.path.clone())?;

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
                        file.write_all(merged.render().as_bytes())?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_srt_conversion_roundtrip() {
        let original = SrtTimestamp {
            hours: 1,
            minutes: 23,
            seconds: 45,
            milliseconds: 678,
        };

        let duration = srt_to_chrono(original);
        let roundtrip = chrono_to_srt(duration);

        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_duration_to_srt_zero() {
        let duration = Duration::zero();
        let expected = SrtTimestamp {
            hours: 0,
            minutes: 0,
            seconds: 0,
            milliseconds: 0,
        };
        assert_eq!(chrono_to_srt(duration), expected);
    }

    #[test]
    fn test_negative_duration_to_srt() {
        let duration = Duration::milliseconds(-3723678); // -1h 2m 3s 678ms
        let srt = chrono_to_srt(duration);
        assert_eq!(
            srt,
            SrtTimestamp {
                hours: 1,
                minutes: 2,
                seconds: 3,
                milliseconds: 678,
            }
        );
    }
}
