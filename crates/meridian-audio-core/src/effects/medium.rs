//! The acoustic medium sound propagates through. Speed of sound, density
//! and high-frequency absorption are properties of the *medium*, not
//! constants of the renderer — air at altitude, water and custom media
//! all change how the same scene sounds.

/// An acoustic medium: what the sound travels through between emitter
/// and listener.
///
/// Physically, the speed of sound is `sqrt(K / density)` (bulk modulus
/// over density) — density alone doesn't determine it, so both are
/// stored and every preset carries a measured, consistent pair. Custom
/// media can be built from a literal (all fields public) or derived via
/// [`from_bulk_modulus`](Self::from_bulk_modulus).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcousticMedium {
    /// Density in kg/m³.
    pub density_kg_m3: f32,
    /// Speed of sound in m/s — sets the interaural time difference.
    pub speed_of_sound_m_s: f32,
    /// How strongly the medium absorbs high frequencies with distance,
    /// per meter (an exponential cutoff decay factor: the shadow/air
    /// filter cutoff is scaled by `exp(-absorption * distance)`).
    /// Air absorbs noticeably; water barely does at audible frequencies.
    pub high_freq_absorption_per_m: f32,
}

impl AcousticMedium {
    /// Dry air at 15 °C, 0 m above sea level (ISA): 1.225 kg/m³, 340 m/s.
    pub fn air_sea_level() -> Self {
        Self {
            density_kg_m3: 1.225,
            speed_of_sound_m_s: 340.3,
            high_freq_absorption_per_m: 0.012,
        }
    }

    /// Dry air at `altitude_m` above sea level, per the International
    /// Standard Atmosphere troposphere model (valid to ~11 km): linear
    /// temperature lapse of 6.5 °C/km sets the speed of sound
    /// (`c = 331.3 * sqrt(1 + T/273.15)`), an exponential barometric
    /// falloff (~8.5 km scale height) the density.
    pub fn air_at_altitude(altitude_m: f32) -> Self {
        let altitude = altitude_m.clamp(0.0, 11_000.0);
        let temperature_c = 15.0 - 6.5e-3 * altitude;
        Self {
            density_kg_m3: 1.225 * (-altitude / 8_500.0).exp(),
            speed_of_sound_m_s: 331.3 * (1.0 + temperature_c / 273.15).sqrt(),
            // Thinner air absorbs slightly less.
            high_freq_absorption_per_m: 0.012 * (-altitude / 8_500.0).exp().max(0.5),
        }
    }

    /// Pure (fresh) water at 20 °C: 998 kg/m³, 1481 m/s.
    pub fn fresh_water() -> Self {
        Self {
            density_kg_m3: 998.0,
            speed_of_sound_m_s: 1481.0,
            high_freq_absorption_per_m: 0.002,
        }
    }

    /// Sea water at 20 °C, typical salinity: 1025 kg/m³, 1522 m/s.
    pub fn sea_water() -> Self {
        Self {
            density_kg_m3: 1025.0,
            speed_of_sound_m_s: 1522.0,
            high_freq_absorption_per_m: 0.003,
        }
    }

    /// A custom medium from its density and bulk modulus (Pa):
    /// `c = sqrt(K / density)` — the physically consistent way to invent
    /// a medium when only material properties are known.
    pub fn from_bulk_modulus(
        density_kg_m3: f32,
        bulk_modulus_pa: f32,
        high_freq_absorption_per_m: f32,
    ) -> Self {
        Self {
            density_kg_m3,
            speed_of_sound_m_s: (bulk_modulus_pa / density_kg_m3.max(1e-6)).sqrt(),
            high_freq_absorption_per_m,
        }
    }
}

impl Default for AcousticMedium {
    fn default() -> Self {
        Self::air_sea_level()
    }
}

#[cfg(test)]
mod medium_tests {
    use super::*;

    #[test]
    fn presets_are_physically_ordered() {
        let air = AcousticMedium::air_sea_level();
        let mountain = AcousticMedium::air_at_altitude(4_000.0);
        let fresh = AcousticMedium::fresh_water();
        let sea = AcousticMedium::sea_water();

        assert!(mountain.density_kg_m3 < air.density_kg_m3);
        assert!(mountain.speed_of_sound_m_s < air.speed_of_sound_m_s); // colder up high
        assert!(fresh.speed_of_sound_m_s > 4.0 * air.speed_of_sound_m_s);
        assert!(sea.speed_of_sound_m_s > fresh.speed_of_sound_m_s);
        assert!(sea.density_kg_m3 > fresh.density_kg_m3);
    }

    #[test]
    fn bulk_modulus_reproduces_water() {
        // Water: K ≈ 2.19 GPa, ρ ≈ 998 -> c ≈ 1481 m/s.
        let m = AcousticMedium::from_bulk_modulus(998.0, 2.19e9, 0.002);
        assert!((m.speed_of_sound_m_s - 1481.0).abs() < 15.0);
    }
}
