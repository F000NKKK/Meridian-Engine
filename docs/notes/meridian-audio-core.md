# `meridian-audio-domain` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

No elevation/height channels (Dolby Atmos-style object audio) —
standard consumer speaker layouts are one horizontal plane, and
panning here only ever looks at horizontal azimuth. No measured HRTF —
that needs an impulse-response database this crate doesn't have. The
[`Mixer`] path is amplitude-only panning (front/back collapse on
headphones — see [`fold_to_front_hemisphere`]); the
[`effects::BinauralRenderer`] path is the physically-motivated
headphone upgrade: interaural time difference, head-shadow filtering
and rear damping, so front and back genuinely differ there. Effects
live one-per-file under [`effects`].
