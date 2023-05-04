use std::{fs::File, io::Read, path::PathBuf};

/// Returns the value of an option giving precedence of command line options
/// over environment variables, and file source over directly specifying the
/// value.
///
/// # Arguments
///
/// * `base_opt_name` - The base opt name, e.g., `db-password` implies
///   argument `db-password-file`, and environment variables `DB_PASSWORD` and
///   `DB_PASSWORD_FILE`.
/// * `path_val` - The value from a `PathBuf` argument that gives precedence of
///   command line option over the environment variable.
/// * `val` - The value from a `String` argument that gives precedence of
///   command line option over the environment variable.
pub fn get_opt_content(
    base_opt_name: &str,
    path_val: &Option<PathBuf>,
    val: &Option<String>,
) -> String {
    match &path_val {
        Some(path) => {
            let mut file = match File::open(path) {
                Err(why) => panic!(
                    "couldn't open {} for base opt {}: {}",
                    path.display(),
                    base_opt_name,
                    why
                ),
                Ok(file) => file,
            };
            let mut s = String::new();
            return match file.read_to_string(&mut s) {
                Err(why) => panic!(
                    "couldn't read {} for base opt {}: {}",
                    path.display(),
                    base_opt_name,
                    why
                ),
                Ok(_) => s,
            };
        }
        None => match val {
            Some(v) => v.to_owned(),
            None => panic!(
                "either option {}-file or {} needs to be set",
                base_opt_name, base_opt_name
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE_OPT_NAME: &str = "db-password";

    #[test]
    fn test_file_arg_priority() {
        let path_opt = Some(PathBuf::from("tests/welcome123.txt"));
        let val_opt = Some(String::from("welcome456"));

        let content = get_opt_content(BASE_OPT_NAME, &path_opt, &val_opt);
        assert_eq!(content, "welcome123");
    }

    #[test]
    #[should_panic]
    fn test_missing_file_arg() {
        let path_opt = Some(PathBuf::from("tests/welcome456.txt"));
        let val_opt = Some(String::from("welcome456"));

        get_opt_content(BASE_OPT_NAME, &path_opt, &val_opt);
    }

    #[test]
    fn test_cli_arg_priority() {
        let path_opt: Option<PathBuf> = None;
        let val_opt = Some(String::from("welcome456"));

        let content = get_opt_content(BASE_OPT_NAME, &path_opt, &val_opt);
        assert_eq!(content, "welcome456");
    }

    #[test]
    #[should_panic]
    fn test_no_arg_set() {
        let path_opt: Option<PathBuf> = None;
        let val_opt: Option<String> = None;

        get_opt_content(BASE_OPT_NAME, &path_opt, &val_opt);
    }
}
