#![feature(nll)]
#![warn(clippy::all)]
#![allow(clippy::try_err)]

#[cfg(test)]
#[macro_use]
pub extern crate proptest;
extern crate ggez;
extern crate petgraph;
extern crate rand;
#[macro_use]
extern crate slotmap;
#[macro_use]
extern crate derivative;
extern crate objekt;
#[macro_use]
extern crate enum_map;
#[macro_use]
extern crate enumset;

mod game_frame;
pub mod game_state;
mod portal_graph;
mod render;
mod tree;
pub mod types;
