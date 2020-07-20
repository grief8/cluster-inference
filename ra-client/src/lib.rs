mod context;
mod error;
mod client;

pub use crate::context::*;
pub use crate::error::*;
pub use crate::client::*;

pub type ClientRaResult<T> = Result<T, ClientRaError>;
