use anyhow::{bail, Context, Result};
use secrecy::SecretString;
use std::{fs, path::PathBuf};

/// Returns the value of an option giving precedence of command line options
/// over environment variables, and file source over directly specifying the
/// value.
pub fn get_opt_secret(
    base_opt_name: &str,
    path: Option<PathBuf>,
    val: Option<SecretString>,
) -> Result<SecretString> {
    match (path, val) {
        (Some(_), Some(_)) => unreachable!("options should conflict"),
        (Some(path), None) => fs::read_to_string(&path)
            .with_context(|| format!("failed to read file `{path}`", path = path.display()))
            .map(Into::into),
        (None, Some(val)) => Ok(val),
        (None, None) => {
            bail!("either option `{base_opt_name}-file` or `{base_opt_name}` needs to be specified")
        }
    }
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;

    const BASE_OPT_NAME: &str = "db-password";

    #[test]
    fn test_missing_file_arg() {
        let path_opt = Some(PathBuf::from("tests/welcome456.txt"));
        let val_opt = None;

        assert_eq!(
            get_opt_secret(BASE_OPT_NAME, path_opt, val_opt)
                .unwrap_err()
                .to_string(),
            "failed to read file `tests/welcome456.txt`"
        );
    }

    #[test]
    fn test_cli_arg_priority() {
        let path_opt: Option<PathBuf> = None;
        let val_opt = Some(String::from("welcome456").into());

        let content = get_opt_secret(BASE_OPT_NAME, path_opt, val_opt).unwrap();
        assert_eq!(content.expose_secret(), "welcome456");
    }

    #[test]
    fn test_no_arg_set() {
        let path_opt: Option<PathBuf> = None;
        let val_opt: Option<SecretString> = None;

        assert_eq!(
            get_opt_secret(BASE_OPT_NAME, path_opt, val_opt)
                .unwrap_err()
                .to_string(),
            "either option `db-password-file` or `db-password` needs to be specified"
        );
    }
}
