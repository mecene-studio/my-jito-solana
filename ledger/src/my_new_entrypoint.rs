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

use shredder::Shredder;
use solana_entry::entry::Entry;
use solana_sdk::pubkey::Pubkey;

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

// use serde_json;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Write;
use std::str::FromStr;
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

    const SLOT_DELAY: u64 = 5;

    let mut current_slot = 0;

    let mut slots_processed = 0;

    let mut dict = HashMap::new();

    let mut buf = [0u8; 4096]; // Adjust buffer size as needed

    let target_program_pubky: Pubkey =
        Pubkey::from_str("TSWAPaqyCSx2KABk68Shruf4rp7CxcNi8hAsbdwmHbN").unwrap();
    // Pubkey::from_str("Vote111111111111111111111111111111111111111").unwrap();

    loop {
        let (nb_bytes, src) = socket.recv_from(&mut buf).await?;
        // println!("Server got: {} bytes from {}", nb_bytes, src);

        let shred_raw = buf[..nb_bytes].to_vec();
        let shred_result = Shred::new_from_serialized_shred(shred_raw);

        if let Ok(shred) = shred_result {
            // match shred {
            //     Shred::ShredCode(shred_code) => match shred_code {
            //         ShredCode::Legacy(legacy_shred_code) => {
            //             // Access fields of `legacy_shred_code` if needed
            //             // println!("shred_common_header {:?}", legacy_shred_code.common_header);
            //             // println!("shred_coding_header {:?}", legacy_shred_code.coding_header);
            //         }
            //         ShredCode::Merkle(merkle_shred_code) => {
            //             // Access fields of `merkle_shred_code`
            //             // println!("shred_common_header {:?}", merkle_shred_code.coding_header);
            //             // println!("shred_coding_header {:?}", merkle_shred_code.coding_header);
            //             // println!("shred_payload len {:?}", merkle_shred_code.payload.len());
            //         }
            //     },
            //     Shred::ShredData(shred_data) => match shred_data {
            //         ShredData::Legacy(legacy_shred_data) => {
            //             // Access fields of `legacy_shred_data` if needed
            //             println!(
            //                 "Legacy shred_common_header {:?}",
            //                 legacy_shred_data.common_header
            //             );
            //             println!(
            //                 "Legacy shred_coding_header {:?}",
            //                 legacy_shred_data.data_header
            //             );
            //             println!(
            //                 "Legacy shred_payload len {:?}",
            //                 legacy_shred_data.payload.len()
            //             );
            //         }
            //         ShredData::Merkle(merkle_shred_data) => {
            //             let slot = merkle_shred_data.common_header.slot;
            //             let index = merkle_shred_data.common_header.index;

            //             // println!("slot: {:?}, index: {:?}", slot, index);

            //             dict.entry(slot)
            //                 .or_insert(HashMap::new())
            //                 .entry(index)
            //                 .or_insert(merkle_shred_data);

            //             if slot > current_slot {
            //                 current_slot = slot;
            //                 println!("current_slot: {:?}", current_slot);
            //             }
            //         }
            //     },
            // }

            if let Shred::ShredData(ref shred_data) = shred {
                if let ShredData::Merkle(ref merkle_shred_data) = *shred_data {
                    let slot = merkle_shred_data.common_header.slot;
                    let index = merkle_shred_data.common_header.index;

                    // println!("slot: {:?}, index: {:?}", slot, index);

                    dict.entry(slot)
                        .or_insert_with(HashMap::new)
                        .entry(index)
                        .or_insert(shred);

                    if slot > current_slot {
                        current_slot = slot;
                        println!("current_slot: {:?}", current_slot);
                    }
                }
            }
        } else if let Err(e) = shred_result {
            // Handle the error case
            println!("Error deserializing shred: {:?}", e);
        }
        if i % 1000 == 0 {
            // // Sort the indexes within each slot

            // // Serialize the sorted dictionary to a JSON string
            // let serialized = serde_json::to_string_pretty(&dict).unwrap();

            // println!("saving dict to file");

            // // Write the JSON string to a file
            // let mut file = File::create("dict.json").unwrap();
            // file.write_all(serialized.as_bytes()).unwrap();

            // println!("dict: {:?}", dict);

            if current_slot > SLOT_DELAY {
                let target_slot = current_slot - SLOT_DELAY;

                if dict.contains_key(&target_slot) {
                    let target_slot_dict = dict.get(&target_slot).unwrap();

                    let mut indexes = target_slot_dict.keys().collect::<Vec<&u32>>();
                    indexes.sort();

                    let min_index = indexes[0];
                    let max_index = indexes[indexes.len() - 1];

                    let missing_indexes: Vec<u32> = (*min_index..=*max_index)
                        .filter(|j| !indexes.contains(&j))
                        .collect();

                    println!(
                        "target_slot: {:?}, min_index: {:?}, max_index: {:?}",
                        target_slot, min_index, max_index
                    );
                    println!("missing_indexes: {:?}", missing_indexes);

                    if missing_indexes.len() == 0 {
                        println!("No missing indexes, deshredding");

                        let data_shreds = indexes
                            .iter()
                            .map(|index| {
                                let shred_data = target_slot_dict.get(index).unwrap();
                                shred_data.clone() // Clone the Shred object
                            })
                            .collect::<Vec<Shred>>(); // Collect into Vec<Shred>

                        // println!(
                        //     "data_shreds len: {:?}, example: {:?}",
                        //     data_shreds.len(),
                        //     data_shreds[0]
                        // );

                        let deshred_payload = Shredder::deshred(&data_shreds[..]).unwrap();

                        // println!(
                        //     "deshred_payload len: {:?}, example: {:?}",
                        //     deshred_payload.len(),
                        //     deshred_payload[0]
                        // );

                        let deshred_entries: Vec<Entry> =
                            bincode::deserialize(&deshred_payload).unwrap();

                        // println!(
                        //     "deshred_entries len: {:?}, example: {:?}",
                        //     deshred_entries.len(),
                        //     deshred_entries[0]
                        // );

                        let nb_txs: usize = deshred_entries
                            .iter()
                            .map(|entry| entry.transactions.len())
                            .sum();

                        println!(
                            "nb entries: {:?}, nb_txs: {:?}",
                            deshred_entries.len(),
                            nb_txs
                        );

                        for entry in deshred_entries.iter() {
                            for tx in entry.transactions.iter() {
                                if tx
                                    .message
                                    .static_account_keys()
                                    .contains(&target_program_pubky)
                                {
                                    let now: DateTime<Utc> = Utc::now();
                                    let utc_string = now.to_rfc2822();
                                    println!(
                                        "\nfound tx with target program id at {:?}",
                                        utc_string
                                    );
                                    println!("tx: {:?}", tx);
                                }
                            }
                        }

                        // write entries to file
                        let serialized = serde_json::to_string_pretty(&deshred_entries).unwrap();
                        let file_name = format!("slots/entries_{}.json", target_slot);
                        let mut file = std::fs::File::create(file_name).unwrap();
                        file.write_all(serialized.as_bytes()).unwrap();
                    }
                }
            }
        }

        i += 1;
    }
}

// scp phil@35.245.148.173:/home/phil/dev/my-jito-solana/slots/entries_255012011.json slots/entries_255012011.json
