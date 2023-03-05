#[macro_use]
extern crate derive_builder;

pub mod cli;

pub mod workspace;

pub mod config;

pub mod init;

pub mod commit;

pub mod objects;

pub mod hex;

pub mod index;

pub mod add;

pub mod hashing;

mod file;

pub mod rm;

mod refs;

pub mod output;

pub mod status;

pub mod diff;

mod object_resolver;
