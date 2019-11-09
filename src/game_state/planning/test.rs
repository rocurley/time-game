use super::apply_plan;
use game_frame::GameFrame;
use ggez::nalgebra::Point2;
use proptest;
use proptest::prelude::*;
use std::collections::HashSet;
use types::{Direction, Move, Plan, Player};

static POSSIBLE_MOVES: [Move; 5] = [
    Move::Jump,
    Move::Direction(Direction::Up),
    Move::Direction(Direction::Down),
    Move::Direction(Direction::Left),
    Move::Direction(Direction::Right),
];

fn arbitrary_player() -> BoxedStrategy<Player> {
    ((0..10), (0..10))
        .prop_map(|(x, y)| Player::new(Point2::new(x, y)))
        .boxed()
}

fn valid_plan(game_frame: GameFrame) -> BoxedStrategy<Plan> {
    #[allow(clippy::range_plus_one)]
    let moves = proptest::collection::vec(
        proptest::sample::select(POSSIBLE_MOVES.as_ref()),
        game_frame.players.len()..game_frame.players.len() + 1,
    )
    .prop_map(move |moves_vec| {
        game_frame
            .players
            .iter()
            .map(|(id, _)| *id)
            .zip(moves_vec.into_iter())
            .collect()
    });
    let portals = prop_oneof![
        4 => Just(HashSet::new()),
        1 =>
    ((0..10), (0..10))
        .prop_map(|(x, y)| {
            let mut portals = HashSet::new();
            let portal = Point2::new(x, y);
            portals.insert(portal);
            portals
        })
    ];
    (moves, portals)
        .prop_map(|(moves, portals)| Plan { moves, portals })
        .boxed()
}

fn unfold_arbitrary_plans(depth: u32) -> BoxedStrategy<Vec<GameFrame>> {
    arbitrary_player()
        .prop_map(|player| {
            let mut frame = GameFrame::new();
            frame
                .insert_player(player)
                .expect("Failed to insert player");
            vec![frame]
        })
        .prop_recursive(depth, depth, 1, |prop_prior_frames| {
            prop_prior_frames
                .prop_flat_map(|prior_frames: Vec<GameFrame>| {
                    let prior_frame: GameFrame =
                        prior_frames.last().expect("Empty frames vec").clone();
                    valid_plan(prior_frame.clone())
                        .prop_map(move |plan| apply_plan(&prior_frame, &plan.clone()))
                        .prop_filter("plan wasn't allowed", Result::is_ok)
                        .prop_map(move |frame| {
                            let new_frame = frame.expect("Should have been filtered");
                            let mut new_frames = prior_frames.clone();
                            new_frames.push(new_frame);
                            new_frames
                        })
                })
                .boxed()
        })
        .boxed()
}

proptest! {
    #[allow(clippy::unnecessary_operation)] //Can't figure this one out
    #[test]
    fn test_apply_plan(ref game_frames in unfold_arbitrary_plans(10)) {
        for frame in game_frames.iter() {
            prop_assert_eq!(frame.players.len() - frame.portals.len(), 1);
        }
    }
}
#[test]
fn test_loop() {
    let game_frame_0 = GameFrame::new();
    let mut plan_0 = Plan::new();
    plan_0.portals.insert(Point2::new(0, 0));
    let game_frame_1 = apply_plan(&game_frame_0, &plan_0).expect("Couldn't create a portal");
    let player_id = game_frame_1
        .players
        .id_by_position(&Point2::new(0, 0))
        .expect("Couldn't find a player at (0,0)");
    let mut plan_1 = Plan::new();
    plan_1.moves.insert(player_id, Move::Jump);
    apply_plan(&game_frame_1, &plan_1).expect_err("Completed infinite loop");
}
