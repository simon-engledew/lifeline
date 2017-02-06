use std::net::{TcpListener, TcpStream};
use std::io::Read;
use std::process;
use std::{thread, time};
use std::process::{Command, Child};
use std::sync::{Arc, Mutex, Condvar};
use std::cell::UnsafeCell;
use std::ptr;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

extern crate getopts;
use getopts::Options;
use std::env;

#[macro_use]
extern crate log;

use log::{LogRecord, LogLevel, LogLevelFilter, LogMetadata};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Info
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
}

static GLOBAL_CLIENT_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} COMMAND [options]", program);
    print!("{}", opts.usage(&brief));
}

fn shutdown(pair: &(Mutex<bool>, Condvar)) {
    let (ref lock, ref cvar) = *pair;
    let mut terminating = lock.lock().unwrap();
    *terminating = true;
    cvar.notify_one();
}

macro_rules! desync {
    ($target:ident, $var:ident, $body:expr) => (
        let reference = $target.value.get();
        let mut $var = unsafe { ptr::read(reference) };
        $body
        mem::forget($var);
    );
}

pub struct SyncWrapper {
    pub value: UnsafeCell<Child>,
}

unsafe impl Sync for SyncWrapper { }

fn main() {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        Box::new(SimpleLogger)
    }).expect("Failed to create logger");

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();

    opts.optopt("p", "port", "port to run on", "PORT");
    opts.optopt("g", "grace", "grace period to wait for clients", "GRACE");
    opts.optflag("h", "help", "print this help menu");

    let matches = opts.parse(&args[1..]).expect("Failed to parse options");

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let port = matches.opt_str("p").unwrap_or(String::from("8888")).parse::<u16>().expect("Invalid port");
    let grace = matches.opt_str("g").unwrap_or(String::from("5")).parse::<u64>().expect("Invalid grace period");

    let command_string = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        print_usage(&program, opts);
        return;
    };

    info!("command: {}", command_string);

    let listener = TcpListener::bind(("0.0.0.0", port)).expect("Failed to start server");

    let child = Command::new("sh")
        .arg("-c")
        .arg(command_string)
        .spawn()
        .expect("Failed to start process");

    let wait_wrapper = Arc::new(SyncWrapper { value: UnsafeCell::new(child) });
    let kill_wrapper = wait_wrapper.clone();

    let wait_pair = Arc::new((Mutex::new(false), Condvar::new()));
    let notify_pair = wait_pair.clone();
    let premature_pair = wait_pair.clone();

    thread::spawn(move || {
        desync!(wait_wrapper, command, {
            command.wait().expect("failed to wait for command");
            shutdown(&*premature_pair);
        });
    });

    info!("Background command started");

    thread::spawn(move || {
        let &(ref lock, ref cvar) = &*wait_pair;
        let mut terminating = lock.lock().unwrap();
        while !*terminating {
            terminating = cvar.wait(terminating).unwrap();
        }

        desync!(kill_wrapper, command, {
            match command.kill() {
                Ok(_) => match command.wait() {
                    _ => info!("Command terminated.")
                },
                Err(e) => error!("Kill failed: {}", e)
            }
        });

        process::exit(0);
    });

    let timeout_pair = notify_pair.clone();

    thread::spawn(move || {
        info!("Starting timeout with grace period of {} seconds.", grace);

        thread::sleep(time::Duration::from_secs(grace));

        if GLOBAL_CLIENT_COUNT.load(Ordering::Relaxed) == 0 {
            info!("Grace period expired.");

            shutdown(&*timeout_pair);
        }
    });

    fn handle_client(mut stream: TcpStream, pair: Arc<(Mutex<bool>, Condvar)>) {
        info!("Client connected");

        GLOBAL_CLIENT_COUNT.fetch_add(1, Ordering::Relaxed);

        let mut buffer = [0; 1];

        loop {
            if stream.read(&mut buffer).unwrap_or(0) == 0 {
                break;
            }
        };

        GLOBAL_CLIENT_COUNT.fetch_sub(1, Ordering::Relaxed);

        let count = GLOBAL_CLIENT_COUNT.load(Ordering::Relaxed);

        if count == 0 {
            info!("Last client disconnected.");

            shutdown(&*pair);
        } else {
            info!("clients: {}", count);
        }
    };

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let pair = notify_pair.clone();

                thread::spawn(move || {
                    handle_client(stream, pair)
                });
            }
            Err(e) => error!("error: {}", e)
        }
    }
}
