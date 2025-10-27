fn main() {
    let mut resources = quake_resource::Resources::new("resources/").unwrap();
    let quake_rc = resources.by_name::<String>("quake.rc").unwrap();
    println!("quake.rc: {:?}", quake_rc);
}
