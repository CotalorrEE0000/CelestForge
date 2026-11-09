use crate::command::{ChoiceOption, Command, Position, TransitionKind};
use serde_json::Value;
use std::{collections::HashMap, path::Path};

pub struct Script {
    pub commands: Vec<Command>,
    pub labels: HashMap<String, usize>,
}

impl Script {
    pub fn label_index(&self, name: &str) -> Option<usize> {
        self.labels.get(name).copied()
    }
}

pub struct Loader;
const FALLBACK_ENDING_TITLE: &str = "感谢游玩";
const FALLBACK_ENDING_DURATION_MS: u32 = 3000;

impl Loader {
    pub fn from_json_str(input: &str) -> Result<Script, String> {
        let v: Value = serde_json::from_str(input).map_err(|e| format!("JSON LOAD FAILED: {e}"))?;
        let arr = v
            .as_array()
            .ok_or_else(|| "THE SCRIPT MUST BE JSON ARRAYS!".to_string())?;

        let mut commands = Vec::with_capacity(arr.len());
        for (i, item) in arr.iter().enumerate() {
            let cmd =
                parse_command(item).map_err(|e| format!("FOUND COMMAND ERROR AT LINE {i}: {e}"))?;
            commands.push(cmd);
        }

        ensure_ending(&mut commands);
        let mut labels = HashMap::new();
        for (i, cmd) in commands.iter().enumerate() {
            if let Command::Label(name) = cmd {
                labels.insert(name.clone(), i);
            }
        }

        Ok(Script { commands, labels })
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Script, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("FAILED TO READ FILE: {e}"))?;
        Loader::from_json_str(&text)
    }
}

fn parse_command(v: &Value) -> Result<Command, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| "COMMAND MUST BE JSON!".to_string())?;
    let type_name = str_field(obj, "type")?;

    match type_name.as_str() {
        "Text" => Ok(Command::Text {
            speaker: opt_str_field(obj, "speaker")?,
            content: str_field(obj, "content")?,
        }),
        "Wait" => Ok(Command::Wait),
        "WaitMs" => Ok(Command::WaitMs(u32_field(obj, "ms")?)),
        "SetBg" => Ok(Command::SetBg {
            asset_id: str_field(obj, "asset_id")?,
        }),
        "ClearBg" => Ok(Command::ClearBg),
        "ShowSprite" => Ok(Command::ShowSprite {
            who: str_field(obj, "who")?,
            asset_id: str_field(obj, "asset_id")?,
            position: parse_position(obj)?,
        }),
        "MoveSprite" => Ok(Command::MoveSprite {
            who: str_field(obj, "who")?,
            x: f32_field(obj, "x")?,
            y: f32_field(obj, "y")?,
            duration_ms: u32_field(obj, "duration_ms")?,
        }),
        "HideSprite" => Ok(Command::HideSprite {
            who: str_field(obj, "who")?,
        }),
        "ClearSprite" => Ok(Command::ClearSprites),
        "PlayVoice" => Ok(Command::PlayVoice {
            who: str_field(obj, "who")?,
            asset_id: str_field(obj, "asset_id")?,
        }),
        "PlaySe" => Ok(Command::PlaySe {
            asset_id: str_field(obj, "asset_id")?,
        }),
        "StopSe" => Ok(Command::StopSe),
        "StopVoice" => Ok(Command::StopVoice),
        "PlayBgm" => Ok(Command::PlayBgm {
            asset_id: str_field(obj, "asset_id")?,
        }),
        "StopBgm" => Ok(Command::StopBgm),
        "Choice" => Ok(Command::Choice {
            options: parse_choices(obj)?,
        }),
        "Label" => Ok(Command::Label(str_field(obj, "name")?)),
        "Jump" => Ok(Command::Jump(str_field(obj, "dest")?)),
        "JumpIf" => Ok(Command::JumpIf {
            flag: str_field(obj, "flag")?,
            dest: str_field(obj, "dest")?,
        }),
        "SetFlag" => Ok(Command::SetFlag {
            flag: str_field(obj, "flag")?,
            value: bool_field(obj, "value")?,
        }),
        "ClearFlag" => Ok(Command::ClearFlag(str_field(obj, "name")?)),
        "SetVar" => Ok(Command::SetVar {
            name: str_field(obj, "name")?,
            value: i64_field(obj, "value")?,
        }),
        "Transition" => Ok(Command::Transition {
            kind: parse_transition_kind(obj)?,
            duration_ms: u32_field(obj, "duration_ms")?,
        }),
        "Log" => Ok(Command::Log(str_field(obj, "text")?)),
        "ChapterTitle" => Ok(Command::ChapterTitle {
            title: str_field(obj, "title")?,
            duration_ms: u32_field(obj, "duration_ms")?,
        }),
        "Movie" => Ok(Command::Movie {
            asset_id: str_field(obj, "asset_id")?,
        }),
        "End" => Ok(Command::End),
        other => Err(format!("UNKNOWN COMMAND TYPE: {other}")),
    }
}

fn parse_position(obj: &serde_json::Map<String, Value>) -> Result<Position, String> {
    match str_field(obj, "position")?.as_str() {
        "Left" => Ok(Position::Left),
        "Center" => Ok(Position::Center),
        "Right" => Ok(Position::Right),
        "Absolute" => Ok(Position::Absolute(
            f32_field(obj, "x")?,
            f32_field(obj, "y")?,
        )),
        other => Err(format!("UNKNOWN POSITION: {other}")),
    }
}

fn parse_transition_kind(obj: &serde_json::Map<String, Value>) -> Result<TransitionKind, String> {
    match str_field(obj, "kind")?.as_str() {
        "Fade" => Ok(TransitionKind::Fade),
        "SlideLeft" => Ok(TransitionKind::SlideLeft),
        "SlideRight" => Ok(TransitionKind::SlideRight),
        "None" => Ok(TransitionKind::None),
        other => Err(format!("UNKNOWN TransitionKind: {other}")),
    }
}

fn parse_choices(obj: &serde_json::Map<String, Value>) -> Result<Vec<ChoiceOption>, String> {
    let arr = obj
        .get("options")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "\'Choice\' NEED \'options\' ARRAYS".to_string())?;

    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let o = item
            .as_object()
            .ok_or_else(|| "THE CHOICE MUST BE OBJECT".to_string())?;
        out.push(ChoiceOption {
            text: str_field(o, "text")?,
            jump_to: str_field(o, "jump_to")?,
            condition: opt_str_field(o, "condition")?,
        });
    }
    Ok(out)
}

fn str_field(obj: &serde_json::Map<String, Value>, key: &str) -> Result<String, String> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("MISSING OR WRONG STRING KEY AT: {key}"))
}

fn opt_str_field(
    obj: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<String>, String> {
    match obj.get(key) {
        Some(v) => v
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| format!("KEY {key} SHOULD BE STRING")),
        None => Ok(None),
    }
}

fn u32_field(obj: &serde_json::Map<String, Value>, key: &str) -> Result<u32, String> {
    obj.get(key)
        .and_then(|v| v.as_u64())
        .map(|n| n as u32)
        .ok_or_else(|| format!("MISSING OR WRONG u32 KEY: {key}"))
}

fn f32_field(obj: &serde_json::Map<String, Value>, key: &str) -> Result<f32, String> {
    obj.get(key)
        .and_then(|v| v.as_f64())
        .map(|n| n as f32)
        .ok_or_else(|| format!("MISSING OR WRONG f32 KEY: {key}"))
}

fn i64_field(obj: &serde_json::Map<String, Value>, key: &str) -> Result<i64, String> {
    obj.get(key)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| format!("MISSING OR WRONG i64 KEY: {key}"))
}

fn bool_field(obj: &serde_json::Map<String, Value>, key: &str) -> Result<bool, String> {
    obj.get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| format!("MISSING OR WRONG bool KEY: {key}"))
}
fn fallback_ending_title() -> Command {
    Command::ChapterTitle {
        title: FALLBACK_ENDING_TITLE.to_string(),
        duration_ms: FALLBACK_ENDING_DURATION_MS,
    }
}

fn ensure_ending(commands: &mut Vec<Command>) {
    let Some(end_idx) = commands.iter().rposition(|c| matches!(c, Command::End)) else {
        eprintln!("[LOADER] SCRIPT LACK END, CELESTFORGE HAS ALREADY ADDED ENDING");
        commands.push(fallback_ending_title());
        commands.push(Command::End);
        return;
    };
    let tail = commands[..end_idx]
        .iter()
        .rev()
        .find(|c| !matches!(c, Command::Log(_) | Command::Label(_)));

    if !matches!(tail, Some(Command::ChapterTitle { .. })) {
        eprintln!("[LOADER] SCRIPT LACK OF ENDING, CELESTFORGE HAS ALREADY ADDED");
        commands.insert(end_idx, fallback_ending_title());
    }
}

#[cfg(test)]
mod ending_tests {
    use super::*;

    fn kinds(s: &Script) -> Vec<&'static str> {
        s.commands
            .iter()
            .map(|c| match c {
                Command::Text { .. } => "Text",
                Command::Log(_) => "Log",
                Command::Label(_) => "Label",
                Command::ChapterTitle { .. } => "Title",
                Command::End => "End",
                _ => "Other",
            })
            .collect()
    }

    #[test]
    fn empty_script_gets_ending() {
        let s = Loader::from_json_str("[]").unwrap();
        assert_eq!(kinds(&s), vec!["Title", "End"]);
    }

    #[test]
    fn missing_end_gets_title_and_end() {
        let s = Loader::from_json_str(r#"[{"type":"Text","content":"hi"}]"#).unwrap();
        assert_eq!(kinds(&s), vec!["Text", "Title", "End"]);
    }

    #[test]
    fn text_then_end_gets_title_inserted() {
        let s =
            Loader::from_json_str(r#"[{"type":"Text","content":"hi"},{"type":"End"}]"#).unwrap();
        assert_eq!(kinds(&s), vec!["Text", "Title", "End"]);
    }

    #[test]
    fn author_title_is_kept() {
        let s = Loader::from_json_str(
            r#"[{"type":"ChapterTitle","title":"my own ending","duration_ms":1000},{"type":"End"}]"#,
        )
        .unwrap();
        assert_eq!(kinds(&s), vec!["Title", "End"]);
        match &s.commands[0] {
            Command::ChapterTitle { title, .. } => assert_eq!(title, "my own ending"),
            _ => panic!("first should be ChapterTitle"),
        }
    }

    #[test]
    fn log_between_title_and_end_is_skipped() {
        let s = Loader::from_json_str(
            r#"[{"type":"ChapterTitle","title":"fin","duration_ms":1000},{"type":"Log","text":"x"},{"type":"End"}]"#,
        )
        .unwrap();
        assert_eq!(kinds(&s), vec!["Title", "Log", "End"]);
    }

    #[test]
    fn labels_still_point_to_right_index_after_insert() {
        let s = Loader::from_json_str(
            r#"[{"type":"Label","name":"start"},{"type":"Text","content":"hi"},{"type":"End"}]"#,
        )
        .unwrap();
        assert_eq!(s.label_index("start"), Some(0));
        assert_eq!(kinds(&s), vec!["Label", "Text", "Title", "End"]);
    }
}
