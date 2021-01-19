use std::env;
use std::io::{stdout, stdin, Write};
use std::process::{Child, Command, Stdio};
use std::path::Path;

use ansi_term::Colour::Red;
use shellexpand::tilde_with_context;
use dirs_next::home_dir;

pub struct Prompt {
    theme: String
}

impl Prompt {
    pub fn run(&mut self) {
        loop {
            if let Err(error) = self.refresh_screen() {
                die(error);
            }
            if self.should_quit {
                break;
            }
            if let Err(error) = self.process_keypress() {
                die(error);
            }
        }
    }

    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut initial_status =
            String::from("HELP: Ctrl-F = find | Ctrl-S = save | Ctrl-Q = quit");

        let document = if let Some(file_name) = args.get(1) {
            let doc = Document::open(file_name);
            if let Ok(doc) = doc {
                doc
            } else {
                initial_status = format!("ERR: Could not open file: {}", file_name);
                Document::default()
            }
        } else {
            Document::default()
        };

        Self {
            theme: "default".to_string(),
        }
    }

    fn refresh_screen(&mut self) -> Result<(), std::io::Error> {
        let current_dir = env::current_dir().unwrap().into_os_string().into_string().unwrap();
        let home_dir_string = home_dir().unwrap().into_os_string().into_string().unwrap();
        println!("{}", home_dir_string);

        let current_tilde_dir = current_dir.replace(home_dir_string.as_str(), "~");
        let prompt_dir = Red.paint(current_tilde_dir);
        println!("{}", prompt_dir);

        let prompt_arrow = Red.paint("❯").to_string();
        print!("{} ", prompt_arrow);
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        // must be peekable so we know when we are on the last command
        let mut commands = input.trim().split(" | ").peekable();
        let mut previous_command = None;

        while let Some(command) = commands.next()  {

            let mut parts = command.trim().split_whitespace();
            let command = parts.next().unwrap();
            let args = parts;

            match command {
                "cd" => {
                    let new_dir = args.peekable().peek()
                        .map_or("/", |x| *x);
                    let new_tilde_dir = tilde_with_context(new_dir, home_dir);
                    let root = Path::new(new_tilde_dir.as_ref());

                    if let Err(e) = env::set_current_dir(&root) {
                        eprintln!("{}", e);
                    }

                    previous_command = None;
                },
                "exit" => return,
                command => {
                    let stdin = previous_command
                        .map_or(
                            Stdio::inherit(),
                            |output: Child| Stdio::from(output.stdout.unwrap())
                        );

                    let stdout = if commands.peek().is_some() {
                        // there is another command piped behind this one
                        // prepare to send output to the next command
                        Stdio::piped()
                    } else {
                        // there are no more commands piped behind this one
                        // send output to shell stdout
                        Stdio::inherit()
                    };

                    let output = Command::new(command)
                        .args(args)
                        .stdin(stdin)
                        .stdout(stdout)
                        .spawn();

                    match output {
                        Ok(output) => { previous_command = Some(output); },
                        Err(e) => {
                            previous_command = None;
                            eprintln!("{}", e);
                        },
                    };
                }
            }
        }

        if let Some(mut final_command) = previous_command {
            // block until the final command has finished
            final_command.wait().unwrap();
        }

    }
}

fn die(e: std::io::Error) {
    panic!(e);
}
