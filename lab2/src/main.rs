use nix::sys::wait::wait;
use nix::unistd::{close, dup2, fork, pipe, ForkResult};
use std::env;
use std::io::{self, Write};
use std::io::{stdin, BufRead};
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};

fn main() -> ! {
    loop {
        // Prompt
        print!(
            "{}$ ",
            env::current_dir()
                .expect("Error when getting cwd")
                .to_str()
                .expect("Error when converting cwd to string")
        );
        io::stdout().flush().expect("Prompt output error");

        //parse
        let input = stdin()
            .lock()
            .lines()
            .next()
            .expect("Nothing read")
            .expect("Read a line from stdin failed");
        let cmds: Vec<&str> = input.split('|').collect();

        if cmds.len() == 1 {
            // No pipe
            let mut args = cmds[0].split_whitespace();
            let prog = args.next();

            if let Some(prog) = prog {
                execute_main(prog, args);
            } else {
                panic!("Not program input");
            };
        } else {
            // Pipe
            let mut read_fd = 0;
            for (i, cmd) in cmds.iter().enumerate() {
                let mut args = cmd.split_whitespace();
                let prog = args.next();

                let mut pipefd = (0, 1);

                if i != cmd.len() - 1 {
                    pipefd = pipe().expect("Error occurred when generating pipe");
                }

                if let Some(prog) = prog {
                    match fork() {
                        Ok(ForkResult::Parent { child: _ }) => (),
                        Ok(ForkResult::Child) => {
                            // Output redirection
                            if i != cmds.len() - 1 {
                                close(pipefd.0);
                                dup2(pipefd.1, 1);
                                close(pipefd.1);
                            }

                            // Input redirection
                            if i != 0 {
                                dup2(read_fd, 0);
                                close(read_fd);
                            }
                            execute_subproc(prog, args);
                            exit(0);
                        }
                        _ => eprintln!("Error occurred when spawning subprocess"),
                    }
                } else {
                    panic!("Not program input");
                };
                read_fd = pipefd.0;
                close(pipefd.1);
            }
            while match wait() {
                Ok(_) => true,
                _ => false,
            } {}
        }
    }
}

fn execute_builtin(prog: &str, args: &mut std::str::SplitWhitespace<'_>) -> bool {
    match prog {
        "cd" => {
            let dir = args.next().expect("No enough args to set current dir");
            env::set_current_dir(dir).expect("Changing current dir failed");
            true
        }
        "pwd" => {
            let err = "Getting current dir failed";
            println!("{}", env::current_dir().expect(err).to_str().expect(err));
            true
        }
        "export" => {
            for arg in args {
                let mut assign = arg.split("=");
                let name = assign.next().expect("No variable name");
                let value = assign.next().expect("No variable value");
                env::set_var(name, value);
            }
            true
        }
        "exit" => {
            exit(0);
        }
        _ => false,
    }
}
fn execute_main(prog: &str, mut args: std::str::SplitWhitespace<'_>) {
    if !execute_builtin(prog, &mut args) {
        Command::new(prog)
            .args(args)
            .status()
            .expect("Run program failed");
    }
}

fn execute_subproc(prog: &str, mut args: std::str::SplitWhitespace<'_>) {
    if !execute_builtin(prog, &mut args) {
        eprintln!("{}", Command::new(prog).args(args).exec());
    }
}
