fn main() {
  std::thread::sleep(std::time::Duration::from_secs(2));

  let mut args = std::env::args();
  let path = args.next().unwrap();
  let delete = args.next().as_deref() == Some("delete");

  if delete {
    std::fs::remove_dir(path).unwrap()
  } else {
    std::fs::create_dir(path).unwrap()
  }
}
