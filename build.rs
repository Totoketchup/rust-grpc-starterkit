use protoc_grpcio;

// This building script generate the necessary files
fn main() {
  let proto_root = "src/protos";
  println!("cargo:rerun-if-changed={} Generating gRPC files...", proto_root) ;
  protoc_grpcio::compile_grpc_protos(
    &["example/helloworld.proto"],
    &[proto_root],
    &proto_root,
    None,
  )
  .map(|_| println!("gRPC files successfully generated."))
  .expect("Failed to compile gRPC definitions!");
}
