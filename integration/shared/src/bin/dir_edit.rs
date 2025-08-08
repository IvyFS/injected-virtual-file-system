fn main() {
  let mut args = std::env::args().skip(1);
  let path = args.next().unwrap();
  let delete = args.next().as_deref() == Some("delete");

  if delete {
    std::fs::remove_dir(path).unwrap()
  } else {
    std::fs::create_dir(dbg!(path)).unwrap()
  }
}
