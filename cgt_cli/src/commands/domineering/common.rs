use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DomineeringResult {
    pub grid: String,
    pub temperature: String,
}
