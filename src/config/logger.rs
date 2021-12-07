pub fn init(debug: bool) {
    if let Ok(_) = log4rs::init_file("etc/log4rs.yaml", Default::default()) {}
    if debug {
        log::set_max_level(log::LevelFilter::Debug);
    }
}
