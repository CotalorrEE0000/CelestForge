use celest_core::command::VmWait;
use celest_core::loader::Loader;
use celest_core::vm::Vm;
use std::io::{self, Write};
use std::sync::Arc;

fn main() {
    println!("=== Celest Forge CLI Player ===\n");

    let script = Loader::from_file("scripts/test_chapter1.json")
        .expect("SCRIPT LOAD FAILED PLEASE CONFORM THAT scripts/test_chapter1.json IS EXIST");

    let script = Arc::new(script);
    let mut vm = Vm::new(Arc::clone(&script));

    println!(
        "ALL COMMAND IS ATTACHED, {} IS SUCCESSED \n",
        script.commands.len()
    );
    println!("PRESS ENTER TO CONTINUE...");

    loop {
        vm.step();
        match &vm.wait {
            VmWait::Finished => {
                println!("\n=== SCRIPT END ===");
                break;
            }
            VmWait::Pending(cmd) => {
                match cmd {
                    celest_core::command::Command::Text { speaker, content } => {
                        if let Some(name) = speaker {
                            println!("【{}】{}", name, content);
                        } else {
                            println!("{}", content);
                        }
                    }
                    celest_core::command::Command::SetBg { asset_id } => {
                        println!("[SCENE CHANGE] bg: {}", asset_id);
                    }
                    celest_core::command::Command::ClearBg => {
                        println!("[SCENE CHANGE] CLEAR BG");
                    }
                    celest_core::command::Command::ShowSprite {
                        who,
                        asset_id,
                        position,
                    } => {
                        println!(
                            "[SPRITE] {} (SPRITE: {}, LOCATION: {:?})",
                            who, asset_id, position
                        );
                    }
                    celest_core::command::Command::HideSprite { who } => {
                        println!("[SPRITE] HIDE {}", who);
                    }
                    celest_core::command::Command::ClearSprites => {
                        println!("[SPRITE] CLEAR ALL");
                    }
                    celest_core::command::Command::PlayVoice { who, asset_id } => {
                        println!("[VOICE] {}: {}", who, asset_id);
                    }
                    celest_core::command::Command::PlaySe { asset_id } => {
                        println!("[SOUNDEFFECT] {}", asset_id);
                    }
                    celest_core::command::Command::PlayBgm { asset_id } => {
                        println!("[BGM] PLAY: {}", asset_id);
                    }
                    celest_core::command::Command::StopBgm => {
                        println!("[BGM] STOP");
                    }
                    celest_core::command::Command::ChapterTitle { title, duration_ms } => {
                        println!("\n========== {} ==========", title);
                        println!("(CONTENT WAIT {}ms)", duration_ms);
                    }
                    celest_core::command::Command::Log(text) => {
                        println!("[LOG] {}", text);
                    }
                    _ => {
                        print!("");
                    }
                }
                vm.resume();
            }
            VmWait::Click => {
                print!("> ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                vm.resume();
            }
            VmWait::Timer(ms) => {
                println!("(WAIT {}ms...)", ms);
                std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
                vm.resume();
            }
            VmWait::Choice(options) => {
                println!("\nPLEASE CHOICE：");
                for (i, opt) in options.iter().enumerate() {
                    println!("  {}. {}", i + 1, opt.text);
                }
                print!("> ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                let choice: usize = input.trim().parse().unwrap_or(1) - 1;
                let jump_to = &options.get(choice).unwrap_or(&options[0]).jump_to.clone();
                vm.resume_with_choice(&jump_to);
            }
            VmWait::Running => {
                continue;
            }
        }
    }
}
