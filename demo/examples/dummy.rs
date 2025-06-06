use std::{thread::sleep, time::Duration};

fn main() {
  loop {
    println!("PID: {} | Looping infinitely", std::process::id());
    sleep(Duration::from_secs(2));
  }
}
