#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Text {
        speaker: Option<String>,
        content: String,
    },
    Wait,
    WaitMs(u32),
    SetBg {
        asset_id: String,
    },
    ClearBg,
    ShowSprite {
        who: String,
        asset_id: String,
        position: Position,
    },
    MoveSprite {
        who: String,
        x: f32,
        y: f32,
        duration_ms: u32,
    },
    HideSprite {
        who: String,
    },
    ClearSprites,
    PlayVoice {
        who: String,
        asset_id: String,
    },
    PlaySe {
        asset_id: String,
    },
    StopSe,
    StopVoice,
    PlayBgm {
        asset_id: String,
    },
    StopBgm,
    Choice {
        options: Vec<ChoiceOption>,
    },
    Label(String),
    Jump(String),
    JumpIf {
        flag: String,
        dest: String,
    },
    SetFlag {
        flag: String,
        value: bool,
    },
    ClearFlag(String),
    SetVar {
        name: String,
        value: i64,
    },
    Transition {
        kind: TransitionKind,
        duration_ms: u32,
    },
    Log(String),
    ChapterTitle {
        title: String,
        duration_ms: u32,
    },
    Movie {
        asset_id: String,
    },
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    Left,
    Center,
    Right,
    Absolute(f32, f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransitionKind {
    Fade,
    SlideLeft,
    SlideRight,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceOption {
    pub text: String,
    pub jump_to: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VmWait {
    Running,
    Pending(Command),
    Click,
    Timer(u32),
    Choice(Vec<ChoiceOption>),
    Finished,
}

impl Command {
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            Command::Wait
                | Command::WaitMs(_)
                | Command::Choice { .. }
                | Command::Transition { .. }
                | Command::Movie { .. }
                | Command::ChapterTitle { .. }
        )
    }

    pub fn has_side_effect(&self) -> bool {
        !matches!(
            self,
            Command::Label(_)
                | Command::SetFlag { .. }
                | Command::ClearFlag(_)
                | Command::SetVar { .. }
                | Command::Jump(_)
                | Command::JumpIf { .. }
                | Command::Wait
                | Command::WaitMs(_)
                | Command::Log(_)
        )
    }
}
