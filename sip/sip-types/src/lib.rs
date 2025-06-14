#![forbid(unsafe_code)]
// header::name::Name triggers these because of `Bytes`
#![allow(
    clippy::declare_interior_mutable_const,
    clippy::borrow_interior_mutable_const
)]

#[macro_use]
mod macros;
#[macro_use]
pub mod print;
#[macro_use]
pub mod parse;
#[macro_use]
pub mod uri;
mod code;
pub mod header;
pub mod host;
mod method;
pub mod msg;

pub use code::CodeKind;
pub use code::StatusCode;

pub use method::Method;

pub use header::headers::Headers;
pub use header::name::Name;

#[doc(hidden)]
pub mod _private_reexport {
    pub use bytes::Bytes;
    pub use internal::{IResult, identity};
    pub use nom;
}
