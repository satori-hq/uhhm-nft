use crate::*;
use near_sdk::json_types::{U64};
use near_sdk::{ext_contract, log, Gas, PromiseResult, AccountId};

const GAS_FOR_NFT_APPROVE: Gas = Gas(25_000_000_000_000);
const GAS_FOR_RESOLVE_TRANSFER_INT: u64 = 10_000_000_000_000;
const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(GAS_FOR_RESOLVE_TRANSFER_INT);
const GAS_FOR_NFT_TRANSFER_CALL: Gas = Gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER_INT);
const NO_DEPOSIT: Balance = 0;

pub trait NonFungibleTokenCore {
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    );

    //calculates the payout for a token given the passed in balance. This is a view method
    fn nft_payout(&self, token_id: TokenId, balance: U128, max_len_payout: u32) -> Payout;

    fn nft_transfer_payout(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: u64,
        memo: Option<String>,
        balance: Option<U128>,
        max_len_payout: Option<u32>,
    ) -> Option<Payout>;

    /// Returns `true` if the token was transferred from the sender's account.
    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: u64,
        memo: Option<String>,
        msg: String,
    ) -> Promise;

    fn nft_approve(&mut self, token_id: TokenId, account_id: AccountId, msg: Option<String>);

    fn nft_revoke(&mut self, token_id: TokenId, account_id: AccountId);

    fn nft_revoke_all(&mut self, token_id: TokenId);

    fn nft_total_supply(&self) -> U64;

    fn nft_token(&self, token_id: TokenId) -> Option<JsonToken>;
}

#[ext_contract(ext_non_fungible_token_receiver)]
trait NonFungibleTokenReceiver {
    /// Returns `true` if the token should be returned back to the sender.
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> Promise;
}

#[ext_contract(ext_non_fungible_approval_receiver)]
trait NonFungibleTokenApprovalsReceiver {
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    );
}

// TODO: create nft_on_revoke

#[ext_contract(ext_self)]
trait NonFungibleTokenResolver {
    fn nft_resolve_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        approved_account_ids: HashMap<AccountId, u64>,
        token_id: TokenId,
    ) -> bool;
}

trait NonFungibleTokenResolver {
    fn nft_resolve_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        approved_account_ids: HashMap<AccountId, u64>,
        token_id: TokenId,
    ) -> bool;
}

#[near_bindgen]
impl NonFungibleTokenCore for Contract {

    #[payable]
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let previous_token = self.internal_transfer(
            &sender_id,
            &receiver_id,
            &token_id,
            approval_id,
            memo,
        );
        refund_approved_account_ids(
            previous_token.owner_id.clone(),
            &previous_token.approved_account_ids,
        );
    }

    //calculates the payout for a token given the passed in balance. This is a view method
	fn nft_payout(&self, token_id: TokenId, balance: U128, max_len_payout: u32) -> Payout {
		//get the token object
		// let token = versioned_token_to_token(self.nft_token(token_id.clone()).expect("no token"));
		let token = self.nft_token(token_id.clone()).expect("no token");

		//get the owner of the token
		let owner_id = token.owner_id;
		//keep track of the total perpetual royalties
		let mut total_perpetual = 0;
		//get the u128 version of the passed in balance (which was U128 before)
		let balance_u128 = u128::from(balance);
		//keep track of the payout object to send back
		let mut payout_object = Payout {
				payout: HashMap::new()
		};
		//get the royalty object from token
		// let mut token_id_iter = token_id.split(TOKEN_DELIMETER);
		// let token_type_id = token_id_iter.next().unwrap().parse().unwrap();
        let royalty = self.tokens_by_id.get(&token_id).expect("No token").royalty;
        // let royalty = token.royalty;

		//make sure we're not paying out to too many people (GAS limits this)
		assert!(royalty.len() as u32 <= max_len_payout, "Market cannot payout to that many receivers");

		//go through each key and value in the royalty object
		for (k, v) in royalty.iter() {
			//get the key
			let key = k.clone();
			//only insert into the payout if the key isn't the token owner (we add their payout at the end)
			if key != owner_id {
				payout_object.payout.insert(key, royalty_to_payout(*v, balance_u128));
				total_perpetual += *v;
			}
		}

        // payout to contract owner - may be previous token owner, they get remainder of balance
        if self.contract_royalty_owner > 0 && self.owner_id != owner_id {
            payout_object.payout.insert(self.owner_id.clone(), royalty_to_payout(self.contract_royalty_owner, balance_u128));
            total_perpetual += self.contract_royalty_owner;
        }

        // payout to Satori
        if self.contract_royalty_satori > 0 {
            payout_object.payout.insert(AccountId::new_unchecked(SATORI_ROYALTY_ACCOUNT.to_string()), royalty_to_payout(self.contract_royalty_satori, balance_u128));
            total_perpetual += self.contract_royalty_satori;
        }

		// payout to previous owner who gets 100% - total perpetual royalties
		let owner_payout = royalty_to_payout(10000 - total_perpetual, balance_u128);
		if u128::from(owner_payout) > 0 {
			payout_object.payout.insert(owner_id, owner_payout);
		}

		//return the payout object
		payout_object
	}

    // CUSTOM - this method is included for marketplaces that respect royalties
    #[payable]
    fn nft_transfer_payout(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: u64,
        memo: Option<String>,
        balance: Option<U128>,
        max_len_payout: Option<u32>,
    ) -> Option<Payout> {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let previous_token = self.internal_transfer(
            &sender_id,
            &receiver_id,
            &token_id,
            Some(approval_id),
            memo,
        );
        refund_approved_account_ids(
            previous_token.owner_id.clone(),
            &previous_token.approved_account_ids,
        );

        // compute payouts based on balance option
        // adds in contract_royalty and computes previous owner royalty from remainder
        let owner_id = previous_token.owner_id;
        let mut total_perpetual = 0;
        let payout_struct = if let Some(balance) = balance {
            let balance_u128 = u128::from(balance);
            // let mut payout: Payout = HashMap::new();
            let mut payout_struct: Payout = Payout {
				payout: HashMap::new()
			};
            let royalty = self.tokens_by_id.get(&token_id).expect("No token").royalty;

            if let Some(max_len_payout) = max_len_payout {
                assert!(royalty.len() as u32 <= max_len_payout, "Market cannot payout to that many receivers");
            }

            for (k, v) in royalty.iter() {
                let key = k.clone();
                if key != owner_id {
                    payout_struct.payout.insert(key, royalty_to_payout(*v, balance_u128));
                    total_perpetual += *v;
                }
            }
            
            // payout to contract owner - may be previous token owner, they get remainder of balance
            if self.contract_royalty_owner > 0 && self.owner_id != owner_id {
                payout_struct.payout.insert(self.owner_id.clone(), royalty_to_payout(self.contract_royalty_owner, balance_u128));
                total_perpetual += self.contract_royalty_owner;
            }

            // payout to Satori
            if self.contract_royalty_satori > 0 {
                payout_struct.payout.insert(AccountId::new_unchecked(SATORI_ROYALTY_ACCOUNT.to_string()), royalty_to_payout(self.contract_royalty_satori, balance_u128));
                total_perpetual += self.contract_royalty_satori;
            }

            assert!(total_perpetual <= MINTER_ROYALTY_CAP + CONTRACT_ROYALTY_CAP, "Royalties should not be more than caps");
            // payout to previous owner
            payout_struct.payout.insert(owner_id.clone(), royalty_to_payout(10000 - total_perpetual, balance_u128));

            Some(payout_struct)
        } else {
            None
        };

        env::log_str(format!("{}{}", EVENT_JSON, json!({
			"standard": "nep171",
			"version": "1.0.0",
			"event": "nft_transfer",
			"data": [
				{
					"old_owner_id": owner_id, "new_owner_id": receiver_id, "token_ids": [token_id]
				}
			]
		})).as_ref());

        payout_struct
    }

    #[payable]
    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: u64,
        memo: Option<String>,
        msg: String,
    ) -> Promise {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let previous_token = self.internal_transfer(
            &sender_id,
            &receiver_id,
            &token_id,
            Some(approval_id),
            memo,
        );
        // Initiating receiver's call and the callback
        ext_non_fungible_token_receiver::nft_on_transfer(
            sender_id,
            previous_token.owner_id.clone(),
            token_id.clone(),
            msg,
            receiver_id.clone(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_NFT_TRANSFER_CALL,
        )
        .then(ext_self::nft_resolve_transfer(
            previous_token.owner_id,
            receiver_id.into(),
            previous_token.approved_account_ids,
            token_id,
            env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
    }

    #[payable]
    fn nft_approve(&mut self, token_id: TokenId, account_id: AccountId, msg: Option<String>) {
        assert_at_least_one_yocto();
        let account_id: AccountId = account_id.into();

        let mut token = self.tokens_by_id.get(&token_id).expect("Token not found");

        assert_eq!(
            &env::predecessor_account_id(),
            &token.owner_id,
            "Predecessor must be the token owner."
        );

        let approval_id: u64 = token.next_approval_id;
        let is_new_approval = token
            .approved_account_ids
            .insert(account_id.clone(), approval_id)
            .is_none();

        let storage_used = if is_new_approval {
            bytes_for_approved_account_id(&account_id)
        } else {
            0
        };

        token.next_approval_id += 1;
        self.tokens_by_id.insert(&token_id, &token);

        refund_deposit(storage_used);

        if let Some(msg) = msg {
            
            // CUSTOM - add token_type to msg
            // let mut final_msg = msg;
            // let token_type = token.token_type;
            // if let Some(token_type) = token_type {
            //     final_msg.insert_str(final_msg.len() - 1, &format!(",\"token_type\":\"{}\"", token_type));
            // }

            ext_non_fungible_approval_receiver::nft_on_approve(
                token_id,
                token.owner_id,
                approval_id,
                msg,
                account_id,
                NO_DEPOSIT,
                env::prepaid_gas() - GAS_FOR_NFT_APPROVE,
            )
            .as_return(); // Returning this promise
        }
    }

    #[payable]
    fn nft_revoke(&mut self, token_id: TokenId, account_id: AccountId) {
        assert_one_yocto();
        let mut token = self.tokens_by_id.get(&token_id).expect("Token not found");
        let predecessor_account_id = env::predecessor_account_id();
        assert_eq!(&predecessor_account_id, &token.owner_id);
        if token
            .approved_account_ids
            .remove(&account_id)
            .is_some()
        {
            refund_approved_account_ids_iter(predecessor_account_id, [account_id.into()].iter());
            self.tokens_by_id.insert(&token_id, &token);
        }
    }

    #[payable]
    fn nft_revoke_all(&mut self, token_id: TokenId) {
        assert_one_yocto();
        let mut token = self.tokens_by_id.get(&token_id).expect("Token not found");
        let predecessor_account_id = env::predecessor_account_id();
        assert_eq!(&predecessor_account_id, &token.owner_id);
        if !token.approved_account_ids.is_empty() {
            refund_approved_account_ids(predecessor_account_id, &token.approved_account_ids);
            token.approved_account_ids.clear();
            self.tokens_by_id.insert(&token_id, &token);
        }
    }

    fn nft_total_supply(&self) -> U64 {
        self.token_metadata_by_id.len().into()
    }

    fn nft_token(&self, token_id: TokenId) -> Option<JsonToken> {
        if let Some(token) = self.tokens_by_id.get(&token_id) {
            let mut metadata = self.token_metadata_by_id.get(&token_id).unwrap();
			if metadata.title.is_none() {
				metadata.title = Some(token_id.clone());
			}
            Some(JsonToken {
                token_id,
                owner_id: token.owner_id,
                metadata,
                royalty: token.royalty,
                approved_account_ids: token.approved_account_ids,
                token_type: token.token_type,
            })
        } else {
            None
        }
    }
}

#[near_bindgen]
impl NonFungibleTokenResolver for Contract {
    #[private]
    fn nft_resolve_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        approved_account_ids: HashMap<AccountId, u64>,
        token_id: TokenId,
    ) -> bool {
        // Whether receiver wants to return token back to the sender, based on `nft_on_transfer`
        // call result.
        if let PromiseResult::Successful(value) = env::promise_result(0) {
            if let Ok(return_token) = near_sdk::serde_json::from_slice::<bool>(&value) {
                if !return_token {
                    // Token was successfully received.
                    refund_approved_account_ids(owner_id, &approved_account_ids);
                    return true;
                }
            }
        }

        let mut token = if let Some(token) = self.tokens_by_id.get(&token_id) {
            if token.owner_id != receiver_id {
                // The token is not owner by the receiver anymore. Can't return it.
                refund_approved_account_ids(owner_id, &approved_account_ids);
                return true;
            }
            token
        } else {
            // The token was burned and doesn't exist anymore.
            refund_approved_account_ids(owner_id, &approved_account_ids);
            return true;
        };

        log!("Return {} from @{} to @{}", token_id, receiver_id, owner_id);

        self.internal_remove_token_from_owner(&receiver_id, &token_id);
        self.internal_add_token_to_owner(&owner_id, &token_id);
        token.owner_id = owner_id;
        refund_approved_account_ids(receiver_id, &token.approved_account_ids);
        token.approved_account_ids = approved_account_ids;
        self.tokens_by_id.insert(&token_id, &token);

        false
    }
}
