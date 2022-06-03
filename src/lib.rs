#![allow(clippy::single_component_path_imports)]

pub use xaoc_lib::*;

#[cfg(feature = "dynamic")]
#[allow(unused_imports)]
use xaoc_lib;
