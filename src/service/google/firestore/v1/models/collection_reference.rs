use super::document_reference::DocumentReference;

#[derive(Clone)]
pub struct CollectionReference {
    id: String,
    parent: Option<Box<DocumentReference>>,
}

impl CollectionReference {
    pub(crate) fn new(id: impl Into<String>, parent: Option<&DocumentReference>) -> Self {
        CollectionReference {
            id: id.into(),
            parent: parent.map(|p| Box::new(p.clone())),
        }
    }

    pub fn doc(&self, id: impl Into<String>) -> DocumentReference {
        DocumentReference::new(id.into(), self)
    }

    pub fn path(&self) -> String {
        match self.parent {
            Some(ref parent) => format!("{}/{}", parent.path(), self.id),
            None => self.id.clone(),
        }
    }
}
