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
	pub(crate) fee: u64,
	pub(crate) token: Vec<u8>,
	pub(crate) trading_type: u8, // at once :0, per day: 1, per week:2
}

impl Order {
	pub fn new() -> Self {
		Self {
			seller: [0u8; 32],
			buyer: [0u8; 32],
			fee: 0,
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
