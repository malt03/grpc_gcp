use crate::proto::google::firestore::v1::value::ValueType;

impl ValueType {
    pub fn is_some_value(&self) -> bool {
        if let ValueType::NullValue(_) = self {
            false
        } else {
            true
        }
    }
}
