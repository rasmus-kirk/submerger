#![feature(let_chains)]

mod merge;
mod test;

use merge::*;

use anyhow::{anyhow, Result};
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
    /// Simple cli tool for merging two srt files
    SimpleCli {
        /// Path to the first subtitle file
        #[arg(required = true)]
        sub1: PathBuf,

        /// Path to the first subtitle file
        #[arg(required = true)]
        sub2: PathBuf,

        /// Output file where the merged subtitles will be saved
        #[arg(short, long)]
        out: PathBuf,

        /// Sets the color for the second subtitle track
        #[arg(short, long)]
        color: Option<String>,

        /// Sets the position of the second subtitle track
        #[arg(short, long, default_value = "top-center")]
        position: SubPosition,
    },
    /// Recursively merge srt files matching the given specification
    ///
    /// By default, this will search the directory for any files matching
    /// the languages given ("en", "ja", "da", etc), but also match hearing
    /// impaired subs ("en.hi", "ja.hi", "da.hi", etc) if no normal subs are found
    RecursiveMerge {
        /// Root directory to recursively search for subtitle files
        #[arg(required = true)]
        path: PathBuf,

        /// Language code for the first subtitle file (e.g., `en` for English)
        #[arg(long, required = true)]
        sub1_lang: String,

        /// Language code for the second subtitle file (e.g., `ja` for Japanese)
        #[arg(long, required = true)]
        sub2_lang: String,

        /// Sets the color for the second subtitle track
        #[arg(short, long)]
        color: Option<String>,

        /// Sets the position of the second subtitle track
        #[arg(short, long, default_value = "top-center")]
        position: SubPosition,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.subcommand {
        Commands::SimpleCli {
            sub1,
            sub2,
            out,
            color,
            position,
        } => {
            let merged = merge(&load_sub(sub1)?, &load_sub(sub2)?, color, position);

            let mut file = File::create(&out)?;
            file.write_all(merged.render().as_bytes())?;

            info!("Successfully merged subtitles into {:?}", out);
        }
        Commands::RecursiveMerge {
            path,
            sub1_lang,
            sub2_lang,
            color,
            position,
        } => {
            let matches = find_matching_subtitle_files(&path, &sub1_lang, &sub2_lang, true)?;

            for (_, subs) in matches {
                // Try to only use non-hearing impaired subs
                let mut l1 = subs.iter().find(|x| x.lang == sub1_lang && !x.hi);
                let mut l2 = subs.iter().find(|x| x.lang == sub2_lang && !x.hi);

                // If none is found, be less picky
                if l1.is_none() {
                    l1 = subs.iter().find(|x| x.lang == sub1_lang)
                }
                if l2.is_none() {
                    l2 = subs.iter().find(|x| x.lang == sub2_lang)
                }

                // If we have found lang each for a file, continue
                if let Some(s1) = l1
                    && let Some(s2) = l2
                    && base_file_stem(&s1.path)? == base_file_stem(&s2.path)?
                {
                    let sub1 = load_sub(s1.path.clone())?;
                    let sub2 = load_sub(s2.path.clone())?;

                    // Create extension for new file, e.g. "enja"
                    let no_ext = base_file_stem(&s1.path)?;
                    let old_ext = s1
                        .path
                        .extension()
                        .ok_or_else(|| anyhow!("invalid extension on {:?}", s1.path))?
                        .to_str()
                        .ok_or_else(|| anyhow!("not valid unicode ({:?})", s1.path))?;
                    let new_ext = sub1_lang.clone() + sub2_lang.as_str() + "." + old_ext;
                    let out = no_ext.with_extension(new_ext);

                    // Create extension for new file, e.g. "enja"
                    let merged = merge(&sub1, &sub2, color.clone(), position);
                    let mut file = File::create(&out)?;
                    file.write_all(merged.render().as_bytes())?;
                }
            }
        }
    }

    Ok(())
}
