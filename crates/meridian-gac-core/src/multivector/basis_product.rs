//! Geometric product of two basis blades, shared by both flavors.

/// Geometric product of two basis blades, given as 4-bit masks: reorders
/// the concatenated factors into canonical increasing order (bubble sort,
/// tracking the sign of the permutation) and contracts adjacent equal
/// factors via the metric (`e0*e0 = 0`, `ei*ei = 1` for i=1,2,3). Returns
/// `(0, _)` when the product vanishes (an `e0` factor met itself), else a
/// sign of `1` or `-1`; each flavor turns that sign into its scalar type.
pub(super) fn basis_product(a: u8, b: u8) -> (i8, u8) {
    let mut seq: [u8; 8] = [0; 8];
    let mut len = 0usize;
    for i in 0..4u8 {
        if a & (1 << i) != 0 {
            seq[len] = i;
            len += 1;
        }
    }
    for i in 0..4u8 {
        if b & (1 << i) != 0 {
            seq[len] = i;
            len += 1;
        }
    }

    let mut sign: i8 = 1;
    loop {
        let mut swapped = false;
        for i in 0..len.saturating_sub(1) {
            if seq[i] > seq[i + 1] {
                seq.swap(i, i + 1);
                sign = -sign;
                swapped = true;
            }
        }

        let mut contracted = false;
        let mut i = 0;
        while i + 1 < len {
            if seq[i] == seq[i + 1] {
                if seq[i] == 0 {
                    return (0, 0); // e0 * e0 = 0
                }
                for k in i..len - 2 {
                    seq[k] = seq[k + 2];
                }
                len -= 2;
                contracted = true;
            } else {
                i += 1;
            }
        }

        if !swapped && !contracted {
            break;
        }
    }

    let mut mask = 0u8;
    for &b in seq.iter().take(len) {
        mask |= 1 << b;
    }
    (sign, mask)
}
