fn main() {
    let pack = quake_pack::Pack::new("resources/").unwrap();
    pack.file_names().for_each(|name| println!("{}", name));
}
