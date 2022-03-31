use bitcoin::Amount;
use sapio::contract::macros::*;
use sapio::contract::Contract;
use sapio::contract::StatefulArgumentsTrait;
use sapio::decl_continuation;
use sapio::util::amountrange::AmountU64;
use sapio_base::timelocks::AbsHeight;
use sapio_trait::SapioJSONTrait;
use sapio_wasm_plugin::client::*;
use sapio_wasm_plugin::*;
use schemars::*;
use serde::*;
use serde_json::Value;
use std::convert::TryFrom;
use std::str::FromStr;
use std::sync::Arc;

/// # Trait for a Mintable NFT
#[derive(Serialize, JsonSchema, Deserialize, Clone)]
pub struct Mint_NFT_Trait_Version_0_1_0 {
    /// # Creator Key
    #[schemars(with = "bitcoin::hashes::sha256::Hash")]
    pub creator: bitcoin::XOnlyPublicKey,
    /// # Initial Owner
    /// The key that will own this NFT
    #[schemars(with = "bitcoin::hashes::sha256::Hash")]
    pub owner: bitcoin::XOnlyPublicKey,
    /// # Locator
    /// A piece of information that will instruct us where the NFT can be
    /// downloaded -- e.g. an IPFs Hash
    pub locator: String,
    /// # Minting Module
    /// If a specific sub-module is to be used / known -- when in doubt, should
    /// be None.
    pub minting_module: Option<SapioHostAPI<Mint_NFT_Trait_Version_0_1_0>>,
    /// how much royalty, should be paid, as a percent
    pub royalty: f64,
}

/// Boilerplate for the Mint trait
pub mod mint_impl {
    use super::*;
    #[derive(Serialize, Deserialize, JsonSchema)]
    pub enum Versions {
        Mint_NFT_Trait_Version_0_1_0(Mint_NFT_Trait_Version_0_1_0),
    }
    /// we must provide an example!
    impl SapioJSONTrait for Mint_NFT_Trait_Version_0_1_0 {
        fn get_example_for_api_checking() -> Value {
            let key = "02996fe4ed5943b281ca8cac92b2d0761f36cc735820579da355b737fb94b828fa";
            let ipfs_hash = "bafkreig7r2tdlwqxzlwnd7aqhkkvzjqv53oyrkfnhksijkvmc6k57uqk6a";
            serde_json::to_value(mint_impl::Versions::Mint_NFT_Trait_Version_0_1_0(
                Mint_NFT_Trait_Version_0_1_0 {
                    creator: bitcoin::XOnlyPublicKey::from_str(key).unwrap(),
                    owner: bitcoin::XOnlyPublicKey::from_str(key).unwrap(),
                    locator: ipfs_hash.into(),
                    minting_module: None,
                    royalty: 0.02,
                },
            ))
            .unwrap()
        }
    }
}

/// # NFT Sale Trait
/// A trait for coordinating a sale of an NFT
#[derive(Serialize, JsonSchema, Deserialize, Clone)]
pub struct NFT_Sale_Trait_Version_0_1_0 {
    /// # Owner
    /// The key that will own this NFT
    #[schemars(with = "bitcoin::hashes::sha256::Hash")]
    pub sell_to: bitcoin::XOnlyPublicKey,
    /// # Price
    /// The price in Sats
    pub price: AmountU64,
    /// # NFT
    /// The NFT's Current Info
    pub data: Mint_NFT_Trait_Version_0_1_0,
    /// # Sale Time
    /// When the sale should be possible after
    pub sale_time: AbsHeight,
    /// # Extra Information
    /// Extra information required by this contract, if any.
    /// Must be Optional for consumer or typechecking will fail.
    /// Usually None unless you know better!
    pub extra: Option<Value>,
}

/// Boilerplate for the Sale trait
pub mod sale_impl {
    use super::*;
    #[derive(Serialize, Deserialize, JsonSchema)]
    pub enum Versions {
        /// # Batching Trait API
        NFT_Sale_Trait_Version_0_1_0(NFT_Sale_Trait_Version_0_1_0),
    }
    impl SapioJSONTrait for NFT_Sale_Trait_Version_0_1_0 {
        fn get_example_for_api_checking() -> Value {
            let key = "02996fe4ed5943b281ca8cac92b2d0761f36cc735820579da355b737fb94b828fa";
            let ipfs_hash = "bafkreig7r2tdlwqxzlwnd7aqhkkvzjqv53oyrkfnhksijkvmc6k57uqk6a";
            serde_json::to_value(sale_impl::Versions::NFT_Sale_Trait_Version_0_1_0(
                NFT_Sale_Trait_Version_0_1_0 {
                    sell_to: bitcoin::XOnlyPublicKey::from_str(key).unwrap(),
                    price: AmountU64::from(0u64),
                    data: Mint_NFT_Trait_Version_0_1_0 {
                        creator: bitcoin::XOnlyPublicKey::from_str(key).unwrap(),
                        owner: bitcoin::XOnlyPublicKey::from_str(key).unwrap(),
                        locator: ipfs_hash.into(),
                        minting_module: None,
                        royalty: 0.02,
                    },
                    sale_time: AbsHeight::try_from(0).unwrap(),
                    extra: None,
                },
            ))
            .unwrap()
        }
    }
}

/// # Sellable NFT Function
/// If a NFT should be sellable, it should have this trait implemented.
pub trait SellableNFT: Contract {
    decl_continuation! {<web={}> sell<Sell>}
}
/// # Sell Instructions
#[derive(Serialize, Deserialize, JsonSchema)]
pub enum Sell {
    /// # Hold
    /// Don't transfer this NFT
    Hold,
    /// # MakeSale
    /// Transfer this NFT
    MakeSale {
        /// # Which Sale Contract to use?
        /// Specify a hash/name for a contract to generate the sale with.
        which_sale: SapioHostAPI<NFT_Sale_Trait_Version_0_1_0>,
        /// # The information needed to create the sale
        sale_info: NFT_Sale_Trait_Version_0_1_0,
    },
}
impl Default for Sell {
    fn default() -> Sell {
        Sell::Hold
    }
}
impl StatefulArgumentsTrait for Sell {}
