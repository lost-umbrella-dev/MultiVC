use serde::{Deserialize, Serialize};
use strum::VariantArray;

#[derive(Debug, Clone, Copy, PartialEq, VariantArray, Serialize, Deserialize)]
pub enum Clients {
    Github,
}
