use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DomineeringResult {
    pub grid: String,
    pub canonical: String,
    pub temperature: String,
}
