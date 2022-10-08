//! This crate provides common, functions for unit tests
use std::{
    env,
    fs::read_dir,
    io::{self, ErrorKind},
    path::PathBuf,
};

/// get the source project root path
pub fn get_project_root() -> io::Result<PathBuf> {
    let path = env::current_dir()?;

    for p in path.as_path().ancestors() {
        let has_cargo = read_dir(p)?
            .into_iter()
            .any(|p| p.unwrap().file_name().eq("Cargo.lock"));
        if has_cargo {
            return Ok(PathBuf::from(p));
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "Ran out of places to find Cargo.toml",
    ))
}

#[cfg(test)]
mod tests {
    use crate::get_project_root;

    #[test]
    fn test_service_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("test_units/config.service.toml");

        println!("{:?}", file_path);
        assert_eq!(file_path, file_path);
    }
}