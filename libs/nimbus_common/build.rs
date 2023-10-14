use std::process::Command;

fn main() {
  let out_dir = "src/protos";

  protobuf_codegen::Codegen::new()
    .pure()
    .out_dir(out_dir)
    .inputs(["protos/message.proto"])
    .include("protos")
    .customize(protobuf_codegen::Customize::default().tokio_bytes(true))
    .run()
    .expect("Codegen failed.");
  Command::new("cargo")
    .args(["fmt"])
    .current_dir("protos")
    .output()
    .unwrap();

  println!("cargo:rerun-if-changed=protos/*.proto");
}
