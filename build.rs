// Copyright 2020 David Li
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use protobuf_codegen::Customize;
use protoc_grpcio;

fn main() {
    let proto_root = "proto";
    println!("cargo:rerun-if-changed={}", proto_root);

    let customize = Customize {
        serde_derive: Some(true),
        ..Default::default()
    };

    protoc_grpcio::compile_grpc_protos(
        &["route_guide.proto"],
        &[proto_root],
        "src",
        Some(customize),
    )
    .expect("Failed to compile route_guide.proto!");
}
