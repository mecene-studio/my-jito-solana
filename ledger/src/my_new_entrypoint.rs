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

use crate::shred::ShredCode;
use crate::shred::ShredData;
use shred::Shred;

#[macro_use]
extern crate solana_metrics;

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate solana_frozen_abi_macro;

use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Write;
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

    let mut dict = HashMap::new();

    let mut buf = [0u8; 4096]; // Adjust buffer size as needed
    loop {
        let (nb_bytes, src) = socket.recv_from(&mut buf).await?;
        // println!("Server got: {} bytes from {}", nb_bytes, src);

        let shred_data = buf[..nb_bytes].to_vec();
        let shred_result = Shred::new_from_serialized_shred(shred_data);

        if let Ok(shred) = shred_result {
            match shred {
                Shred::ShredCode(shred_code) => match shred_code {
                    ShredCode::Legacy(legacy_shred_code) => {
                        // Access fields of `legacy_shred_code` if needed
                        // println!("shred_common_header {:?}", legacy_shred_code.common_header);
                        // println!("shred_coding_header {:?}", legacy_shred_code.coding_header);
                    }
                    ShredCode::Merkle(merkle_shred_code) => {
                        // Access fields of `merkle_shred_code`
                        // println!("shred_common_header {:?}", merkle_shred_code.coding_header);
                        // println!("shred_coding_header {:?}", merkle_shred_code.coding_header);
                        // println!("shred_payload len {:?}", merkle_shred_code.payload.len());
                    }
                },
                Shred::ShredData(shred_data) => match shred_data {
                    ShredData::Legacy(legacy_shred_data) => {
                        // Access fields of `legacy_shred_data` if needed
                        println!(
                            "Legacy shred_common_header {:?}",
                            legacy_shred_data.common_header
                        );
                        println!(
                            "Legacy shred_coding_header {:?}",
                            legacy_shred_data.data_header
                        );
                        println!(
                            "Legacy shred_payload len {:?}",
                            legacy_shred_data.payload.len()
                        );
                    }
                    ShredData::Merkle(merkle_shred_data) => {
                        // Access fields of `merkle_shred_data`
                        // println!(
                        //     "Merkle shred_common_header {:?}",
                        //     merkle_shred_data.common_header
                        // );
                        // println!(
                        //     "Merkle shred_coding_header {:?}",
                        //     merkle_shred_data.data_header
                        // );
                        // println!(
                        //     "Merkle shred_payload len {:?}",
                        //     merkle_shred_data.payload.len()
                        // );

                        let slot = merkle_shred_data.common_header.slot;
                        let index = merkle_shred_data.common_header.index;

                        println!("slot: {:?}, index: {:?}", slot, index);

                        dict.entry(slot)
                            .or_insert(HashMap::new())
                            .entry(index)
                            .or_insert(shred);
                    }
                },
            }
        } else if let Err(e) = shred_result {
            // Handle the error case
            println!("Error deserializing shred: {:?}", e);
        }
        if i % 1000 == 0 {
            // Sort the indexes within each slot
            let sorted_dict: HashMap<u64, HashMap<u32, Shred>> =
                dict.clone().into_iter().map(|(slot, mut shred_map)| {
                    let mut sorted_shred_map: HashMap<u32, Shred> = shred_map
                        .drain()
                        .map(|(index, shred)| (index, shred))
                        .collect();
                    (slot, sorted_shred_map)
                });

            // Serialize the sorted dictionary to a JSON string
            let serialized = serde_json::to_string_pretty(&sorted_dict).unwrap();

            println!("saving dict to file");

            // Write the JSON string to a file
            let mut file = File::create("dict.json").unwrap();
            file.write_all(serialized.as_bytes()).unwrap();
        }

        i += 1;
    }
}
