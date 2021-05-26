use nix::sys::signal;
use nix::sys::signal::SigHandler;
use nix::sys::wait::wait;
use nix::unistd::{close, dup, dup2, fork, getpgrp, getpid, pipe, setpgid, ForkResult};
use std::env;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::io::{stdin, BufRead};
use std::net::{Shutdown, TcpStream};
use std::os::unix::io::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{exit, Command};
use std::str::FromStr;

/// Used to handle signal SIGINT.  
/// `MAIN_PID` is the pid of shell process.
/// When receiving signal SIGINT, the process terminates
/// if it is a child process.  For the parent process,
/// i.e. the shell, it continues with a prompt
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
    // Register for the ctrl+C signal
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

        // Read and parse the command
        let mut is_fd_directed = false;
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

        // No pipe
        if cmds_num == 1 {
            let mut cmd = cmds[0];
            let stdout_copy = dup(1).expect("Failed to fetch stdout fd");
            let stdin_copy = dup(0).expect("Failed to fetch stdin fd");
            let mut stream;

            // Check the IO redirection
            if let (_, Some(file)) = get_token_after(cmd, ">>") {
                redirection(
                    IOSelect::Output,
                    IORedirection::file(file, RedirectionMode::append),
                );
            } else if let (_, Some(fd)) = get_token_after(cmd, ">&") {
                let fd = i32::from_str(&fd).expect("Invalid file descriptor");
                match get_fd_before(cmd, ">&") {
                    None => redirection(IOSelect::Output, IORedirection::fd(fd)),
                    Some(src_fd) => {
                        redirection(IOSelect::out_fd(src_fd), IORedirection::fd(fd));
                        is_fd_directed = true
                    }
                };
            } else if let (_, Some(file)) = get_token_after(cmd, ">") {
                match get_fd_before(cmd, ">") {
                    None => match tcp_handler(&file, IOSelect::Output) {
                        None => redirection(
                            IOSelect::Output,
                            IORedirection::file(file, RedirectionMode::write),
                        ),
                        Some(s) => stream = s,
                    },
                    Some(fd) => {
                        redirection(
                            IOSelect::out_fd(fd),
                            IORedirection::file(file, RedirectionMode::write),
                        );
                        is_fd_directed = true;
                    }
                };
            }

            if let (_, Some(file)) = get_token_after(cmd, "<<") {
                let delimiter = file;
                let mut buf = Vec::new();
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
                            )
                            .expect("Failed to write to tmp file");
                        }
                        _ => panic!("Error occurred when reading"),
                    };
                }
                f.flush().expect("Failed to write to tmp file");
                redirection(
                    IOSelect::Input,
                    IORedirection::file(tmp_file_path, RedirectionMode::read),
                );
            } else if let (_, Some(file)) = get_token_after(cmd, "<") {
                match get_fd_before(cmd, "<") {
                    None => match tcp_handler(&file, IOSelect::Input) {
                        None => redirection(
                            IOSelect::Input,
                            IORedirection::file(file, RedirectionMode::read),
                        ),
                        Some(s) => stream = s,
                    },
                    Some(src_fd) => {
                        redirection(
                            IOSelect::in_fd(src_fd),
                            IORedirection::file(file, RedirectionMode::read),
                        );
                        is_fd_directed = true;
                    }
                };
            }

            // Remove the redirection part in `cmd`
            cmd = cmd
                .split(|ch| ch == '>' || ch == '<')
                .next()
                .expect("No command");
            if is_fd_directed == true {
                cmd = cmd.trim_end_matches(|ch: char| ch == ' ' || ch.is_digit(10));
            }

            let mut args = cmd.split_whitespace();
            let prog = args.next();

            // Execute
            if let Some(prog) = prog {
                execute_main(prog, args);
            } else {
                panic!("Not program input");
            };

            // stream
            //     .shutdown(Shutdown::Both)
            //     .expect("shutdown call failed");

            // Restore stdin, stdout
            dup2(stdin_copy, 0).expect("error");
            dup2(stdout_copy, 1).expect("error");
        } else {
            // Pipe
            let mut last_pipefd = (0, 0);

            for (i, mut cmd) in cmds.into_iter().enumerate() {
                let mut pipefd = (0, 1);

                // Record connection between processes through pipe
                if i != cmds_num - 1 {
                    pipefd = pipe().expect("Error occurred when generating pipe");
                }
                let (mut in_redirection, mut out_redirection) =
                    (IORedirection::default, IORedirection::default);
                if i != cmds_num - 1 {
                    out_redirection = IORedirection::pipe(pipefd);
                }
                if i != 0 {
                    in_redirection = IORedirection::pipe(last_pipefd);
                }

                // Check the IO redirection
                if let (_, Some(file)) = get_token_after(cmd, ">>") {
                    out_redirection = IORedirection::file(file, RedirectionMode::append);
                } else if let (_, Some(file)) = get_token_after(cmd, ">") {
                    out_redirection = IORedirection::file(file, RedirectionMode::write);
                }
                if let (_, Some(file)) = get_token_after(cmd, "<") {
                    in_redirection = IORedirection::file(file, RedirectionMode::read);
                }

                cmd = &cmd
                    .split(|ch| ch == '>' || ch == '<')
                    .next()
                    .expect("No command");

                let mut args = cmd.split_whitespace();
                let prog = args.next();

                // Execute with child process
                if let Some(prog) = prog {
                    match fork() {
                        Ok(ForkResult::Parent { child: pid }) => {
                            setpgid(pid, getpgrp()).expect("Error")
                        }
                        Ok(ForkResult::Child) => {
                            // Execute in child process
                            redirection(IOSelect::Input, in_redirection);
                            redirection(IOSelect::Output, out_redirection);
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
                    close(pipefd.1).expect("Failed to redirect IO");
                }
            }

            while match wait() {
                Ok(_) => true,
                _ => false,
            } {}
        }
    }
}

enum IOSelect {
    Input,
    Output,
    in_fd(i32),
    out_fd(i32),
}
enum IORedirection {
    pipe((i32, i32)),
    file(String, RedirectionMode),
    fd(i32),
    default,
}

enum RedirectionMode {
    append,
    write,
    read,
}

fn tcp_handler(file_path: &str, io_mode: IOSelect) -> Option<TcpStream> {
    // TCP parser
    let path: Vec<_> = file_path.split('/').collect();
    if path.len() != 5 || path[0] != "" || path[1] != "dev" || path[2] != "tcp" {
        return None;
    } else {
        if let Ok(stream) =
            TcpStream::connect(path[path.len() - 2].to_string() + ":" + path[path.len() - 1])
        {
            let raw_fd = stream.as_raw_fd();
            match io_mode {
                IOSelect::Input => {
                    dup2(raw_fd, 0).expect("error");
                    close(raw_fd).expect("Failed to redirect IO");
                    Some(stream)
                }
                IOSelect::Output => {
                    dup2(raw_fd, 1).expect("error");
                    close(raw_fd).expect("Failed to redirect IO");
                    Some(stream)
                }
                _ => panic!("Failed to redirect IO"),
            }
        } else {
            panic!("Failed to connect!");
        }
    }
}

/// Used to perform IO redirection
/// It supports redirection beteen file descripters, pipe fd, files.
fn redirection(io_select: IOSelect, io_redirection: IORedirection) -> () {
    match io_redirection {
        IORedirection::pipe(pipefd) => match io_select {
            IOSelect::Input => {
                close(pipefd.1).expect("Failed to redirect IO");
                dup2(pipefd.0, 0).expect("error");
                close(pipefd.0).expect("Failed to redirect IO");
            }
            IOSelect::Output => {
                close(pipefd.0).expect("Failed to redirect IO");
                dup2(pipefd.1, 1).expect("error");
                close(pipefd.1).expect("Failed to redirect IO");
            }
            _ => (),
        },
        IORedirection::file(file, mode) => {
            let fd_to_redirect = match io_select {
                IOSelect::Input => 0,
                IOSelect::Output => 1,
                IOSelect::in_fd(fd) => fd,
                IOSelect::out_fd(fd) => fd,
            };
            match mode {
                RedirectionMode::append => {
                    let f = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(file)
                        .expect("Failed to open file");
                    let fd = f.as_raw_fd();
                    dup2(fd, fd_to_redirect).expect("error");
                    close(fd).expect("Failed to redirect IO");
                }
                RedirectionMode::write => {
                    let f = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(file)
                        .expect("Failed to open file");
                    let fd = f.as_raw_fd();
                    dup2(fd, fd_to_redirect).expect("error");
                    close(fd).expect("Failed to redirect IO");
                }
                RedirectionMode::read => {
                    let f = OpenOptions::new()
                        .read(true)
                        .open(file)
                        .expect("Failed to open file");
                    let fd = f.as_raw_fd();
                    dup2(fd, fd_to_redirect).expect("error");
                    close(fd).expect("Failed to redirect IO");
                }
            }
        }
        IORedirection::fd(obj_fd) => {
            let fd_to_redirect = match io_select {
                IOSelect::Input => 0,
                IOSelect::Output => 1,
                IOSelect::in_fd(fd) => fd,
                IOSelect::out_fd(fd) => fd,
            };
            dup2(obj_fd, fd_to_redirect).expect("error");
            close(obj_fd).expect("Failed to redirect IO");
        }
        _ => (),
    }
}

/// A string parsing tool. Returns the number(file descripter) right before
/// the delimiter. If no number exists, return None.
fn get_fd_before(source: &str, delimiter: &str) -> Option<i32> {
    let mut iter = source.split(delimiter);
    let cmd = iter.next().expect("No command");
    if let Some(fd_str) = cmd.split_whitespace().last() {
        match i32::from_str(fd_str) {
            Ok(fd_n) => Some(fd_n),
            _ => None,
        }
    } else {
        None
    }
}

/// A string parsing tool.  Returns the token after delimiter along with the substring before delimiter.
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

/// Executes built-in commands
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

/// For commands with no pipe
/// Execute commands in current process if it is a built-in command.  
/// Spawn a child process otherwise.  
fn execute_main(prog: &str, mut args: std::str::SplitWhitespace<'_>) {
    if !execute_builtin(prog, &mut args) {
        match fork() {
            Ok(ForkResult::Parent { child: _ }) => (),
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
