use crate::world::PlayerId;

pub fn player_exists(entities: &legion::World, player_id: PlayerId) -> bool {
    use legion::query::IntoQuery;
    let mut query = <&PlayerId>::query();
    query.iter(entities).any(|id| *id == player_id)
}
