// `/industrialforegoing info|energy|water|progress|drain` (API 8).
fn observe_register_commands(commands: &mut Vec<NativeCommand>) {
    crate::game::industrialforegoing_state::on_register_commands(commands);
}
