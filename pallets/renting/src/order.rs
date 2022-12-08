use codec::{DecodeLength, Error};
use frame_support::{pallet_prelude::*};
use frame_support::storage::StorageDecodeLength;
use sp_std::{vec::Vec,vec};

#[derive(Clone, Encode, Decode, PartialEq, TypeInfo, Debug)]
#[scale_info(skip_type_params(T))]
pub struct Order {
	//pub(crate) id:u64,
	pub(crate) lender: [u8; 32],
	pub(crate) borrower: [u8; 32],
	pub(crate) fee: u64,
	pub(crate) token: Vec<u8>,
	pub(crate) due_date: u64,
	pub(crate) paid_type: u8, // at once :0, per day: 1, per week:2
}

impl Order {
	pub fn new() -> Self {
		Self{
			lender: [0u8; 32],
			borrower: [0u8; 32],
			fee: 0,
			token: vec![],
			due_date: 0,
			paid_type: 0,
		}
	}
}

impl Default for Order {
	fn default() -> Self {
		Self::new()
	}
}
