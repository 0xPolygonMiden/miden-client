use std::io;

fn main() -> io::Result<()> {
    // Compile the proto files into Rust code
    tonic_build::configure().build_server(false).build_client(true).compile(
        &[
            "src/web_client/rpc/proto/account.proto",
            "src/web_client/rpc/proto/block_header.proto",
            "src/web_client/rpc/proto/block_producer.proto",
            "src/web_client/rpc/proto/digest.proto",
            "src/web_client/rpc/proto/merkle.proto",
            "src/web_client/rpc/proto/mmr.proto",
            "src/web_client/rpc/proto/note.proto",
            "src/web_client/rpc/proto/requests.proto",
            "src/web_client/rpc/proto/responses.proto",
            "src/web_client/rpc/proto/rpc.proto",
            "src/web_client/rpc/proto/smt.proto",
            "src/web_client/rpc/proto/store.proto",
            "src/web_client/rpc/proto/transaction.proto",
        ],
        &["src/web_client/rpc/proto"],
    )
}
