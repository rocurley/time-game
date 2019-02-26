#![feature(nll)]
#![feature(copied)]
#![allow(unknown_lints)]
#![warn(clippy::all)]

#[cfg(test)]
#[macro_use]
pub extern crate proptest;

extern crate ggez;
extern crate petgraph;
extern crate rand;

mod game_frame;
pub mod game_state;
mod portal_graph;
mod render;
mod tree;
pub mod types;
