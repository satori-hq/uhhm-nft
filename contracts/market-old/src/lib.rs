use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, AccountId, Balance, Gas, PanicOnDefault,
    Promise, PromiseOrValue, CryptoHash, BorshStorageKey,
};
use std::cmp::min;
use std::collections::HashMap;

use crate::external::*;
use crate::internal::*;
use crate::sale::*;
use near_sdk::env::STORAGE_PRICE_PER_BYTE;

mod external;
mod ft_callbacks;
mod internal;
mod nft_callbacks;
mod sale;
mod sale_views;

// near_sdk::setup_alloc!();

// TODO check seller supports storage_deposit at ft_token_id they want to post sale in

const GAS_FOR_FT_TRANSFER: Gas = Gas(5_000_000_000_000);
/// greedy max Tgas for resolve_purchase
const GAS_FOR_ROYALTIES: Gas = Gas(115_000_000_000_000);
const GAS_FOR_NFT_TRANSFER: Gas = Gas(15_000_000_000_000);
const BID_HISTORY_LENGTH_DEFAULT: u8 = 1;
const NO_DEPOSIT: Balance = 0;
const STORAGE_PER_SALE: u128 = 1000 * STORAGE_PRICE_PER_BYTE;
static DELIMETER: &str = "||";

pub type SaleConditions = HashMap<FungibleTokenId, U128>;
pub type Bids = HashMap<FungibleTokenId, Vec<Bid>>;
pub type TokenId = String;
pub type TokenType = Option<String>;
pub type FungibleTokenId = AccountId;
pub type ContractAndTokenId = String;

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Payout {
	pub payout: HashMap<AccountId, U128>
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub owner_id: AccountId,
    pub sales: UnorderedMap<ContractAndTokenId, Sale>,
    pub by_owner_id: LookupMap<AccountId, UnorderedSet<ContractAndTokenId>>,
    pub by_nft_contract_id: LookupMap<AccountId, UnorderedSet<TokenId>>,
    pub by_nft_token_type: LookupMap<AccountId, UnorderedSet<ContractAndTokenId>>,
    pub ft_token_ids: UnorderedSet<AccountId>,
    pub storage_deposits: LookupMap<AccountId, Balance>,
    pub bid_history_length: u8,
}

/// Helper structure to for keys of the persistent collections.
#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKey {
    Sales,
    ByOwnerId,
    ByOwnerIdInner { account_id_hash: CryptoHash },
    ByNFTContractId,
    ByNFTContractIdInner { account_id_hash: CryptoHash },
    ByNFTTokenType,
    ByNFTTokenTypeInner { token_type_hash: CryptoHash },
    FTTokenIds,
    StorageDeposits,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, ft_token_ids:Option<Vec<AccountId>>, bid_history_length:Option<u8>) -> Self {
        let mut this = Self {
            owner_id: owner_id.into(),
            sales: UnorderedMap::new(StorageKey::Sales),
            by_owner_id: LookupMap::new(StorageKey::ByOwnerId),
            by_nft_contract_id: LookupMap::new(StorageKey::ByNFTContractId),
            by_nft_token_type: LookupMap::new(StorageKey::ByNFTTokenType),
            ft_token_ids: UnorderedSet::new(StorageKey::FTTokenIds),
            storage_deposits: LookupMap::new(StorageKey::StorageDeposits),
            bid_history_length: bid_history_length.unwrap_or(BID_HISTORY_LENGTH_DEFAULT),
        };
        // support NEAR by default
        this.ft_token_ids.insert(&AccountId::new_unchecked("near".to_string()));
        
        if let Some(ft_token_ids) = ft_token_ids {
            for ft_token_id in ft_token_ids {
                this.ft_token_ids.insert(&ft_token_id);
            }
        }

        this
    }

    /// only owner 
    pub fn add_ft_token_ids(&mut self, ft_token_ids: Vec<AccountId>) -> Vec<bool> {
        self.assert_owner();
        let mut added = vec![];
        for ft_token_id in ft_token_ids {
            added.push(self.ft_token_ids.insert(&ft_token_id));
        }
        added
    }

    /// TODO remove token (should check if sales can complete even if owner stops supporting token type)

    #[payable]
    pub fn storage_deposit(&mut self, account_id: Option<AccountId>) {
        let storage_account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(env::predecessor_account_id);
        let deposit = env::attached_deposit();
        assert!(
            deposit >= STORAGE_PER_SALE,
            "Requires minimum deposit of {}",
            STORAGE_PER_SALE
        );
        let mut balance: u128 = self.storage_deposits.get(&storage_account_id).unwrap_or(0);
        balance += deposit;
        self.storage_deposits.insert(&storage_account_id, &balance);
    }

    #[payable]
    pub fn storage_withdraw(&mut self) {
        assert_one_yocto();
        let owner_id = env::predecessor_account_id();
        let mut amount = self.storage_deposits.remove(&owner_id).unwrap_or(0);
        let sales = self.by_owner_id.get(&owner_id);
        let len = sales.map(|s| s.len()).unwrap_or_default();
        amount -= u128::from(len) * STORAGE_PER_SALE;
        if amount > 0 {
            Promise::new(owner_id).transfer(amount);
        }
    }

    /// views

    pub fn supported_ft_token_ids(&self) -> Vec<AccountId> {
        self.ft_token_ids.to_vec()
    }

    pub fn storage_amount(&self) -> U128 {
        U128(STORAGE_PER_SALE)
    }

    pub fn storage_paid(&self, account_id: AccountId) -> U128 {
        U128(self.storage_deposits.get(&account_id).unwrap_or(0))
    }
}