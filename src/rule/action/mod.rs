mod modify;
use modify::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Reject,
    Redirect(String),
    ModifyRequest(Modify),
    ModifyResponse(Modify),
}
