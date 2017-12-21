extern crate types;

use self::types::{Constraint, Direction, GameFrame, Move, Plan, Player};

use std::collections::hash_map::Entry;

pub fn apply_plan(initial_frame: &GameFrame, plan: &Plan) -> Result<GameFrame, &'static str> {
    let mut constraints = initial_frame.constraints.clone();
    let mut players = initial_frame
        .players
        .iter()
        .filter_map(|old_player: &Player| match plan.moves.get(&old_player.id) {
            None => Some(Ok(old_player.clone())),
            Some(&Move::Direction(ref direction)) => {
                let mut player: Player = old_player.clone();
                match *direction {
                    Direction::Up => player.position.0 -= 1,
                    Direction::Down => player.position.0 += 1,
                    Direction::Left => player.position.1 -= 1,
                    Direction::Right => player.position.1 += 1,
                }
                Some(Ok(player))
            }
            Some(&Move::Jump) => {
                if let Entry::Occupied(constraint) = constraints.entry(old_player.position) {
                    constraint.remove_entry();
                    None
                } else {
                    Some(Err("Tried to close loop at wrong positon"))
                }
            }
        })
        .collect::<Result<Vec<Player>, &str>>()?;
    for pos in plan.portals.iter() {
        players.push(Player::new(*pos));
        match constraints.entry(*pos) {
            Entry::Occupied(_) => return Err("Overlapping portals prohibited."),
            Entry::Vacant(vacant_entry) => vacant_entry.insert(Constraint::new(0, *pos)),
        };
    }
    Ok(GameFrame {
        players,
        constraints,
    })
}
