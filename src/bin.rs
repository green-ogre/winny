use std::env;

use winny::prelude::trace;

fn main() {
    let path_to_lib = retrieve_path_to_lib().unwrap();
    winny::winny_engine::enter_platform(path_to_lib);
}

fn retrieve_path_to_lib() -> Result<String, ()> {
    let dir = env::current_dir().map_err(|_| ())?;
    let project_name = dir.to_str().ok_or(())?.split("/").last().ok_or(())?;

    #[cfg(debug_assertions)]
    let path_to_lib = format!("/target/debug/{}.dll", project_name);
    #[cfg(not(debug_assertions))]
    let path_to_lib = format!("/target/release/{}.dll", project_name);

    trace!("Path to lib: {:?}", path_to_lib);

    Ok(path_to_lib)
}
