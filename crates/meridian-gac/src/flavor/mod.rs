//! Flavor abstraction: the traits generic code is written against, and the two flavors.

mod bivector_like;
pub mod fixed;
pub mod float;
mod ga_flavor;
mod motor_like;
mod rotor_like;
mod scalar_like;
mod vector_like;

pub use bivector_like::BivectorLike;
pub use ga_flavor::GaFlavor;
pub use motor_like::MotorLike;
pub use rotor_like::RotorLike;
pub use scalar_like::ScalarLike;
pub use vector_like::VectorLike;
