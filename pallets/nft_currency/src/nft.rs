use frame_support::{dispatch::{result::Result, DispatchError, DispatchResult}};
pub use sp_std::*;
use sp_std::vec::Vec;

pub trait NonFungibleToken<AccountId>{
	fn token_uri(token_id: Vec<u8>) -> Vec<u8>;
	fn custodian_of_token(token_id: Vec<u8>) -> AccountId;
	fn owner_of_token(token_id: Vec<u8>) -> AccountId;

	fn mint(owner:AccountId) -> Result<Vec<u8>,DispatchError>;
	fn transfer_ownership(from: AccountId, to: AccountId, token_id: Vec<u8>) -> DispatchResult;
	fn transfer_custodian(from: AccountId, to: AccountId, token_id: Vec<u8>) -> DispatchResult;
	fn set_token_uri(token_id: Vec<u8>, token_uri: Vec<u8>) -> DispatchResult;
	fn is_approve_for_all(account_approve:(AccountId,AccountId)) -> bool;
	fn approve(from: AccountId, to: AccountId,token_id: Vec<u8>) -> DispatchResult;
	fn set_approve_for_all(from: AccountId, to: AccountId) -> DispatchResult;
}
