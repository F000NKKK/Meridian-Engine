//! The packed inputs of one soft-body step. A flavor packs its scalars into 4-byte words (`f32`
//! little-endian bytes, or `Fixed` raw bits) and unpacks the result; the
//! buffer plumbing, in [`super::dispatch`], is identical.

use meridian_gac::GaFlavor;

use super::csr::Adjacency;

/// One 32-bit GPU word, little-endian.
pub(super) type Word = [u8; 4];

/// Everything one step uploads, already packed as bytes.
pub(super) struct StepInputs {
    pub params: Vec<u8>,
    pub positions: Vec<u8>,
    pub velocities: Vec<u8>,
    pub inverse_masses: Vec<u8>,
    pub edge_offsets: Vec<u8>,
    pub edge_neighbor: Vec<u8>,
    pub edge_params: Vec<u8>,
    pub particle_count: usize,
}

/// Zero-length GPU buffers are rejected by some backends; pads an empty
/// byte vector to `min_len` zero bytes (inert: the shader never reads it).
pub(super) fn pad_or(bytes: Vec<u8>, min_len: usize) -> Vec<u8> {
    if bytes.is_empty() {
        vec![0u8; min_len]
    } else {
        bytes
    }
}

/// Concatenates words into a byte vector.
pub(super) fn words_to_bytes(words: impl IntoIterator<Item = Word>) -> Vec<u8> {
    words.into_iter().flatten().collect()
}

/// The uniform buffer: nine scalar words (gravity, `dt`, ground normal and
/// offset, restitution), the particle count and two padding words.
pub(super) fn params_bytes(scalars: [Word; 9], particle_count: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(48);
    for word in scalars {
        bytes.extend_from_slice(&word);
    }
    bytes.extend_from_slice(&(particle_count as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes
}

/// Packs 3-component items as consecutive words.
pub(super) fn pack_vec3s<V: Copy>(items: &[V], words: impl Fn(V) -> [Word; 3]) -> Vec<u8> {
    words_to_bytes(items.iter().flat_map(|&item| words(item)))
}

/// The packed CSR topology of a step: offsets, neighbors and interleaved
/// edge parameters; `word` turns one flavor scalar into a word.
pub(super) fn edge_bytes<F: GaFlavor>(
    adjacency: &Adjacency<F>,
    word: impl Fn(F::Scalar) -> Word,
) -> [Vec<u8>; 3] {
    let offsets = words_to_bytes(adjacency.offsets.iter().map(|v| v.to_le_bytes()));
    // A body with no springs at all (single free particle) would otherwise
    // allocate zero-length buffers; pad to one element. The shader never
    // reads past `edge_offsets[particle_count]`, so the padding is inert.
    let neighbor = pad_or(
        words_to_bytes(adjacency.neighbor.iter().map(|v| v.to_le_bytes())),
        4,
    );
    // Interleaved [rest_length, stiffness, damping] per half-edge — see the
    // WGSL `edge_params` binding's comment for why this is one buffer.
    let params = pad_or(
        words_to_bytes((0..adjacency.rest_length.len()).flat_map(|i| {
            [
                word(adjacency.rest_length[i]),
                word(adjacency.stiffness[i]),
                word(adjacency.damping[i]),
            ]
        })),
        4,
    );
    [offsets, neighbor, params]
}

/// Reads `count` 3-word items back out of a readback buffer.
pub(super) fn unpack_vec3s<V>(bytes: &[u8], count: usize, item: impl Fn([Word; 3]) -> V) -> Vec<V> {
    (0..count)
        .map(|i| {
            let base = i * 12;
            item([
                crate::readback::le_word(bytes, base),
                crate::readback::le_word(bytes, base + 4),
                crate::readback::le_word(bytes, base + 8),
            ])
        })
        .collect()
}
