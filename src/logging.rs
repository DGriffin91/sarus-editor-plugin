use crate::preset_manager::setup_dirs;

pub fn init_logging(file_name: &str) {
    if let Ok(project_paths) = setup_dirs() {
        let log_folder = project_paths.application_path;

        let _ = ::std::fs::create_dir(log_folder.clone());

        let log_file = ::std::fs::File::create(log_folder.join(file_name)).unwrap();

        let log_config = ::simplelog::ConfigBuilder::new()
            .set_time_to_local(true)
            .build();

        let _ = ::simplelog::WriteLogger::init(simplelog::LevelFilter::Info, log_config, log_file);

        ::log_panics::init();

        ::log::info!("init");
    }
}
