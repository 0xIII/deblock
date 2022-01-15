mod erc;
mod twitter;
mod ipfs;

use std::{env, collections::HashMap, fs::{File, OpenOptions}, string::ToString};
use serde::{Deserialize, Serialize};
use web3::{Web3, transports::WebSocket, types::{BlockId, BlockNumber, H160}, contract::{Contract, Error}, Transport};

use crate::erc::erc721::{ContractManager, Erc721Info, resolve_contract, token_uri};
use crate::twitter::nft_tweet;

// TODO: Add log4rs

const SAV: &str = "save.json";
const ERR: &str = "error.json";

#[tokio::main]
async fn main() -> web3::Result<()>{
    dotenv::dotenv().expect(".env not found");
    let wsstring = &env::var("INFURA").unwrap();

    let ttoken = twitter::auth().await;

    let ws = WebSocket::new(wsstring).await?;
    let interface = Web3::new(ws);

    let mut last_block = interface.eth().block_with_txs(BlockId::Number(BlockNumber::Latest)).await?;
    loop {
        let current_block = interface.eth().block_with_txs(BlockId::Number(BlockNumber::Latest)).await?;
        if current_block != last_block {
            last_block = current_block.clone();
            match current_block {
                Some(blk) => {
                    let transactions = blk.transactions;
                    match blk.number {Some(x) => {println!("--{}--", x)}, _ => {}};
                    
                    for tx in transactions {
                        if erc::erc721::is_mint_function(&tx).await {
                            match tx.to {
                                Some(addr) => {
                                    let saved_contracts = ContractManager::new(SAV).await;
                                    let error_contracts = ContractManager::new(ERR).await;
                                    match resolve_contract(&interface, addr, (saved_contracts, error_contracts))
                                        .await {
                                        Ok(res) => {
                                            match res {
                                                None => {}
                                                Some(info) => {
                                                    println!("{:?}", info);
                                                }
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                }
                                None => {},
                            }
                        }
                    }
                },
                None => {
                    println!("No new blocks available!");
                }
            }
        }
    }
}