use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs::{self, Metadata, Permissions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::Duration;

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode};

const USAGE: &str = "usage: keyinject server | keyinject input <key-or-button> | keyinject down <key-or-button> | keyinject up <key-or-button> | keyinject reset";
const DEVICE_NAME: &str = "keyinject virtual input";
const SOCKET_NAME: &str = "keyinject.sock";
const MAX_REQUEST: usize = 256;
const IO_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Request {
    Input(KeyCode),
    Down(KeyCode),
    Up(KeyCode),
    Reset,
}

fn key_code(name: &str) -> Option<KeyCode> {
    let upper = name.to_ascii_uppercase();
    let name = upper.strip_prefix("KEY_").unwrap_or(&upper);
    let code = match name {
        "A" => KeyCode::KEY_A,
        "B" => KeyCode::KEY_B,
        "C" => KeyCode::KEY_C,
        "D" => KeyCode::KEY_D,
        "E" => KeyCode::KEY_E,
        "F" => KeyCode::KEY_F,
        "G" => KeyCode::KEY_G,
        "H" => KeyCode::KEY_H,
        "I" => KeyCode::KEY_I,
        "J" => KeyCode::KEY_J,
        "K" => KeyCode::KEY_K,
        "L" => KeyCode::KEY_L,
        "M" => KeyCode::KEY_M,
        "N" => KeyCode::KEY_N,
        "O" => KeyCode::KEY_O,
        "P" => KeyCode::KEY_P,
        "Q" => KeyCode::KEY_Q,
        "R" => KeyCode::KEY_R,
        "S" => KeyCode::KEY_S,
        "T" => KeyCode::KEY_T,
        "U" => KeyCode::KEY_U,
        "V" => KeyCode::KEY_V,
        "W" => KeyCode::KEY_W,
        "X" => KeyCode::KEY_X,
        "Y" => KeyCode::KEY_Y,
        "Z" => KeyCode::KEY_Z,
        "0" => KeyCode::KEY_0,
        "1" => KeyCode::KEY_1,
        "2" => KeyCode::KEY_2,
        "3" => KeyCode::KEY_3,
        "4" => KeyCode::KEY_4,
        "5" => KeyCode::KEY_5,
        "6" => KeyCode::KEY_6,
        "7" => KeyCode::KEY_7,
        "8" => KeyCode::KEY_8,
        "9" => KeyCode::KEY_9,
        "ENTER" => KeyCode::KEY_ENTER,
        "ESC" | "ESCAPE" => KeyCode::KEY_ESC,
        "SPACE" => KeyCode::KEY_SPACE,
        "TAB" => KeyCode::KEY_TAB,
        "BACKSPACE" => KeyCode::KEY_BACKSPACE,
        "DELETE" => KeyCode::KEY_DELETE,
        "INSERT" => KeyCode::KEY_INSERT,
        "HOME" => KeyCode::KEY_HOME,
        "END" => KeyCode::KEY_END,
        "PAGEUP" => KeyCode::KEY_PAGEUP,
        "PAGEDOWN" => KeyCode::KEY_PAGEDOWN,
        "UP" => KeyCode::KEY_UP,
        "DOWN" => KeyCode::KEY_DOWN,
        "LEFT" => KeyCode::KEY_LEFT,
        "RIGHT" => KeyCode::KEY_RIGHT,
        "LEFTCTRL" => KeyCode::KEY_LEFTCTRL,
        "RIGHTCTRL" => KeyCode::KEY_RIGHTCTRL,
        "LEFTSHIFT" => KeyCode::KEY_LEFTSHIFT,
        "RIGHTSHIFT" => KeyCode::KEY_RIGHTSHIFT,
        "LEFTALT" => KeyCode::KEY_LEFTALT,
        "RIGHTALT" => KeyCode::KEY_RIGHTALT,
        "LEFTMETA" => KeyCode::KEY_LEFTMETA,
        "RIGHTMETA" => KeyCode::KEY_RIGHTMETA,
        "CTRL" => KeyCode::KEY_LEFTCTRL,
        "SHIFT" => KeyCode::KEY_LEFTSHIFT,
        "ALT" => KeyCode::KEY_LEFTALT,
        "META" | "SUPER" => KeyCode::KEY_LEFTMETA,
        "BTN_LEFT" | "MOUSELEFT" | "MOUSE-LEFT" | "MOUSE1" => KeyCode::BTN_LEFT,
        "BTN_RIGHT" | "MOUSERIGHT" | "MOUSE-RIGHT" | "MOUSE2" => KeyCode::BTN_RIGHT,
        "BTN_MIDDLE" | "MOUSEMIDDLE" | "MOUSE-MIDDLE" | "MOUSE3" => KeyCode::BTN_MIDDLE,
        "BTN_SIDE" | "MOUSESIDE" | "MOUSE4" => KeyCode::BTN_SIDE,
        "BTN_EXTRA" | "MOUSEEXTRA" | "MOUSE5" => KeyCode::BTN_EXTRA,
        "BTN_FORWARD" | "MOUSEFORWARD" => KeyCode::BTN_FORWARD,
        "BTN_BACK" | "MOUSEBACK" => KeyCode::BTN_BACK,
        _ => return f_key_code(name),
    };
    Some(code)
}

fn f_key_code(name: &str) -> Option<KeyCode> {
    let digits = name.strip_prefix('F')?;
    let number = digits.parse::<u8>().ok()?;
    if !(1..=24).contains(&number) || digits != number.to_string() {
        return None;
    }
    Some(
        [
            KeyCode::KEY_F1,
            KeyCode::KEY_F2,
            KeyCode::KEY_F3,
            KeyCode::KEY_F4,
            KeyCode::KEY_F5,
            KeyCode::KEY_F6,
            KeyCode::KEY_F7,
            KeyCode::KEY_F8,
            KeyCode::KEY_F9,
            KeyCode::KEY_F10,
            KeyCode::KEY_F11,
            KeyCode::KEY_F12,
            KeyCode::KEY_F13,
            KeyCode::KEY_F14,
            KeyCode::KEY_F15,
            KeyCode::KEY_F16,
            KeyCode::KEY_F17,
            KeyCode::KEY_F18,
            KeyCode::KEY_F19,
            KeyCode::KEY_F20,
            KeyCode::KEY_F21,
            KeyCode::KEY_F22,
            KeyCode::KEY_F23,
            KeyCode::KEY_F24,
        ][usize::from(number - 1)],
    )
}

fn parse_command(words: &[&str]) -> Result<Request, String> {
    match words {
        ["reset"] => Ok(Request::Reset),
        [command @ ("input" | "down" | "up"), name] => {
            let key = key_code(name).ok_or_else(|| format!("unknown key or button: {name}"))?;
            Ok(match *command {
                "input" => Request::Input(key),
                "down" => Request::Down(key),
                _ => Request::Up(key),
            })
        }
        _ => Err("expected server, input/down/up and one key or button, or reset".into()),
    }
}

fn parse_args(args: &[OsString]) -> Result<Option<Request>, String> {
    if args.len() == 1 && args[0] == "server" {
        return Ok(None);
    }
    let words: Result<Vec<_>, _> = args
        .iter()
        .map(|arg| arg.to_str().ok_or("arguments must be valid UTF-8"))
        .collect();
    parse_command(&words?).map(Some)
}

fn parse_wire(bytes: &[u8]) -> Result<Request, String> {
    if bytes.len() > MAX_REQUEST {
        return Err("request is too long".into());
    }
    let line = std::str::from_utf8(bytes).map_err(|_| "request must be valid UTF-8")?;
    let line = line
        .strip_suffix('\n')
        .ok_or("request must end with a newline")?;
    if line.contains('\n') || line.contains('\r') {
        return Err("request must contain one line".into());
    }
    parse_command(&line.split_ascii_whitespace().collect::<Vec<_>>())
}

fn supported_keys() -> AttributeSet<KeyCode> {
    let mut keys = AttributeSet::new();
    for name in [
        "a",
        "b",
        "c",
        "d",
        "e",
        "f",
        "g",
        "h",
        "i",
        "j",
        "k",
        "l",
        "m",
        "n",
        "o",
        "p",
        "q",
        "r",
        "s",
        "t",
        "u",
        "v",
        "w",
        "x",
        "y",
        "z",
        "0",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "f1",
        "f2",
        "f3",
        "f4",
        "f5",
        "f6",
        "f7",
        "f8",
        "f9",
        "f10",
        "f11",
        "f12",
        "f13",
        "f14",
        "f15",
        "f16",
        "f17",
        "f18",
        "f19",
        "f20",
        "f21",
        "f22",
        "f23",
        "f24",
        "enter",
        "escape",
        "space",
        "tab",
        "backspace",
        "delete",
        "insert",
        "home",
        "end",
        "pageup",
        "pagedown",
        "up",
        "down",
        "left",
        "right",
        "leftctrl",
        "rightctrl",
        "leftshift",
        "rightshift",
        "leftalt",
        "rightalt",
        "leftmeta",
        "rightmeta",
        "btn_left",
        "btn_right",
        "btn_middle",
        "btn_side",
        "btn_extra",
        "btn_forward",
        "btn_back",
    ] {
        keys.insert(key_code(name).expect("supported key name"));
    }
    keys
}

struct Held {
    keys: HashSet<KeyCode>,
}
impl Held {
    fn apply<F>(&mut self, request: Request, emit: &mut F) -> Result<(), String>
    where
        F: FnMut(KeyCode, i32) -> Result<(), String>,
    {
        match request {
            Request::Down(key) if self.keys.contains(&key) => Ok(()),
            Request::Down(key) => {
                emit(key, 1)?;
                self.keys.insert(key);
                Ok(())
            }
            Request::Up(key) if !self.keys.contains(&key) => Ok(()),
            Request::Up(key) => {
                emit(key, 0)?;
                self.keys.remove(&key);
                Ok(())
            }
            Request::Input(key) if self.keys.contains(&key) => {
                Err("cannot tap a held key or button; use up or reset first".into())
            }
            Request::Input(key) => {
                emit(key, 1)?;
                if let Err(error) = emit(key, 0) {
                    self.keys.insert(key);
                    return Err(format!("failed to release tap: {error}"));
                }
                Ok(())
            }
            Request::Reset => {
                let mut errors = Vec::new();
                for key in self.keys.clone() {
                    if let Err(error) = emit(key, 0) {
                        errors.push(error);
                    } else {
                        self.keys.remove(&key);
                    }
                }
                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(format!(
                        "failed to release {} held input(s): {}",
                        errors.len(),
                        errors.join("; ")
                    ))
                }
            }
        }
    }
}

fn uid() -> Result<u32, String> {
    fs::metadata("/proc/self")
        .map(|m| m.uid())
        .map_err(|e| format!("cannot determine current user: {e}"))
}
fn private_mode(mode: u32) -> bool {
    mode & 0o7777 == 0o700
}
fn private_dir(path: &Path, uid: u32) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("XDG_RUNTIME_DIR must be an absolute path".into());
    }
    let meta =
        fs::symlink_metadata(path).map_err(|e| format!("cannot inspect XDG_RUNTIME_DIR: {e}"))?;
    if meta.file_type().is_symlink()
        || !meta.is_dir()
        || meta.uid() != uid
        || !private_mode(meta.mode())
    {
        return Err(
            "XDG_RUNTIME_DIR must be an owned, non-symlink directory with mode 0700".into(),
        );
    }
    Ok(())
}
fn runtime_socket_path(runtime: Option<OsString>) -> Result<PathBuf, String> {
    let path = PathBuf::from(runtime.ok_or("XDG_RUNTIME_DIR is not set")?);
    private_dir(&path, uid()?)?;
    Ok(path.join(SOCKET_NAME))
}
fn socket_path() -> Result<PathBuf, String> {
    runtime_socket_path(env::var_os("XDG_RUNTIME_DIR"))
}
fn socket_mode(mode: u32) -> bool {
    mode & 0o7777 == 0o600
}
fn safe_socket(meta: &Metadata, uid: u32) -> bool {
    meta.file_type().is_socket() && meta.uid() == uid && socket_mode(meta.mode())
}
fn socket_meta(path: &Path, uid: u32) -> Result<Metadata, String> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| format!("cannot inspect {}: {e}", path.display()))?;
    if !safe_socket(&meta, uid) {
        return Err(format!(
            "{} is not an owned mode-0600 Unix socket",
            path.display()
        ));
    }
    Ok(meta)
}

struct SocketGuard {
    path: PathBuf,
    dev: u64,
    ino: u64,
    ctime: i64,
    ctime_nsec: i64,
    uid: u32,
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        if let Ok(meta) = fs::symlink_metadata(&self.path) {
            if safe_socket(&meta, self.uid)
                && meta.dev() == self.dev
                && meta.ino() == self.ino
                && meta.ctime() == self.ctime
                && meta.ctime_nsec() == self.ctime_nsec
            {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}

fn guard(path: &Path, meta: &Metadata, uid: u32) -> SocketGuard {
    SocketGuard {
        path: path.to_owned(),
        dev: meta.dev(),
        ino: meta.ino(),
        ctime: meta.ctime(),
        ctime_nsec: meta.ctime_nsec(),
        uid,
    }
}

fn bind_socket(path: &Path) -> Result<(UnixListener, SocketGuard), String> {
    let uid = uid()?;
    match fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(format!(
                "{} already exists; after verifying no keyinject server is running, remove the stale socket as the same user",
                path.display()
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("cannot inspect {}: {error}", path.display())),
    }
    let listener =
        UnixListener::bind(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    fs::set_permissions(path, Permissions::from_mode(0o600))
        .map_err(|e| format!("cannot secure {}: {e}", path.display()))?;
    let meta = socket_meta(path, uid)?;
    Ok((listener, guard(path, &meta, uid)))
}

fn read_request_with_timeout(
    stream: &mut UnixStream,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("cannot set read timeout: {e}"))?;
    let mut data = Vec::new();
    stream
        .take((MAX_REQUEST + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|e| format!("cannot read request: {e}"))?;
    if data.len() > MAX_REQUEST {
        return Err("request is too long".into());
    }
    Ok(data)
}
fn read_request(stream: &mut UnixStream) -> Result<Vec<u8>, String> {
    read_request_with_timeout(stream, IO_TIMEOUT)
}
fn reply(stream: &mut UnixStream, result: &Result<(), String>) -> Result<(), String> {
    let message = match result {
        Ok(()) => "OK\n".into(),
        Err(error) => format!("ERR {}\n", error.replace(['\n', '\r'], " ")),
    };
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|e| format!("cannot set write timeout: {e}"))?;
    stream
        .write_all(message.as_bytes())
        .map_err(|e| format!("cannot write response: {e}"))
}
fn handle_connection<F>(
    stream: &mut UnixStream,
    held: &mut Held,
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(KeyCode, i32) -> Result<(), String>,
{
    let result = read_request(stream)
        .and_then(|bytes| parse_wire(&bytes))
        .and_then(|request| held.apply(request, emit));
    reply(stream, &result)?;
    result
}

fn run_server() -> Result<(), String> {
    let path = socket_path()?;
    let (listener, _socket) = bind_socket(&path)?;
    let mut device = VirtualDevice::builder()
        .map_err(|e| setup_error("cannot open /dev/uinput", e))?
        .name(DEVICE_NAME)
        .with_keys(&supported_keys())
        .map_err(|e| setup_error("cannot configure virtual input", e))?
        .build()
        .map_err(|e| setup_error("cannot create virtual input", e))?;
    thread::sleep(Duration::from_secs(1));
    let mut held = Held {
        keys: HashSet::new(),
    };
    let result = loop {
        let (mut stream, _) = match listener.accept() {
            Ok(value) => value,
            Err(error) => break Err(format!("socket accept failed: {error}")),
        };
        let _ = handle_connection(&mut stream, &mut held, &mut |key, value| {
            device
                .emit(&[InputEvent::new(EventType::KEY.0, key.0, value)])
                .map_err(|e| format!("failed to emit input event: {e}"))
        });
    };
    let _ = held.apply(Request::Reset, &mut |key, value| {
        device
            .emit(&[InputEvent::new(EventType::KEY.0, key.0, value)])
            .map_err(|e| format!("failed to emit input event: {e}"))
    });
    result
}

fn run_client(line: String) -> Result<(), String> {
    let path = socket_path()?;
    socket_meta(&path, uid()?).map_err(|error| format!("{error}; start `keyinject server`"))?;
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        format!(
            "cannot connect to {} (start `keyinject server`): {e}",
            path.display()
        )
    })?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|e| format!("cannot set socket timeout: {e}"))?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|e| format!("cannot set socket timeout: {e}"))?;
    stream
        .write_all(line.as_bytes())
        .map_err(|e| format!("cannot send request: {e}"))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| format!("cannot finish request: {e}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("cannot read server response: {e}"))?;
    match response.strip_suffix('\n') {
        Some("OK") => Ok(()),
        Some(error) if error.starts_with("ERR ") => Err(error[4..].into()),
        _ => Err("invalid response from server".into()),
    }
}

fn setup_error(context: &str, error: io::Error) -> String {
    if error.kind() == io::ErrorKind::PermissionDenied {
        format!(
            "cannot access /dev/uinput; grant the trusted account read/write uinput access. On NixOS, enable hardware.uinput and add it to the uinput group, then start a fresh login. Do not run keyinject with sudo: {error}"
        )
    } else {
        format!("{context}: {error}")
    }
}

fn main() {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match parse_args(&args) {
        Ok(None) => {
            if let Err(error) = run_server() {
                eprintln!("keyinject: {error}");
                process::exit(1);
            }
        }
        Ok(Some(_)) => {
            let line = format!(
                "{}\n",
                args.iter()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            if let Err(error) = run_client(line) {
                eprintln!("keyinject: {error}");
                process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(|value| (*value).into()).collect()
    }
    fn held() -> Held {
        Held {
            keys: HashSet::new(),
        }
    }
    fn temp_dir() -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "keyinject-test-{}-{}",
            process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        fs::set_permissions(&dir, Permissions::from_mode(0o700)).unwrap();
        dir
    }

    #[test]
    fn permission_denied_guidance_uses_restricted_unprivileged_access() {
        let message = setup_error("ignored", io::Error::from(io::ErrorKind::PermissionDenied));
        assert!(message.contains("grant the trusted account read/write uinput access"));
        assert!(message.contains("hardware.uinput"));
        assert!(message.contains("uinput group"));
        assert!(message.contains("fresh login"));
        assert!(message.contains("Do not run keyinject with sudo"));
    }

    #[test]
    fn all_mappings_are_advertised() {
        let keys = supported_keys();
        for (name, key) in [
            ("a", KeyCode::KEY_A),
            ("b", KeyCode::KEY_B),
            ("c", KeyCode::KEY_C),
            ("d", KeyCode::KEY_D),
            ("e", KeyCode::KEY_E),
            ("f", KeyCode::KEY_F),
            ("g", KeyCode::KEY_G),
            ("h", KeyCode::KEY_H),
            ("i", KeyCode::KEY_I),
            ("j", KeyCode::KEY_J),
            ("k", KeyCode::KEY_K),
            ("l", KeyCode::KEY_L),
            ("m", KeyCode::KEY_M),
            ("n", KeyCode::KEY_N),
            ("o", KeyCode::KEY_O),
            ("p", KeyCode::KEY_P),
            ("q", KeyCode::KEY_Q),
            ("r", KeyCode::KEY_R),
            ("s", KeyCode::KEY_S),
            ("t", KeyCode::KEY_T),
            ("u", KeyCode::KEY_U),
            ("v", KeyCode::KEY_V),
            ("w", KeyCode::KEY_W),
            ("x", KeyCode::KEY_X),
            ("y", KeyCode::KEY_Y),
            ("z", KeyCode::KEY_Z),
            ("f1", KeyCode::KEY_F1),
            ("f24", KeyCode::KEY_F24),
            ("btn_left", KeyCode::BTN_LEFT),
            ("btn_right", KeyCode::BTN_RIGHT),
            ("btn_middle", KeyCode::BTN_MIDDLE),
            ("btn_side", KeyCode::BTN_SIDE),
            ("btn_extra", KeyCode::BTN_EXTRA),
            ("btn_forward", KeyCode::BTN_FORWARD),
            ("btn_back", KeyCode::BTN_BACK),
        ] {
            assert_eq!(key_code(name), Some(key), "{name}");
            assert!(keys.contains(key), "{name} is not advertised");
        }
        for name in [
            "mouseleft",
            "mouse-left",
            "mouse1",
            "mouseright",
            "mouse-right",
            "mouse2",
            "mousemiddle",
            "mouse-middle",
            "mouse3",
            "mouseside",
            "mouse4",
            "mouseextra",
            "mouse5",
            "mouseforward",
            "mouseback",
            "KEY_A",
        ] {
            assert!(
                keys.contains(key_code(name).unwrap()),
                "{name} is not advertised"
            );
        }
        for name in ["f0", "f25", "f01", "ctrl+a", "mouse6", "btn_foo"] {
            assert_eq!(key_code(name), None, "{name}");
        }
    }

    #[test]
    fn parser_accepts_aliases_and_rejects_malformed_input() {
        assert_eq!(parse_args(&args(&["server"])), Ok(None));
        assert_eq!(
            parse_args(&args(&["down", "a"])),
            Ok(Some(Request::Down(KeyCode::KEY_A)))
        );
        for name in [
            "mouseleft",
            "mouse-left",
            "mouse1",
            "mouseright",
            "mouse-right",
            "mouse2",
            "mousemiddle",
            "mouse-middle",
            "mouse3",
            "mouseside",
            "mouse4",
            "mouseextra",
            "mouse5",
            "mouseforward",
            "mouseback",
            "BTN_LEFT",
            "BTN_RIGHT",
            "BTN_MIDDLE",
            "BTN_SIDE",
            "BTN_EXTRA",
            "BTN_FORWARD",
            "BTN_BACK",
        ] {
            assert!(
                parse_wire(format!("input {name}\n").as_bytes()).is_ok(),
                "{name}"
            );
        }
        for input in [
            b"input a".as_slice(),
            b"input a\nup b\n",
            b"input a extra\n",
            b"server\n",
            b"input #30\n",
            b"\xff\n",
            b"input a\r\n",
        ] {
            assert!(parse_wire(input).is_err());
        }
        assert!(parse_wire(&vec![b'a'; MAX_REQUEST + 1]).is_err());
        assert!(parse_args(&args(&["input", "a", "extra"])).is_err());
        assert!(parse_args(&[OsString::from_vec(vec![0xff])]).is_err());
    }

    #[test]
    fn state_is_idempotent_and_recovers_failed_releases() {
        let mut state = held();
        let mut events = Vec::new();
        state
            .apply(Request::Down(KeyCode::KEY_A), &mut |k, v| {
                events.push((k, v));
                Ok(())
            })
            .unwrap();
        state
            .apply(Request::Down(KeyCode::KEY_A), &mut |k, v| {
                events.push((k, v));
                Ok(())
            })
            .unwrap();
        assert!(state
            .apply(Request::Input(KeyCode::KEY_A), &mut |_, _| Ok(()))
            .is_err());
        state
            .apply(Request::Up(KeyCode::KEY_A), &mut |k, v| {
                events.push((k, v));
                Ok(())
            })
            .unwrap();
        state
            .apply(Request::Up(KeyCode::KEY_A), &mut |k, v| {
                events.push((k, v));
                Ok(())
            })
            .unwrap();
        assert_eq!(events, vec![(KeyCode::KEY_A, 1), (KeyCode::KEY_A, 0)]);
        let mut calls = 0;
        assert!(state
            .apply(Request::Input(KeyCode::KEY_B), &mut |_, _| {
                calls += 1;
                if calls == 2 {
                    Err("no".into())
                } else {
                    Ok(())
                }
            })
            .is_err());
        assert!(state.keys.contains(&KeyCode::KEY_B));
        assert!(state
            .apply(Request::Reset, &mut |_, _| Err("still no".into()))
            .is_err());
        assert!(state.keys.contains(&KeyCode::KEY_B));
        state.apply(Request::Reset, &mut |_, _| Ok(())).unwrap();
        assert!(state.keys.is_empty());
        assert!(state
            .apply(Request::Input(KeyCode::KEY_C), &mut |_, _| Err(
                "press failed".into()
            ))
            .is_err());
        assert!(!state.keys.contains(&KeyCode::KEY_C));
    }

    #[test]
    fn runtime_path_requires_present_absolute_private_directory() {
        let dir = temp_dir();
        let owner = uid().unwrap();
        assert!(runtime_socket_path(None).is_err());
        assert!(runtime_socket_path(Some("relative".into())).is_err());
        assert!(runtime_socket_path(Some(dir.clone().into_os_string())).is_ok());
        for mode in [0o4700, 0o2700, 0o1700, 0o755] {
            assert!(!private_mode(mode), "{mode:o}");
        }
        fs::set_permissions(&dir, Permissions::from_mode(0o755)).unwrap();
        assert!(private_dir(&dir, owner).is_err());
        fs::set_permissions(&dir, Permissions::from_mode(0o700)).unwrap();
        let link = dir.with_extension("link");
        std::os::unix::fs::symlink(&dir, &link).unwrap();
        assert!(private_dir(&link, owner).is_err());
        fs::remove_file(link).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn exact_modes_reject_special_bits() {
        let dir = temp_dir();
        let path = dir.join(SOCKET_NAME);
        let listener = UnixListener::bind(&path).unwrap();
        let owner = uid().unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o600)).unwrap();
        assert!(socket_meta(&path, owner).is_ok());
        for mode in [0o4600, 0o2600, 0o1600, 0o666] {
            assert!(!socket_mode(mode), "{mode:o}");
        }
        fs::set_permissions(&path, Permissions::from_mode(0o666)).unwrap();
        assert!(socket_meta(&path, owner).is_err());
        drop(listener);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn startup_preserves_every_existing_socket_path() {
        let dir = temp_dir();
        let path = dir.join(SOCKET_NAME);
        fs::write(&path, "keep").unwrap();
        assert!(bind_socket(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "keep");
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("missing", &path).unwrap();
        assert!(bind_socket(&path).is_err());
        assert!(fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
        fs::remove_file(&path).unwrap();
        let active = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o600)).unwrap();
        let active_ino = fs::symlink_metadata(&path).unwrap().ino();
        assert!(bind_socket(&path).is_err());
        assert_eq!(fs::symlink_metadata(&path).unwrap().ino(), active_ino);
        drop(active);
        assert!(bind_socket(&path).is_err());
        assert_eq!(fs::symlink_metadata(&path).unwrap().ino(), active_ino);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn socket_guard_removes_only_its_original_socket() {
        let dir = temp_dir();
        let path = dir.join(SOCKET_NAME);
        let (listener, guard) = bind_socket(&path).unwrap();
        drop(listener);
        drop(guard);
        assert!(!path.exists());
        let (listener, guard) = bind_socket(&path).unwrap();
        drop(listener);
        fs::remove_file(&path).unwrap();
        let replacement = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o600)).unwrap();
        drop(guard);
        assert!(path.exists());
        drop(replacement);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn transport_limits_and_recovers_without_uinput() {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        client.write_all(&vec![b'a'; MAX_REQUEST + 1]).unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        assert!(read_request(&mut server).is_err());
        let (_client, mut server) = UnixStream::pair().unwrap();
        assert!(read_request_with_timeout(&mut server, Duration::from_millis(1)).is_err());
        let (mut client, mut server) = UnixStream::pair().unwrap();
        client.write_all(b"input a extra\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        let mut state = held();
        assert!(handle_connection(&mut server, &mut state, &mut |_, _| Ok(())).is_err());
        let mut response = [0; 256];
        let response_len = client.read(&mut response).unwrap();
        assert!(std::str::from_utf8(&response[..response_len])
            .unwrap()
            .starts_with("ERR "));
        let (mut client, mut server) = UnixStream::pair().unwrap();
        client.write_all(b"down a\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        handle_connection(&mut server, &mut state, &mut |_, _| Ok(())).unwrap();
        assert_eq!(client.read(&mut response).unwrap(), 3);
        assert_eq!(&response[..3], b"OK\n");
        assert!(state.keys.contains(&KeyCode::KEY_A));
    }

    #[test]
    fn response_loss_does_not_rollback_applied_state() {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        client.write_all(b"down a\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        drop(client);
        let mut state = held();
        assert!(handle_connection(&mut server, &mut state, &mut |_, _| Ok(())).is_err());
        assert!(state.keys.contains(&KeyCode::KEY_A));
    }
}
