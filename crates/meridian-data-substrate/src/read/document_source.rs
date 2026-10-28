//! [`DocumentSource`]: a document's text or bytes, parsed and read.

use substrate_data::{Bump, Value, binary, text};

use super::placement::Placement;
use super::{DocumentRole, ReadContext};
use crate::draft::DocumentDraft;
use crate::load::LoadError;

/// A document as it arrives: text, or the `SDAT` binary form.
#[derive(Clone, Copy)]
pub(crate) enum DocumentSource<'a> {
    /// Text.
    Text(&'a str),
    /// The binary form.
    Binary(&'a [u8]),
    /// A document already decoded (a save's `scene`): problems have paths.
    Document(&'a Value),
}

impl<'a> DocumentSource<'a> {
    /// `bytes` as the form their magic says: `SDAT` is binary, anything
    /// else is text (as every Substrate Data reader and `asset-domain`'s
    /// decoders do, never by extension).
    ///
    /// # Errors
    ///
    /// [`LoadError`] when it isn't binary and isn't UTF-8 either.
    pub(crate) fn detect(bytes: &'a [u8]) -> Result<Self, LoadError> {
        if bytes.starts_with(binary::MAGIC) {
            return Ok(DocumentSource::Binary(bytes));
        }
        match std::str::from_utf8(bytes) {
            Ok(text) => Ok(DocumentSource::Text(text)),
            Err(e) => Err(LoadError::not_utf8(e.valid_up_to())),
        }
    }

    /// Parses and reads the document: a draft when it is right, else
    /// every problem in it, placed.
    pub(crate) fn draft(
        self,
        role: &DocumentRole,
        cx: &mut ReadContext<'_>,
    ) -> Result<DocumentDraft, LoadError> {
        let arena = Bump::new();
        match self {
            DocumentSource::Text(source) => {
                let doc = text::parse(source).map_err(LoadError::syntax)?;
                let root = doc.value.borrow(&arena);
                Placement::Text(source, &doc).draft(cx.read(role, &root))
            }
            DocumentSource::Binary(bytes) => {
                let root = binary::decode_ref(bytes, &arena).map_err(LoadError::decode)?;
                Placement::Binary.draft(cx.read(role, &root))
            }
            DocumentSource::Document(value) => {
                let root = value.borrow(&arena);
                Placement::Binary.draft(cx.read(role, &root))
            }
        }
    }
}
