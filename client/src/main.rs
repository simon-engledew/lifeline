extern crate libc;

extern crate getopts;
use getopts::Options;
use std::env;

use std::net::TcpStream;

unsafe fn daemonize() -> libc::pid_t {
    let process_id = libc::fork();

    if process_id < 0
    {
        panic!("fork failed");
    }

    if process_id > 0
    {
        return process_id;
    }

    libc::umask(0);

    let sid = libc::setsid();

    if sid < 0
    {
        panic!("failed to lead session")
    }

    drop(std::io::stdin);
    drop(std::io::stdout);
    drop(std::io::stderr);

    return process_id;
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} HOST [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();

    opts.optopt("p", "port", "port to connect to", "PORT");
    opts.optflag("h", "help", "print this help menu");

    let matches = opts.parse(&args[1..]).expect("Failed to parse options");

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let host = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        print_usage(&program, opts);
        return;
    };

    let port = matches.opt_str("p").unwrap_or(String::from("8888")).parse::<u16>().expect("Invalid port");

    let connection = TcpStream::connect((host.as_ref(), port)).expect("Failed to open connection");

    let pid = unsafe { daemonize() };

    if pid > 0 {
        print!("{}", pid);

        drop(connection);

        return;
    }

    unsafe { libc::pause() };

    unreachable!();
}
