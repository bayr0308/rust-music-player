use mellow::audio::Audio;
use std::{thread, time::Duration};

#[test]
#[ignore = "requires an audio output device"]
fn real_audio_device_pause_seek_resume_and_end() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("silence.wav");
    let mut wav = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for _ in 0..24000 {
        wav.write_sample(0_i16).unwrap();
    }
    wav.finalize().unwrap();
    let mut audio = Audio::default();
    audio.play(&path, 0.0).unwrap();
    thread::sleep(Duration::from_millis(150));
    assert!(!audio.paused());
    assert!(!audio.finished());
    assert!(audio.position() > 0.0);
    audio.toggle();
    assert!(audio.paused());
    let paused_at = audio.position();
    thread::sleep(Duration::from_millis(60));
    assert!((audio.position() - paused_at).abs() < 0.03);
    // Reopening an endpoint must retain the paused track and its seek position.
    audio.seek(1.2).unwrap();
    let output = audio.output_name().unwrap().to_owned();
    audio.select_output(Some(output.clone())).unwrap();
    assert_eq!(audio.output_name(), Some(output.as_str()));
    assert!(audio.paused());
    assert!((audio.position() - 1.2).abs() < 0.05);
    assert!(
        audio
            .select_output(Some("nonexistent test endpoint 749fe1".into()))
            .is_err()
    );
    assert_eq!(audio.output_name(), Some(output.as_str()));
    assert!(audio.paused());
    audio.select_output(None).unwrap();
    assert!(audio.check_output().is_none());
    assert!((audio.position() - 1.2).abs() < 0.05);
    audio.seek(2.8).unwrap();
    audio.toggle();
    assert!(!audio.paused());
    thread::sleep(Duration::from_millis(600));
    assert!(audio.finished());
    // A real sink must advance without UI involvement, report the next track,
    // and reset its position to that track rather than the whole queue.
    audio.play_track(&path, 0.0, 7).unwrap();
    audio.enqueue(&path, 8).unwrap();
    audio.enqueue(&path, 9).unwrap();
    audio.seek(2.85).unwrap();
    thread::sleep(Duration::from_millis(450));
    assert_eq!(audio.take_transitions(), vec![8]);
    assert!(!audio.finished());
    assert!(audio.position() < 1.0);
    assert_eq!(audio.upcoming_count(), 1);
    audio.toggle();
    audio.clear_upcoming().unwrap();
    assert!(audio.paused());
    assert_eq!(audio.upcoming_count(), 0);
    audio.enqueue(&path, 10).unwrap();
    audio.seek(2.85).unwrap();
    audio.toggle();
    thread::sleep(Duration::from_millis(450));
    assert_eq!(audio.take_transitions(), vec![10]);
    assert!(!audio.finished());
    assert!(audio.play(&dir.path().join("missing.wav"), 0.0).is_err());
    audio.stop();
    assert!(audio.paused());
}
