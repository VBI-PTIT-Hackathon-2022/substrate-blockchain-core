use codec::{DecodeLength, Error};
use frame_support::pallet_prelude::*;
use frame_support::storage::StorageDecodeLength;
use sp_std::{vec, vec::Vec};

#[derive(Clone, Encode, Decode, PartialEq, TypeInfo, Debug)]
#[scale_info(skip_type_params(T))]
pub struct Order {
	//pub(crate) id:u64,
	pub(crate) seller: [u8; 32],
	pub(crate) buyer: [u8; 32],
	pub(crate) price: u64,
	pub(crate) token: Vec<u8>,
	pub(crate) trading_type: u8, // buy now 0, offer from buyer 1, offer from seller 2
}

impl Order {
	pub fn new() -> Self {
		Self {
			seller: [0u8; 32],
			buyer: [0u8; 32],
			price: 0,
			token: vec![],
			trading_type: 0,
		}
	}
}

impl Default for Order {
	fn default() -> Self {
		Self::new()
	}
}
