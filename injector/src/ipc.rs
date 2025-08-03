use std::hash::{Hash, Hasher};

use interprocess::local_socket::{
  GenericNamespaced, ListenerOptions, Name as SocketName, ToNsName, tokio::Stream,
  traits::tokio::Listener as _,
};
use shared_types::message::{CobsAccumulator, FeedResult, Message};
use tokio::{io::AsyncReadExt, sync::Notify};
use tracing::{debug, trace};

pub static PATCH_COMPLETE: Notify = Notify::const_new();

pub fn generate_socket_name() -> (SocketName<'static>, String) {
  let mut hasher = std::hash::DefaultHasher::new();
  std::process::id().hash(&mut hasher);
  let name = format!("{}.sock", hasher.finish());
  (
    name
      .as_str()
      .to_ns_name::<GenericNamespaced>()
      .unwrap()
      .into_owned(),
    name,
  )
}

pub fn start_message_listener(socket_name: SocketName<'_>) {
  let listener = ListenerOptions::new()
    .name(socket_name)
    .create_tokio()
    .expect("Create IPC socket");

  trace!("Starting message listener");
  tokio::spawn(async move {
    loop {
      if let Ok(stream) = listener.accept().await {
        trace!("Incoming connection, spawning handler");
        tokio::spawn(handle_connection(stream));
      }
    }
  });
}

async fn handle_connection(mut stream: Stream) {
  let mut buf = [0u8; 128];
  let mut cobs_buf: CobsAccumulator<1024> = CobsAccumulator::new();

  while let Ok(len) = stream.read(&mut buf).await {
    if len == 0 {
      break;
    }

    let mut window = &buf[0..len];

    'cobs: while !window.is_empty() {
      window = match cobs_buf.feed::<Message>(&window) {
        FeedResult::Consumed => break 'cobs,
        FeedResult::OverFull(items) | FeedResult::DeserError(items) => items,
        FeedResult::Success { data, remaining } => {
          match data {
            Message::FinishedPatching => {
              trace!(target: "hooked_process.hooks", "Patching complete");
              PATCH_COMPLETE.notify_one();
            }
            message => debug!(target: "hooked_process.hooks", "{message}"),
          }

          remaining
        }
      }
    }
  }
}
