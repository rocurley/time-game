use super::apply_plan;
use crate::{
    game_frame::GameFrame,
    types::{player_at, Direction, Entity, ImageMap, Move, Plan, Point},
};
use ggez::nalgebra::Point2;
use proptest::{self, prelude::*};
use std::collections::HashSet;

static POSSIBLE_MOVES: [Move; 5] = [
    Move::Jump,
    Move::Direction(Direction::Up),
    Move::Direction(Direction::Down),
    Move::Direction(Direction::Left),
    Move::Direction(Direction::Right),
];

fn arbitrary_point() -> BoxedStrategy<Point> {
    ((0..10), (0..10))
        .prop_map(|(x, y)| Point2::new(x, y))
        .boxed()
}

fn valid_plan(game_frame: GameFrame) -> BoxedStrategy<Plan> {
    let players: Vec<Entity> = game_frame
        .ecs
        .players
        .iter()
        .map(|(k, _)| k)
        .filter(|k| game_frame.ecs.entities.contains_key(*k))
        .collect();
    #[allow(clippy::range_plus_one)]
    let moves = proptest::collection::vec(
        proptest::sample::select(POSSIBLE_MOVES.as_ref()),
        players.len()..players.len() + 1,
    )
    .prop_map(move |moves_vec| players.iter().copied().zip(moves_vec.into_iter()).collect());
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
    let image_map_owned = ImageMap::mock();
    let image_map = &*Box::leak(Box::new(image_map_owned));
    arbitrary_point()
        .prop_map(move |pos| {
            let mut frame = GameFrame::new();
            frame
                .insert_player(image_map, pos)
                .expect("Failed to insert player");
            vec![frame]
        })
        .prop_recursive(depth, depth, 1, move |prop_prior_frames| {
            prop_prior_frames
                .prop_flat_map(move |prior_frames: Vec<GameFrame>| {
                    let prior_frame: GameFrame =
                        prior_frames.last().expect("Empty frames vec").clone();
                    valid_plan(prior_frame.clone())
                        .prop_map(move |plan| apply_plan(&image_map, &prior_frame, &plan))
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
    #[test]
    fn test_apply_plan(ref game_frames in unfold_arbitrary_plans(10)) {
        for frame in game_frames.iter() {
            let n_players = frame.ecs.players.iter().filter(|(k,_)| frame.ecs.entities.contains_key(*k)).count();
            prop_assert_eq!(n_players - frame.portals.len(), 1);
        }
    }
}
#[test]
fn test_loop() {
    let image_map = ImageMap::mock();
    let game_frame_0 = GameFrame::new();
    let mut plan_0 = Plan::new();
    plan_0.portals.insert(Point2::new(0, 0));
    let game_frame_1 =
        apply_plan(&image_map, &game_frame_0, &plan_0).expect("Couldn't create a portal");
    let player_id =
        player_at(&game_frame_1.ecs, Point2::new(0, 0)).expect("Couldn't find a player at (0,0)");
    let mut plan_1 = Plan::new();
    plan_1.moves.insert(player_id, Move::Jump);
    apply_plan(&image_map, &game_frame_1, &plan_1).expect_err("Completed infinite loop");
}
#[test]
fn test_two_jumps() {
    let image_map = ImageMap::mock();
    let mut game_frame_0 = GameFrame::new();
    let player_0_id = game_frame_0
        .insert_player(&image_map, Point2::new(0, 0))
        .expect("Error insterting player");
    let mut plan_0 = Plan::new();
    plan_0.portals.insert(Point2::new(1, 0));
    plan_0.portals.insert(Point2::new(2, 0));
    let game_frame_1 =
        apply_plan(&image_map, &game_frame_0, &plan_0).expect("Couldn't create portals");
    let player_1_id =
        player_at(&game_frame_1.ecs, Point2::new(1, 0)).expect("Couldn't find a player at (1,0)");
    let player_2_id =
        player_at(&game_frame_1.ecs, Point2::new(2, 0)).expect("Couldn't find a player at (2,0)");
    let mut plan_1 = Plan::new();
    plan_1
        .moves
        .insert(player_0_id, Move::Direction(Direction::Right));
    plan_1
        .moves
        .insert(player_1_id, Move::Direction(Direction::Right));
    plan_1
        .moves
        .insert(player_2_id, Move::Direction(Direction::Right));
    let game_frame_2 =
        apply_plan(&image_map, &game_frame_1, &plan_1).expect("Couldn't move right.");
    let mut plan_2 = Plan::new();
    plan_2.moves.insert(player_1_id, Move::Jump);
    let game_frame_3 =
        apply_plan(&image_map, &game_frame_2, &plan_2).expect("Couldn't perform first jump.");
    let mut plan_3 = Plan::new();
    plan_3.moves.insert(player_0_id, Move::Jump);
    apply_plan(&image_map, &game_frame_3, &plan_3).expect("Couldn't perform second jump.");
}
