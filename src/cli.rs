use std::ffi::OsString;

pub(crate) const HELP: &str = "keyinject — trusted local Linux input injection\n\nUsage:\n  keyinject server\n  keyinject input <key-or-button>\n  keyinject down <key-or-button>\n  keyinject up <key-or-button>\n  keyinject reset\n  keyinject available\n\navailable probes Linux support without creating input or socket state.\n";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    Server,
    Input(String),
    Down(String),
    Up(String),
    Reset,
    Available,
}

pub(crate) fn parse(args: &[OsString]) -> Result<Command, &'static str> {
    let words: Result<Vec<_>, _> = args
        .iter()
        .map(|arg| arg.to_str().ok_or("arguments must be valid UTF-8"))
        .collect();
    match words?.as_slice() {
        ["help"] => Ok(Command::Help),
        ["server"] => Ok(Command::Server),
        ["input", key] => Ok(Command::Input((*key).into())),
        ["down", key] => Ok(Command::Down((*key).into())),
        ["up", key] => Ok(Command::Up((*key).into())),
        ["reset"] => Ok(Command::Reset),
        ["available"] => Ok(Command::Available),
        _ => Err("invalid command"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(|value| (*value).into()).collect()
    }
    #[test]
    fn help_and_shapes_are_strict() {
        assert!(matches!(parse(&args(&["help"])), Ok(Command::Help)));
        assert!(matches!(
            parse(&args(&["available"])),
            Ok(Command::Available)
        ));
        assert!(parse(&args(&[])).is_err());
        assert!(parse(&args(&["help", "x"])).is_err());
        assert_eq!(parse(&args(&["server"])), Ok(Command::Server));
        assert_eq!(parse(&args(&["reset"])), Ok(Command::Reset));
        assert_eq!(
            parse(&args(&["input", "btn_left"])),
            Ok(Command::Input("btn_left".into()))
        );
        assert_eq!(parse(&args(&["down", "a"])), Ok(Command::Down("a".into())));
        assert_eq!(parse(&args(&["up", "f24"])), Ok(Command::Up("f24".into())));
        assert!(parse(&args(&["down"])).is_err());
        assert!(parse(&[OsString::from_vec(vec![0xff])]).is_err());
    }
}
