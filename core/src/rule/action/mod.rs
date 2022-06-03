pub mod js;
mod log;
mod modify;

pub use self::log::*;
pub use modify::Modify;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Action {
    Reject,
    Redirect(String),
    ModifyRequest(Modify),
    ModifyResponse(Modify),
    LogRes,
    LogReq,
    Js(String),
}
