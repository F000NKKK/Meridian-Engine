//! `AudioTrack` as a looping `SampleSource`.

use meridian_sdk::{AudioTrack, Repeat, SampleSource};

fn memory(mono: &[f32]) -> AudioTrack {
    track(mono, Repeat::Loop)
}

fn track(mono: &[f32], repeat: Repeat) -> AudioTrack {
    AudioTrack::Memory {
        mono: mono.to_vec(),
        cursor: 0,
        repeat,
    }
}

#[test]
fn a_memory_track_loops_across_the_buffer_end() {
    let mut track = memory(&[1.0, 2.0, 3.0]);
    let mut out = [0.0; 7];

    assert_eq!(track.read(&mut out), 7);

    assert_eq!(out, [1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0]);
}

#[test]
fn a_memory_track_continues_where_the_last_read_stopped() {
    let mut track = memory(&[1.0, 2.0, 3.0]);
    let mut first = [0.0; 2];
    let mut second = [0.0; 2];

    track.read(&mut first);
    track.read(&mut second);

    assert_eq!(first, [1.0, 2.0]);
    assert_eq!(second, [3.0, 1.0]);
}

#[test]
fn an_empty_memory_track_is_silent() {
    let mut track = memory(&[]);
    let mut out = [1.0; 3];

    assert_eq!(track.read(&mut out), 3);

    assert_eq!(out, [0.0; 3]);
}

#[test]
fn a_one_shot_track_ends_at_its_last_sample() {
    let mut clip = track(&[1.0, 2.0, 3.0], Repeat::Once);
    let mut out = [9.0; 8];

    assert_eq!(clip.read(&mut out), 3);
    assert_eq!(&out[..3], &[1.0, 2.0, 3.0]);
    assert_eq!(clip.read(&mut out), 0, "stays ended");
}

#[test]
fn a_one_shot_track_ends_across_reads() {
    let mut clip = track(&[1.0, 2.0, 3.0], Repeat::Once);
    let mut out = [0.0; 2];

    assert_eq!(clip.read(&mut out), 2);
    assert_eq!(clip.read(&mut out), 1);
    assert_eq!(out[0], 3.0);
}
