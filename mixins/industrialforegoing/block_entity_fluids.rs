// The machine's 16,000 mB water tank (API 54).
fn observe_block_entity_fluids(
    block: &str,
    fluids: &mut Option<horizon_client::containers::ModFluidTanks>,
) {
    crate::game::industrialforegoing_state::on_block_entity_fluids(block, fluids);
}
