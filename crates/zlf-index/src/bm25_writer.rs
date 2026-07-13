use tantivy::{doc, IndexWriter, Term};
use zlf_core::Result;

use crate::bm25_support::{document_key, entity_parts, internal, DocumentParts};
use crate::{BM25Index, IndexDocument};

impl BM25Index {
    pub(crate) fn write_index_document(
        &self,
        writer: &mut IndexWriter,
        document: &IndexDocument,
    ) -> Result<()> {
        let (entity_kind, entity_id) = entity_parts(&document.id.entity);
        let key = document_key(&document.id);
        self.write_document(
            writer,
            DocumentParts {
                key: &key,
                entity_kind,
                entity_id,
                field: &document.id.field,
                chunk: &document.id.chunk_id,
                language: document.language.as_deref().unwrap_or(""),
                text: &document.content,
            },
        )
    }

    pub(crate) fn write_document(
        &self,
        writer: &mut IndexWriter,
        parts: DocumentParts<'_>,
    ) -> Result<()> {
        writer.delete_term(Term::from_field_text(self.fields.key, parts.key));
        writer
            .add_document(doc!(
                self.fields.key => parts.key,
                self.fields.entity_kind => parts.entity_kind,
                self.fields.entity_id => parts.entity_id,
                self.fields.field => parts.field,
                self.fields.chunk => parts.chunk,
                self.fields.language => parts.language,
                self.fields.body => self.tokenize(parts.text).join(" ")
            ))
            .map(|_| ())
            .map_err(internal)
    }

    pub(crate) fn remove_key(&self, key: &str) -> Result<()> {
        let mut writer = self.writer.lock().map_err(internal)?;
        writer.delete_term(Term::from_field_text(self.fields.key, key));
        self.commit(&mut writer)
    }

    pub(crate) fn commit(&self, writer: &mut IndexWriter) -> Result<()> {
        writer.commit().map_err(internal)?;
        self.reader.reload().map_err(internal)
    }
}
