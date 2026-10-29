# Mellow

Mellow is a minimal desktop music player for Windows. Each music folder becomes a playlist, with tracks played directly from your files in a compact, fixed-size window.

## Benefits

- One folder, one playlist, with natural track ordering and instant search.
- Windows 11 Acrylic transparency tinted by the current album artwork.
- Pin the player above other windows; the setting is remembered between launches.
- Gapless playback with a prepared track queue and decoder delay/padding trimming where supported by the audio file. Intentional silence in recordings is preserved.
- Five-band equalizer: 60 Hz, 250 Hz, 1 kHz, 4 kHz and 12 kHz, with ±12 dB adjustment and Flat, Bass and Voice presets.
- Automatic Windows audio-output switching, plus manual device selection.
- Embedded artwork and metadata, shuffle, track or folder repeat, and saved preferences. Damaged covers are skipped in favor of another embedded picture or a local cover file.
- Local playback of MP3, FLAC, WAV, Ogg Vorbis, AAC/M4A and AIFF. No account required.

## Controls

| Action | Control |
| --- | --- |
| Add music folders | Add in the folder dropdown or settings, folder-plus button, drag and drop, or `Ctrl+O` |
| Switch playlists | Dropdown folder selector at the top |
| Keep the player above other windows | `…` → Pin (Закрепить); choose Unpin (Открепить) to disable |
| Open the track list | List button at the bottom right |
| Search the current folder | `Ctrl+F` |
| Play or pause | Play button or `Space` |
| Next / previous track | Transport buttons or `Ctrl+Right` / `Ctrl+Left` |
| Seek by five seconds | `Right` / `Left` |
| Seek to a position | Playback progress bar |
| Adjust volume or mute | Volume slider or speaker button |
| Shuffle / repeat | Transport controls |
| Open settings | `…` → Settings (a separate window) |
| Equalizer / audio output | Settings → Sound in the left sidebar; refresh icon beside the output selector |
| Add, refresh, or remove folders | Settings → Folders; refresh and remove icons on each folder card |
| Refresh the selected folder | `F5` or `…` → Refresh folder |
| Return to the player | `Esc` or the music-note button |

The interface currently uses Russian labels. Playback shortcuts apply when text fields and other controls do not have keyboard focus.

Playback controls and the volume slider stay in the same position when switching between the player and track list. Settings apply immediately and are saved automatically. `Esc` dismisses an open dropdown first, or closes the settings window when it has focus.

## Stack

Rust 2024 · eframe/egui · Windows DWM Acrylic · Segoe UI and Segoe Fluent Icons · Rodio/CPAL/WASAPI · Symphonia · RBJ biquad equalizer · Lofty · image · rfd · Serde/JSON.

