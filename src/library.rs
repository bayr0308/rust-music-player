use lofty::{config::ParseOptions, file::AudioFile, prelude::*, probe::Probe};
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub duration: f64,
}

impl Track {
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        self.title.to_lowercase().contains(&query)
            || self.artist.to_lowercase().contains(&query)
            || self
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .contains(&query)
    }
}

#[derive(Clone, Debug)]
pub struct Playlist {
    pub path: PathBuf,
    pub name: String,
    pub tracks: Vec<Track>,
    pub loading: bool,
    pub error: Option<String>,
    pub generation: u64,
}

impl Playlist {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .filter(|n| !n.is_empty())
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned();
        Self {
            path,
            name,
            tracks: vec![],
            loading: false,
            error: None,
            generation: 0,
        }
    }
}

pub fn supported(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        matches!(
            ext.to_string_lossy().to_ascii_lowercase().as_str(),
            "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" | "aif" | "aiff"
        )
    })
}

// A directory is one playlist. Subdirectories are deliberately separate playlists.
pub fn scan_folder(path: &Path) -> Result<Vec<Track>, String> {
    let entries = fs::read_dir(path).map_err(|e| format!("Не удалось открыть папку: {e}"))?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Не удалось прочитать содержимое папки: {e}"))?;
        let path = entry.path();
        if path.is_file() && supported(&path) {
            paths.push(path);
        }
    }
    paths.sort_by(|a, b| {
        natural_cmp(
            &a.file_name().unwrap_or_default().to_string_lossy(),
            &b.file_name().unwrap_or_default().to_string_lossy(),
        )
    });
    Ok(paths.into_iter().map(read_track).collect())
}

pub fn read_track(path: PathBuf) -> Track {
    let fallback = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut track = Track {
        path: path.clone(),
        title: fallback,
        artist: "Неизвестный исполнитель".into(),
        duration: 0.0,
    };
    if let Ok(file) =
        Probe::open(path).and_then(|p| p.options(ParseOptions::new().read_cover_art(false)).read())
    {
        track.duration = file.properties().duration().as_secs_f64();
        if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
            if let Some(title) = tag.title().filter(|v| !v.trim().is_empty()) {
                track.title = title.into_owned();
            }
            if let Some(artist) = tag.artist().filter(|v| !v.trim().is_empty()) {
                track.artist = artist.into_owned();
            }
        }
    }
    track
}

pub fn cover_bytes(path: &Path) -> Option<Vec<u8>> {
    if let Ok(file) = lofty::read_from_path(path)
        && let Some(picture) = file
            .primary_tag()
            .or_else(|| file.first_tag())
            .and_then(|tag| tag.pictures().first())
        && picture.data().len() <= 20 * 1024 * 1024
    {
        return Some(picture.data().to_vec());
    }
    let parent = path.parent()?;
    for name in [
        "cover.jpg",
        "cover.png",
        "folder.jpg",
        "folder.png",
        "Cover.jpg",
        "Cover.png",
    ] {
        let candidate = parent.join(name);
        if fs::metadata(&candidate).is_ok_and(|m| m.len() <= 20 * 1024 * 1024)
            && let Ok(bytes) = fs::read(candidate)
        {
            return Some(bytes);
        }
    }
    None
}

// Compare digit runs numerically without parsing them (arbitrarily long filenames work).
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let (start_i, start_j) = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let mut aa = &a[start_i..i];
            let mut bb = &b[start_j..j];
            while aa.len() > 1 && aa[0] == b'0' {
                aa = &aa[1..];
            }
            while bb.len() > 1 && bb[0] == b'0' {
                bb = &bb[1..];
            }
            let cmp = aa.len().cmp(&bb.len()).then_with(|| aa.cmp(bb));
            if cmp != Ordering::Equal {
                return cmp;
            }
        } else {
            let cmp = a[i].cmp(&b[j]);
            if cmp != Ordering::Equal {
                return cmp;
            }
            i += 1;
            j += 1;
        }
    }
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

pub fn time_label(seconds: f64) -> String {
    let seconds = if seconds.is_finite() {
        seconds.max(0.0) as u64
    } else {
        0
    };
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

pub fn track_count(count: usize) -> String {
    let suffix = match (count % 10, count % 100) {
        (_, 11..=14) => "треков",
        (1, _) => "трек",
        (2..=4, _) => "трека",
        _ => "треков",
    };
    format!("{count} {suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folder_scan_is_direct_natural_case_insensitive_and_resilient() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["10 song.MP3", "2 song.flac", "1 song.WAV", "notes.txt"] {
            fs::write(dir.path().join(name), b"broken audio").unwrap();
        }
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested/3.mp3"), b"nested").unwrap();
        let tracks = scan_folder(dir.path()).unwrap();
        assert_eq!(
            tracks.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            ["1 song", "2 song", "10 song"]
        );
        assert!(tracks[0].matches("SONG"));
        assert!(tracks[0].matches("неизвестный"));
        assert!(!tracks[0].matches("unmatched"));
        assert!(scan_folder(&dir.path().join("missing")).is_err());
    }
    #[test]
    fn wav_metadata_is_read_without_tags() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("тест.wav");
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 8000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..16000 {
            writer.write_sample(0_i16).unwrap();
        }
        writer.finalize().unwrap();
        let track = read_track(path);
        assert_eq!(track.title, "тест");
        assert!((track.duration - 2.0).abs() < 0.01);
    }
    #[test]
    fn labels_and_long_numbers() {
        assert_eq!(track_count(11), "11 треков");
        assert_eq!(track_count(21), "21 трек");
        assert_eq!(time_label(3661.0), "1:01:01");
        assert_eq!(time_label(f64::NAN), "0:00");
        assert_eq!(
            natural_cmp("99999999999999999999", "100000000000000000000"),
            Ordering::Less
        );
    }
}
