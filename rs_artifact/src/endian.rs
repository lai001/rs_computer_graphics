use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum EEndianType {
    Big,
    Little,
    Native,
}
