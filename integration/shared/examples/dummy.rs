fn main() {
  let current_dir = dbg!(std::env::current_dir().unwrap());

  let target_dir = current_dir.join("demo/examples");

  for entry in target_dir.read_dir().into_iter().flatten().flatten() {
    dbg!(entry.file_name());
  }
}
