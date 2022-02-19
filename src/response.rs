use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    GetOk(String),
    SetOk,
    RmOk,
    Error(String),
}
