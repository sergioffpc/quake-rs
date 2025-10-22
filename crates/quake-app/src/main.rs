fn main() {
    let mut pack = quake_pack::Pack::new("resources/").unwrap();
    pack.file_names().for_each(|name| println!("{}", name));

    let start = pack.by_name::<quake_map::Bsp>("maps/start.bsp").unwrap();
    let player = pack
        .by_name::<quake_model::Mdl>("progs/player.mdl")
        .unwrap();
}
