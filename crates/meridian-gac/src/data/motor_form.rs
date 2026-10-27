//! [`MotorForm`]: how a motor is stored.

use substrate_data::Schema;

/// How a motor is stored (the `Stored` form of `Motor3`/`FixedMotor3`): a
/// rigid motion that rotates by `rotor`, then moves the origin to
/// `translation`. Written with `dual` instead of `translation` when
/// rebuilding the motor from the two wouldn't give the same bits.
#[derive(Debug, Clone, Copy, PartialEq, Schema)]
pub struct MotorForm<S: Schema> {
    /// The rotation: `[s, e23, e31, e12]`.
    pub(super) rotor: [S; 4],
    /// Where the origin goes, after the rotation: `[x, y, z]`; default `[0, 0, 0]`.
    #[data(default)]
    pub(super) translation: Option<[S; 3]>,
    /// The motor's `e0` part `[e01, e02, e03, e0123]` as stored.
    #[data(default)]
    pub(super) dual: Option<[S; 4]>,
}
