use std::path::{Path, PathBuf};

const NET_TEXTFSM: &str = "NET_TEXTFSM";

/// The templates directory from the `NET_TEXTFSM` environment variable, as netmiko reads it.
pub fn ntc_templates_dir() -> Option<PathBuf> {
    std::env::var_os(NET_TEXTFSM).map(|dir| index_dir(Path::new(&dir)))
}

/// netmiko also accepts the directory above `templates/` when it holds no `index`.
fn index_dir(dir: &Path) -> PathBuf {
    if dir.join("index").is_file() { dir.to_path_buf() } else { dir.join("templates") }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/textfsm");

    #[test]
    fn keeps_a_directory_that_holds_the_index() {
        assert_eq!(index_dir(Path::new(FIXTURE_DIR)), PathBuf::from(FIXTURE_DIR));
    }

    #[test]
    fn descends_into_templates_when_the_index_is_missing() {
        let parent = Path::new(FIXTURE_DIR).parent().unwrap();
        assert_eq!(index_dir(parent), parent.join("templates"));
    }
}
