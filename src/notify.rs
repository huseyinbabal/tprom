use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Built-in sounds embedded in the binary (see `scripts/generate-sounds.py`)
pub const BUILTIN_SOUNDS: &[(&str, &[u8])] = &[
    ("chime", include_bytes!("../assets/sounds/chime.wav")),
    ("alarm", include_bytes!("../assets/sounds/alarm.wav")),
    ("siren", include_bytes!("../assets/sounds/siren.wav")),
    ("klaxon", include_bytes!("../assets/sounds/klaxon.wav")),
];

/// macOS keeps its system notification sound as the default
const MACOS_DEFAULT_SOUND: &str = "/System/Library/Sounds/Glass.aiff";

/// Play the configured alert sound `repeat` times without blocking the UI.
///
/// `sound` is `default` (Glass on macOS, `chime` elsewhere), one of
/// [`BUILTIN_SOUNDS`], a macOS system sound name (e.g. `Sosumi`), or a path
/// to an audio file. Anything that can't be played falls back to the terminal bell.
pub fn play_alert_sound(sound: &str, repeat: u32) {
    let file = resolve_sound(sound);
    std::thread::spawn(move || {
        for i in 0..repeat.clamp(1, 10) {
            if i > 0 {
                std::thread::sleep(Duration::from_millis(150));
            }
            if !file.as_deref().is_some_and(play_file_blocking) {
                ring_bell();
                std::thread::sleep(Duration::from_millis(300));
            }
        }
    });
}

/// Map a configured sound name to a playable file, if any
fn resolve_sound(sound: &str) -> Option<PathBuf> {
    let sound = sound.trim();
    if sound.is_empty() || sound.eq_ignore_ascii_case("default") {
        let glass = PathBuf::from(MACOS_DEFAULT_SOUND);
        if cfg!(target_os = "macos") && glass.exists() {
            return Some(glass);
        }
        return builtin_sound_file("chime");
    }

    let lower = sound.to_ascii_lowercase();
    if BUILTIN_SOUNDS.iter().any(|(name, _)| *name == lower) {
        return builtin_sound_file(&lower);
    }

    if sound.contains('/') || sound.contains('\\') || sound.starts_with('~') {
        let path = match sound.strip_prefix("~/") {
            Some(rest) => dirs::home_dir()?.join(rest),
            None => PathBuf::from(sound),
        };
        return path.exists().then_some(path);
    }

    // macOS system sound by name, e.g. "Sosumi" or "Basso"
    let system = PathBuf::from(format!("/System/Library/Sounds/{}.aiff", sound));
    if system.exists() {
        return Some(system);
    }
    resolve_sound("default")
}

/// Write the embedded WAV for `name` to the cache dir so a player can open it
fn builtin_sound_file(name: &str) -> Option<PathBuf> {
    let (_, bytes) = BUILTIN_SOUNDS.iter().find(|(n, _)| *n == name)?;
    let dir = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("tprom");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{}.wav", name));
    if std::fs::read(&path).ok().as_deref() != Some(*bytes) {
        std::fs::write(&path, bytes).ok()?;
    }
    Some(path)
}

/// Play an audio file with the first available system player, waiting for it
fn play_file_blocking(file: &Path) -> bool {
    let players: Vec<(&str, Vec<String>)> = if cfg!(target_os = "macos") {
        vec![("afplay", vec![])]
    } else if cfg!(windows) {
        let script = format!(
            "(New-Object Media.SoundPlayer '{}').PlaySync()",
            file.display().to_string().replace('\'', "''")
        );
        return run("powershell", &["-NoProfile", "-Command", &script]);
    } else {
        vec![
            ("paplay", vec![]),
            ("pw-play", vec![]),
            ("aplay", vec!["-q".to_string()]),
        ]
    };

    let file = file.display().to_string();
    players.iter().any(|(program, args)| {
        let mut all: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        all.push(&file);
        run(program, &all)
    })
}

fn run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn ring_bell() {
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\x07");
    let _ = stdout.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_sounds_are_wavs() {
        for (name, bytes) in BUILTIN_SOUNDS {
            assert_eq!(&bytes[0..4], b"RIFF", "{}", name);
            assert_eq!(&bytes[8..12], b"WAVE", "{}", name);
            let path = resolve_sound(name).expect("builtin sound path");
            assert_eq!(std::fs::read(path).unwrap(), *bytes);
        }
    }

    #[test]
    fn unknown_or_missing_sounds_fall_back() {
        assert!(resolve_sound("default").is_some());
        assert_eq!(resolve_sound("no-such-sound"), resolve_sound("default"));
        assert_eq!(resolve_sound("/no/such/file.wav"), None);
    }
}
