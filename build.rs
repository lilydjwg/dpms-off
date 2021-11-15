use std::env::var;
use std::path::Path;

use wayland_scanner::{Side, generate_code};

fn main() {
  let protocol_file = "proto/idle.xml";

  let out_dir_str = var("OUT_DIR").unwrap();
  let out_dir = Path::new(&out_dir_str);

  generate_code(
    protocol_file,
    out_dir.join("idle.rs"),
    Side::Client,
  );
}
