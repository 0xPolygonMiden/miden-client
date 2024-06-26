pub mod rpc {
    tonic::include_proto!("rpc");
}

pub mod block_producer {
    tonic::include_proto!("block_producer");
}

pub mod store {
    tonic::include_proto!("store");
}

pub mod responses {
    tonic::include_proto!("responses");
}

pub mod requests {
    tonic::include_proto!("requests");
}

pub mod block_header {
    tonic::include_proto!("block_header");
}

pub mod note {
    tonic::include_proto!("note");
}

pub mod transaction {
    tonic::include_proto!("transaction");
}

pub mod account {
    tonic::include_proto!("account");
}

pub mod mmr {
    tonic::include_proto!("mmr");
}

pub mod smt {
    tonic::include_proto!("smt");
}

pub mod merkle {
    tonic::include_proto!("merkle");
}

pub mod digest {
    tonic::include_proto!("digest");
}
