#![cfg_attr(RUSTC_WITH_SPECIALIZATION, feature(min_specialization))]
#![allow(clippy::arithmetic_side_effects)]

pub mod bank_forks_utils;
pub mod bigtable_delete;
pub mod bigtable_upload;
pub mod bigtable_upload_service;
pub mod block_error;
#[macro_use]
pub mod blockstore;
pub mod ancestor_iterator;
pub mod blockstore_db;
pub mod blockstore_meta;
pub mod blockstore_metrics;
pub mod blockstore_options;
pub mod blockstore_processor;
pub mod entry_notifier_interface;
pub mod entry_notifier_service;
pub mod genesis_utils;
pub mod leader_schedule;
pub mod leader_schedule_cache;
pub mod leader_schedule_utils;
pub mod next_slots_iterator;
pub mod rooted_slot_iterator;
pub mod shred;
mod shredder;
pub mod sigverify_shreds;
pub mod slot_stats;
mod staking_utils;
pub mod token_balances;
pub mod use_snapshot_archives_at_startup;

use shred::Shred;

#[macro_use]
extern crate solana_metrics;

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate solana_frozen_abi_macro;

use std::io;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> io::Result<()> {
    listen_to_shredstream().await
}

async fn listen_to_shredstream() -> io::Result<()> {
    let port = 8001;
    let host = "0.0.0.0"; // Listen on all available interfaces
    let addr = format!("{}:{}", host, port);

    let socket = UdpSocket::bind(&addr).await?;
    println!("Server listening on {}", addr);

    let mut i = 0;

    let mut buf = [0u8; 4096]; // Adjust buffer size as needed
    loop {
        let (nb_bytes, src) = socket.recv_from(&mut buf).await?;
        // println!("Server got: {} bytes from {}", nb_bytes, src);

        let shred_data = buf[..nb_bytes].to_vec();
        let shred = Shred::new_from_serialized_shred(shred_data);

        println!("{}, {:?}", i, shred);

        // if i % 1000 == 0 {
        //     println!("{}, {:?}, {:?}", i, shred);
        // }

        i += 1;
    }
}
