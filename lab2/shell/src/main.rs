use nix::sys::signal;
use nix::sys::signal::{SigHandler, Signal};
use nix::sys::wait::{wait, waitpid};
use nix::unistd::{close, dup, dup2, fork, getpgrp, getpid, pipe, setpgid, ForkResult, Pid};
use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::io::{stdin, BufRead};
use std::os::unix::io::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};

static mut MAIN_PID: i32 = 0;

extern "C" fn handle_sigint(_: i32) {
    let pid: i32 = getpid().into();
    unsafe {
        if pid != MAIN_PID {
            exit(0);
        } else {
            print!(
                "\n{} $ ",
                env::current_dir()
                    .expect("Error when getting cwd")
                    .to_str()
                    .expect("Error when converting cwd to string")
            );
            io::stdout().flush().expect("Prompt output error");
        }
    }
}
fn main() -> ! {
    let sig_action = signal::SigAction::new(
        SigHandler::Handler(handle_sigint),
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );
    unsafe {
        signal::sigaction(signal::SIGINT, &sig_action).expect("Error");
    }

    unsafe {
        MAIN_PID = getpid().into();
    }
    loop {
        // Prompt
        print!(
            "{} $ ",
            env::current_dir()
                .expect("Error when getting cwd")
                .to_str()
                .expect("Error when converting cwd to string")
        );
        io::stdout().flush().expect("Prompt output error");

        // parse
        let mut buf = Vec::new();
        let input;
        match stdin().lock().read_until(10, &mut buf) {
            Ok(0) => exit(0),
            Ok(1) => continue,
            Ok(_) => {
                buf.pop();
                input = std::str::from_utf8(&buf).expect("Invalid UTF-8 sequence");
            }
            _ => panic!("Error occurred when reading"),
        };
        let cmds: Vec<&str> = input.split('|').collect();
        let cmds_num = cmds.len();

        if cmds_num == 1 {
            // No pipe
            let mut cmd = cmds[0];
            let stdout_copy = dup(1).expect("Failed to fetch stdout fd");
            let stdin_copy = dup(0).expect("Failed to fetch stdin fd");
            if let (_, Some(file)) = get_token_after(cmd, ">>") {
                redirection(
                    IO_Select::output,
                    IO_redirection::file(file, RedirectionMode::append),
                );
            } else if let (_, Some(file)) = get_token_after(cmd, ">") {
                redirection(
                    IO_Select::output,
                    IO_redirection::file(file, RedirectionMode::write),
                );
            }

            if let (_, Some(file)) = get_token_after(cmd, "<<") {
                let delimiter = file;
                let mut buf = Vec::new();
                let mut input = String::new();
                let tmp_file_path = String::from("/tmp/ryan_shell_redirection_tmp");
                let mut f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(&tmp_file_path)
                    .expect("Failed to open file");
                loop {
                    buf.clear();
                    print!("> ");
                    io::stdout().flush().expect("Prompt output error");
                    match stdin().lock().read_until(10, &mut buf) {
                        Ok(0) => exit(0),
                        Ok(_) => {
                            buf.pop();
                            if std::str::from_utf8(&buf).expect("Invalid UTF-8 sequence")
                                == delimiter
                            {
                                break;
                            }
                            writeln!(
                                &mut f,
                                "{}",
                                std::str::from_utf8(&buf).expect("Invalid UTF-8 sequence")
                            );
                        }
                        _ => panic!("Error occurred when reading"),
                    };
                }
                f.flush();
                redirection(
                    IO_Select::input,
                    IO_redirection::file(tmp_file_path, RedirectionMode::read),
                );
            } else if let (_, Some(file)) = get_token_after(cmd, "<") {
                redirection(
                    IO_Select::input,
                    IO_redirection::file(file, RedirectionMode::read),
                );
            }

            cmd = cmd
                .split(|ch| ch == '>' || ch == '<')
                .next()
                .expect("No command");

            let mut args = cmd.split_whitespace();
            let prog = args.next();

            if let Some(prog) = prog {
                execute_main(prog, args);
            } else {
                panic!("Not program input");
            };
            dup2(stdin_copy, 0);
            dup2(stdout_copy, 1);
        } else {
            // Pipe
            let mut last_pipefd = (0, 0);
            for (i, mut cmd) in cmds.into_iter().enumerate() {
                let mut pipefd = (0, 1);

                if i != cmds_num - 1 {
                    pipefd = pipe().expect("Error occurred when generating pipe");
                }
                let (mut in_redirection, mut out_redirection) =
                    (IO_redirection::default, IO_redirection::default);
                if i != cmds_num - 1 {
                    out_redirection = IO_redirection::pipe(pipefd);
                }
                if i != 0 {
                    in_redirection = IO_redirection::pipe(last_pipefd);
                }

                if let (_, Some(file)) = get_token_after(cmd, ">>") {
                    out_redirection = IO_redirection::file(file, RedirectionMode::append);
                } else if let (_, Some(file)) = get_token_after(cmd, ">") {
                    out_redirection = IO_redirection::file(file, RedirectionMode::write);
                }
                if let (_, Some(file)) = get_token_after(cmd, "<") {
                    in_redirection = IO_redirection::file(file, RedirectionMode::read);
                }

                cmd = &cmd
                    .split(|ch| ch == '>' || ch == '<')
                    .next()
                    .expect("No command");

                let mut args = cmd.split_whitespace();
                let prog = args.next();

                if let Some(prog) = prog {
                    match fork() {
                        Ok(ForkResult::Parent { child: pid }) => {
                            setpgid(pid, getpgrp()).expect("Error")
                        }
                        Ok(ForkResult::Child) => {
                            redirection(IO_Select::input, in_redirection);
                            redirection(IO_Select::output, out_redirection);
                            if !execute_builtin(prog, &mut args) {
                                eprintln!("{}", Command::new(prog).args(args).exec());
                            }
                            exit(0);
                        }
                        _ => eprintln!("Error occurred when spawning subprocess"),
                    }
                } else {
                    panic!("Not program input");
                };

                last_pipefd = pipefd;

                if i != cmds_num - 1 {
                    close(pipefd.1);
                }
            }

            while match wait() {
                Ok(_) => true,
                _ => false,
            } {}
        }
    }
}

enum IO_Select {
    input,
    output,
}
enum IO_redirection {
    pipe((i32, i32)),
    file(String, RedirectionMode),
    default,
}

enum RedirectionMode {
    append,
    write,
    read,
}

fn redirection(io_selct: IO_Select, io_redirection: IO_redirection) -> () {
    match io_redirection {
        IO_redirection::pipe(pipefd) => match io_selct {
            IO_Select::input => {
                close(pipefd.1);
                dup2(pipefd.0, 0).expect("error");
                close(pipefd.0);
            }
            IO_Select::output => {
                close(pipefd.0);
                dup2(pipefd.1, 1).expect("error");
                close(pipefd.1);
            }
        },
        IO_redirection::file(file, mode) => match mode {
            RedirectionMode::append => {
                let f = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file)
                    .expect("Failed to open file");
                let fd = f.as_raw_fd();
                dup2(fd, 1).expect("error");
                close(fd);
            }
            RedirectionMode::write => {
                let f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(file)
                    .expect("Failed to open file");
                let fd = f.as_raw_fd();
                dup2(fd, 1).expect("error");
                close(fd);
            }
            RedirectionMode::read => {
                let f = OpenOptions::new()
                    .read(true)
                    .open(file)
                    .expect("Failed to open file");
                let fd = f.as_raw_fd();
                dup2(fd, 0).expect("error");
                close(fd);
            }
        },
        IO_redirection::default => (),
    }
}

fn get_token_after(source: &str, delimiter: &str) -> (String, Option<String>) {
    let mut iter = source.split(delimiter);
    let cmd = iter.next().expect("No command");
    if let Some(rest) = iter.next() {
        let rest = rest.trim();
        if let Some(file) = rest.split(|ch| ch == '>' || ch == '<' || ch == ' ').next() {
            return (cmd.to_string(), Some(file.to_string()));
        } else {
            return (cmd.to_string(), Some(rest.to_string()));
        }
    } else {
        return (cmd.to_string(), None);
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
        match fork() {
            Ok(ForkResult::Parent { child: pid }) => (),
            Ok(ForkResult::Child) => {
                eprintln!("{}", Command::new(prog).args(args).exec());
                exit(0);
            }
            _ => eprintln!("Error occurred when spawning subprocess"),
        };
        while match wait() {
            Ok(_) => true,
            _ => false,
        } {}
    }
}
