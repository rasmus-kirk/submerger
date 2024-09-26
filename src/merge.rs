use anyhow::{bail, Context, Result};
use log::{info, trace};
use regex::Regex;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use subtp::{
    srt::{SrtSubtitle, SrtTimestamp, SubRip},
    vtt::{VttBlock, WebVtt},
};
use walkdir::WalkDir;

use crate::SubPosition;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubFile {
    pub path: PathBuf,
    pub lang: String,
    pub hi: bool,
}

/// Matches a subtitle file of either `.srt` or `.vtt` for the specified languages
/// for example `movie.en.srt` or `movie.ja.srt` if the languages are `en` and `ja`.
///
/// Yes, this is awful. I hate regex. Without variables it's:
///
/// > `r"[^\.]+\.(?P<lang>en|ja)(\.(?P<hearing>hi))?\.(?P<ext>srt|vtt)$"`
///
/// Which is still not good, but see the corresponding test to see how it behaves in more detail.
pub fn get_sub_path_regex(lang1: &String, lang2: &String, find_vtt: bool) -> String {
    let langs = lang1.to_owned() + "|" + lang2;
    let ext = if find_vtt { "srt|vtt" } else { "srt" };
    r"[^\.]+\.(?P<lang>".to_owned() + &langs + r")(\.(?P<hearing>hi))?\.(?P<ext>" + ext + ")$"
}

/// Return the filename, as in, all characters up to a `.`
/// `let p: Pathbuf; p.file_stem` returns `filename.en`, this returns `filename`
pub fn base_file_stem(p: &Path) -> Result<PathBuf> {
    let pattern = Regex::new(r"[^\.]+")?;
    let path_string = p
        .file_name()
        .and_then(|x| x.to_str())
        .context(format!("unable to parse filepath {:?}", p))?;
    let x = pattern
        .find(path_string)
        .context(format!("unable to compute filestem for {:?}", path_string))?
        .as_str();
    Ok(Path::new(x).to_path_buf())
}

/// Recursively search a directory for the specified subtitle files.
pub fn find_matching_subtitle_files(
    root_dir: &PathBuf,
    lang1: &String,
    lang2: &String,
    find_vtt: bool,
) -> Result<HashMap<PathBuf, Vec<SubFile>>> {
    let regex = get_sub_path_regex(lang1, lang2, find_vtt);
    let subtitle_pattern = Regex::new(regex.as_str())?;
    let mut ret = HashMap::new();

    if root_dir.is_file() {
        bail!("the given path must be a directory!")
    }

    for entry in WalkDir::new(root_dir).follow_links(true) {
        let entry = entry?;
        trace!("Found entry: {:?}", entry.path());

        let dir_path = entry.path();
        if !entry.file_type().is_dir() {
            trace!("Entry {:?} was not a dir", entry.path());
            continue;
        }

        // Now find files with matching subtitle names in this directory
        for entry in dir_path.read_dir()? {
            let file_path = entry?.path();
            if file_path.is_file()
                && let Some(file_name) = file_path.file_name().and_then(|n| n.to_str())
                && let Some(captures) = subtitle_pattern.captures(file_name)
            {
                trace!("Found file: {}", file_name);

                let lang = captures
                    .name("lang")
                    .context(format!(
                        "impossible error: unable to find lang in {}",
                        file_name
                    ))?
                    .as_str()
                    .to_owned();
                let hi = captures.name("hearing").is_some();
                let val = SubFile {
                    path: file_path,
                    lang,
                    hi,
                };

                if !ret.contains_key(dir_path) {
                    let _ = ret.insert(dir_path.to_owned(), Vec::new());
                };
                ret.get_mut(dir_path).unwrap().push(val);
            }
        }
    }

    Ok(ret)
}

pub fn load_sub(path: PathBuf) -> Result<SubRip> {
    let file = fs::read_to_string(&path)?;
    let ext = path
        .extension()
        .context(format!("unable to retrieve extension from file {}", file))?
        .to_str()
        .context(format!(
            "unable to parse extension as a string from file {}",
            file
        ))?;
    let subfile = match ext {
        "vtt" => vtt_to_subrip(WebVtt::parse(&file)?),
        "srt" => SubRip::parse(&file)?,
        _ => bail!(
            "invalid extension ({}), supported extensions are: srt, vtt",
            ext
        ),
    };

    info!(
        "Loaded {} subtitles from {:?}",
        subfile.subtitles.len(),
        path
    );

    Ok(subfile)
}

pub fn merge(
    srt1: &SubRip,
    srt2: &SubRip,
    srt2_color_opt: Option<String>,
    srt2_position: SubPosition,
) -> SubRip {
    let position = srt2_position.to_string();
    let (color_start, color_end) = if let Some(color) = srt2_color_opt {
        (format!("<font color=\"{color}\">"), "</font>".to_owned())
    } else {
        ("".to_owned(), "".to_owned())
    };

    let mut subs = srt2.subtitles.clone();
    for sub in &mut subs {
        for txt in &mut sub.text {
            *txt = format!("{position} {color_start}{txt}{color_end}");
        }
    }

    let mut merged_subs = srt1.clone();
    merged_subs.subtitles.extend(subs);

    for i in 0..merged_subs.subtitles.len() {
        merged_subs.subtitles[i].sequence = i as u32 + 1;
    }
    merged_subs
}

fn vtt_block_to_srt(vtt_block: VttBlock, sequence: u32) -> Option<SrtSubtitle> {
    let cue = match vtt_block {
        VttBlock::Que(y) => y,
        _ => return None,
    };

    let start = SrtTimestamp {
        hours: cue.timings.start.hours,
        minutes: cue.timings.start.minutes,
        seconds: cue.timings.start.seconds,
        milliseconds: cue.timings.start.milliseconds,
    };
    let end = SrtTimestamp {
        hours: cue.timings.end.hours,
        minutes: cue.timings.end.minutes,
        seconds: cue.timings.end.seconds,
        milliseconds: cue.timings.end.milliseconds,
    };

    Some(SrtSubtitle {
        sequence,
        start,
        end,
        text: cue.payload,
        line_position: None,
    })
}

fn vtt_to_subrip(vtt: WebVtt) -> SubRip {
    let mut i = 1;
    let mut subtitles = Vec::new();

    for vtt_block in vtt {
        if let Some(sub) = vtt_block_to_srt(vtt_block, i) {
            subtitles.push(sub)
        }
        i += 1;
    }

    SubRip { subtitles }
}
