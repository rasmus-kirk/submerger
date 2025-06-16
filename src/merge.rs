use anyhow::{bail, Context, Result};
use log::trace;
use regex::Regex;
use rsubs_lib::{SRT, SSA, VTT};
use std::{
    collections::HashMap,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};
use time::Duration;
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
pub fn get_sub_path_regex(lang1: &str, lang2: &str, find_vtt: bool) -> String {
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
        .context(format!("unable to parse filepath {p:?}"))?;
    let x = pattern
        .find(path_string)
        .context(format!("unable to compute filestem for {path_string:?}"))?
        .as_str();
    Ok(Path::new(x).to_path_buf())
}

/// Recursively search a directory for the specified subtitle files.
pub fn find_matching_subtitle_files(
    root_dir: &PathBuf,
    lang1: &str,
    lang2: &str,
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
                        "impossible error: unable to find lang in {file_name}"
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

pub fn load_sub(path: &Path) -> Result<SRT> {
    let content = fs::read_to_string(path)?;

    let srt = match path
        .extension()
        .map(std::ffi::OsStr::to_ascii_lowercase)
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
    {
        "srt" => SRT::parse(content)?,
        "vtt" => VTT::parse(content)?.to_srt(),
        "ass" | "ssa" => SSA::parse(content)?.to_srt(),
        _ => bail!("Unknown format"),
    };

    Ok(srt)
}

pub fn apply_sub_changes(
    srt: &mut SRT,
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

    for line in &mut srt.lines {
        if let Some(s) = line.text.strip_prefix(r"{\an8}") {
            line.text = s.to_string();
        }
        line.text = format!("{position} {color_start}{}{color_end}", line.text);
        line.start += Duration::seconds_f32(offset);
        line.end += Duration::seconds_f32(offset);
    }
}

pub fn merge(mut srt1: SRT, srt2: SRT) -> SRT {
    let srt2_len = srt2.lines.len();
    srt1.lines.extend(srt2.lines);
    for i in 0..srt2_len {
        srt1.lines[i].sequence_number = i as u32 + 1;
    }
    srt1
}
