//! The Substrate Data schema of [`Fixed`] (`data` feature): stored as its
//! raw Q16.16 bits in every form — an int in text and binary, a varint in
//! a frame — and never converted through a float, so deterministic state
//! round-trips bit-exactly. Tools know it as `fixed`. The float flavour
//! needs nothing here: `substrate-data` implements `Schema` for `f32`.
//! See docs/data-formats.md ("Math values").

use substrate_data::schema::NamedType;

use crate::Fixed;

impl NamedType for Fixed {
    const NAME: &'static str = "fixed";
    type Stored = i32;

    fn to_stored(&self) -> i32 {
        self.to_bits()
    }

    fn from_stored(bits: i32) -> Option<Self> {
        Some(Fixed::from_bits(bits))
    }
}

substrate_data::named_schema!(Fixed);

#[cfg(test)]
mod tests {
    use substrate_data::{Schema, frame, read_bytes, to_bytes};

    use super::*;

    const SAMPLES: [i32; 6] = [0, 1, -1, 98304, i32::MIN, i32::MAX];

    #[test]
    fn text_is_the_raw_bits() {
        let f = Fixed::from_num(1.5);
        assert_eq!(substrate_data::to_text(&f).trim(), "98304");
        let back: Fixed = substrate_data::from_text("98304").unwrap().value;
        assert_eq!(back, f);
    }

    #[test]
    fn every_form_round_trips_bit_exactly() {
        for bits in SAMPLES {
            let f = Fixed::from_bits(bits);
            let text = substrate_data::to_text(&f);
            let t: Fixed = substrate_data::from_text(&text).unwrap().value;
            let b: Fixed = read_bytes(&to_bytes(&f)).unwrap();
            let fr: Fixed = frame::read_frame(&frame::to_frame(&f)).unwrap();
            assert_eq!([t, b, fr].map(Fixed::to_bits), [bits; 3]);
        }
        let list: Vec<Fixed> = SAMPLES.into_iter().map(Fixed::from_bits).collect();
        let b: Vec<Fixed> = read_bytes(&to_bytes(&list)).unwrap();
        let bytes = frame::to_frame(&list);
        let fr: Vec<Fixed> = frame::read_frame(&bytes).unwrap();
        assert_eq!((b, fr), (list.clone(), list));
        assert!(frame::read_shaped(&bytes, &Vec::<Fixed>::shape()).is_some());
    }

    #[test]
    fn bits_beyond_i32_are_refused() {
        let e = substrate_data::from_text::<Fixed>("4294967296").unwrap_err();
        assert_eq!(
            e.0[0].code,
            Some(substrate_data::Problem::OutOfRange.into())
        );
    }

    #[test]
    fn describes_itself_as_named_fixed() {
        assert!(matches!(Fixed::shape(), substrate_data::Shape::Named(n) if n.name() == "fixed"));
    }
}
