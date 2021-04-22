use rust_apgas::logging;
use rust_apgas::logging::*;
use rust_apgas::network;
use signal_hook::consts::signal;
use std::cell::Cell;
use std::sync::atomic;
use std::sync::Arc;
use std::thread;
use std::time;

extern crate rust_apgas;
extern crate signal_hook;

fn print_hostname(){
    use std::process::Command;
    let mut cmd = Command::new("hostname");
    let name = cmd.output().unwrap().stdout;
    info!("My hostname is {}", String::from_utf8_lossy(&name[..]));
}

pub fn main() {
    // signal handling
    let got = Arc::new(atomic::AtomicBool::new(false));
    signal_hook::flag::register(signal::SIGQUIT, Arc::clone(&got)).unwrap();

    let payload_len = 9002usize;
    let payload: Vec<u8> = (0..payload_len).map(|a| (a % 256) as u8).collect();
    let done_mark = "done".as_bytes();

    let should_terminate = Cell::new(false);

    let a = "hello world!";
    let mut callback = |src: network::Rank, buf: &[u8]| {
        if buf == done_mark {
            should_terminate.set(true);
            return;
        }
        assert_eq!(buf, &payload[..]);
        // assert_eq!(buf.len(), payload.len());
        info!("{} {} bytes from:{} ", a, buf.len(), src.as_i32());
    };
    logging::setup_logger().unwrap();
    let mut context = network::CommunicationContext::new(&mut callback);
    context.run();
    let sender = context.single_sender();
    let context = context;

    let here = context.here();
    let world = context.world_size();

    print_hostname();
    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            let mut sender = sender;
            for p in 0..world {
                sender.send(network::Rank::new(p as i32), &payload[..]);
            }
            info!("before barrier");
            log::logger().flush();
            sender.barrier();
            info!("after barrier");
            info!("send done to {}", here);
            sender.send(here, done_mark);
        });

        let sleep_itval = time::Duration::from_millis(100);
        while !should_terminate.get() && !got.load(atomic::Ordering::Relaxed) {
            thread::sleep(sleep_itval);
        }
        if got.load(atomic::Ordering::Relaxed) {
            error!("Terminate on signal")
        }
    })
    .unwrap();

    info!("exit gracefully!");
}
