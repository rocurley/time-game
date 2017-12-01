extern crate types;

use self::types::{GameState, GameFrame, Constraint, Move, Plan, Player};

use std::collections::hash_map::Entry;

pub fn apply_plan(initial_frame : & GameFrame, plan : Plan) -> Result<GameFrame, & 'static str> {
    let mut constraints = initial_frame.constraints.clone();
    initial_frame.players.iter().filter_map(|old_player : & Player| {
        let mut player : Player = old_player.clone();
        match plan.moves.get(& old_player.get_id()) {
            None => {},
            Some(& Move::Up)    => player.position.1 += 1,
            Some(& Move::Down)  => player.position.1 -= 1,
            Some(& Move::Left)  => player.position.0 -= 1,
            Some(& Move::Right) => player.position.0 += 1,
            Some(& Move::Jump) => {
                if let Entry::Occupied(constraint) = constraints.entry(player.position){
                    constraint.remove_entry();
                    return None;
                } else {
                    return Some(Err("Tried to close loop at wrong positon"));
                }
            }
        }
        return Some(Ok(player))
    }).collect::<Result<Vec<Player>,& str>>().map(|players| {
        GameFrame{ players, constraints}
    })
}


