use web3::{types::{Transaction, BlockNumber}, transports::WebSocket, Web3};

pub fn isContract(tx: &Transaction) -> bool {
    !(tx.input.0.len() < 4)
}

pub async fn hasCode(interface: Web3<WebSocket>, tx: &Transaction) -> bool {
    0 != interface.eth().code(tx.to.unwrap(), Some(BlockNumber::Latest)).await.unwrap().0.len()
}

pub mod erc721 {
    use std::collections::HashMap;
    use std::fmt::format;
    use std::fs::{File, OpenOptions};
    use std::io;
    use serde::{Serialize, Deserialize};
    use serde_json::{json, Value};

    use web3::{types::{Transaction, U256, H160}, contract::{Contract, Error, Options}, Transport, Web3};
    use crate::ipfs::{to_ipfs};

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct ContractManager(String, HashMap<H160, String>);

    impl ContractManager {
        pub async fn new<T: ToString>(file: T) -> Self {
            let temp = ContractManager(file.to_string(), HashMap::new());
            temp.get().await
        }

        async fn save(self) {
            let fhandler = OpenOptions::new()
                .write(true)
                .open(self.0)
                .expect("Failed to open file");

            serde_json::to_writer(fhandler, &self.1)
                .expect("Unable to write");
        }

        pub async fn get(mut self) -> ContractManager {
            let fhandler = File::open(&self.0[..])
                .unwrap_or_else(|_| {
                    File::create(&self.0[..])
                        .expect("Unable to create new file")
                });

            let save: HashMap<H160, String> = serde_json::from_reader(fhandler)
                .unwrap_or_else(|_| {
                    println!("Creating new contract save:\nFile: {}", self.0);
                    HashMap::new()
                });

            self.1 = save;

            self
        }

        pub async fn add<T: ToString>(mut self, addr:H160, data: T) -> bool {
            if !(&self.1.contains_key(&addr)) {
                self.1.insert(addr, data.to_string());
                self.save().await;
                return true;
            }
            false
        }
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Root {
        pub count: i64,
        pub next: Value,
        pub previous: Value,
        pub results: Vec<EthFunc>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EthFunc {
        pub id: i64,
        #[serde(rename = "created_at")]
        pub created_at: String,
        #[serde(rename = "text_signature")]
        pub text_signature: String,
        #[serde(rename = "hex_signature")]
        pub hex_signature: String,
        #[serde(rename = "bytes_signature")]
        pub bytes_signature: String,
    }

    pub async fn token_uri<T: Transport>(id: u128, contract: Contract<T>) -> Result<String, Error> {
        let id = U256::from(id);

        let uri: String = contract
            .query("tokenURI", id, None, Options::default(), None)
            .await?;
        
        Ok(uri)
    }

    pub async fn supply<T: Transport>(contract: Contract<T>) -> Result<u128, Error> {
        let supply: U256 = contract
            .query("totalSupply", (), None, Options::default(), None)
            .await?;
        
        Ok(supply.as_u128())
    }

    // check if the transaction function contains mint or NFT
    pub async fn is_mint_function(tx: &Transaction) -> bool {
        if super::isContract(tx) {
            let bytes = &tx.input.0;
            let mut sig: String = String::from("0x");
            for byte in &bytes[..4] {
                sig.push_str(&format!("{:02x}", byte));
            }

            let raw_resp = reqwest::get(format!("https://www.4byte.directory/api/v1/signatures/?format=json&hex_signature={}", sig))
                .await;

            match raw_resp {
                Ok(resp) => {
                    let resp = resp
                        .json::<Root>()
                        .await.unwrap();
                    
                        if resp.count  != 0 {
                        for result in resp.results {
                            if result.text_signature.contains("NFT") || result.text_signature.contains("mint") {
                                return true;
                            }
                        }
                    }

                    return false;
                },
                Err(err) => {
                    println!("Error {} @ {}", err, sig);
                    return false;
                }
            }
        }
        false
    }

    #[derive(Debug)]
    pub struct Erc721Info {
        pub name: String,
        pub description: String,
        pub uri: String,
        pub image_uri: String
    }

    impl Erc721Info {
        pub fn set_uri<T: ToString>(mut self, uri: T) -> Self {
            self.uri = to_ipfs(uri.to_string());
            self
        }
    }

    impl From<serde_json::Value> for Erc721Info {
        fn from(data: Value) -> Self {
            Erc721Info {
                name: data["name"].to_string(),
                description: data["description"].to_string(),
                uri: "".to_string(),
                image_uri: to_ipfs(data["image"].to_string())
            }
        }
    }

    pub async fn resolve_contract<T: Transport>(interface: &Web3<T>, address: H160, managers: (ContractManager, ContractManager)) -> Result<Option<Erc721Info>, Box<dyn std::error::Error>> {
        let contract = Contract::from_json(interface.eth(), address, include_bytes!("erc721_simple.json"))
            .expect("Failed to initialise smart contract");

        match token_uri(1, contract).await {
            Ok(uri) => {
                if managers.0.add(address, &uri[..]).await {
                    let raw_resp = reqwest::get(to_ipfs(&uri[..]))
                        .await;

                    match raw_resp {
                        Ok(resp) => {
                            let json_data = resp.text()
                                .await?;
                            let serde_data: serde_json::Value = serde_json::from_str(
                                &json_data[..])?;

                            return Ok(Some(Erc721Info::from(serde_data).set_uri(&uri[..])));
                        },
                        Err(err) => {
                            return Err(Box::new(io::Error::new(io::ErrorKind::AddrNotAvailable, format!("Unable to reach endpoint: {}", err.to_string()))));
                        }
                    }
                }
            }
            Err(err) => {
                managers.1.add(address, err.to_string()).await;
                return Err(Box::new(io::Error::new(io::ErrorKind::InvalidData, format!("Unable to call tokenURI(the endpoint might not support it): {}", err.to_string()))));
            }
        }

        Ok(None)
    }
}