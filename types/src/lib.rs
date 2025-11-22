#![forbid(unsafe_code)]
#![deny(trivial_casts, trivial_numeric_casts)]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod address;
mod aliases;
mod context;
mod genesis;
mod height;
mod proposal;
mod proposal_part;
mod signing;
mod validator_set;
mod value;
mod vote;

pub mod codec;
pub mod proposer_selector;
pub mod proto;
pub mod utils;

pub use crate::{
    address::*, aliases::*, context::*, genesis::*, height::*, proposal::*, proposal_part::*,
    signing::*, validator_set::*, value::*, vote::*,
};
