// One server tick: count the bar down, then run the attempt at zero (API 17).
fn observe_block_entity_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut horizon_client::containers::ModBlockEntity,
) {
    crate::game::industrialforegoing_state::on_tick(game, pos, entity);
}
