//! The rendering components register and convert to their file forms.

use meridian_data_substrate::{ComponentRegistry, LightForm, register_rendering};
use meridian_rendering_domain::{LightKind, LightSource};

fn spot() -> LightSource {
    LightSource {
        kind: LightKind::Spot {
            range: 12.0,
            inner_angle: 0.3,
            outer_angle: 0.5,
        },
        color: [1.0, 0.8, 0.6],
        intensity: 4.0,
    }
}

#[test]
fn a_light_survives_its_file_form() {
    for light in [
        spot(),
        LightSource {
            kind: LightKind::Directional,
            color: [1.0; 3],
            intensity: 1.0,
        },
        LightSource {
            kind: LightKind::Point { range: 5.0 },
            color: [0.2, 0.4, 0.6],
            intensity: 2.5,
        },
    ] {
        assert_eq!(LightSource::from(LightForm::from(light)), light);
    }
}

#[test]
fn light_registers_once_under_its_name() {
    let mut registry = ComponentRegistry::new();
    register_rendering(&mut registry).expect("the first registration succeeds");
    assert!(registry.names().any(|name| name.as_str() == "light"));
    assert!(register_rendering(&mut registry).is_err());
}
