use frame_support::{pallet_prelude::*};
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, PartialEq, TypeInfo, Debug)]
#[scale_info(skip_type_params(T))]
pub struct Order {
	//pub(crate) id:u64,
	pub(crate) lender: [u8; 32],
	pub(crate) borrower: [u8; 32],
	pub(crate) fee: u64,
	pub(crate) token: Vec<u8>,
	pub(crate) due_date: u64,
	pub(crate) paid_type: u8, // per day :0, per week: 1, per month:2
}


