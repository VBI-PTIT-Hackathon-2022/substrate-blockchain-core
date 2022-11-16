#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::{DispatchError, DispatchResult, result::Result}, ensure, log, pallet_prelude::*, traits::{Currency, Randomness}};
use frame_support::traits::{ExistenceRequirement, UnixTime};
use frame_system::{ensure_signed, pallet_prelude::*};
use lite_json::json_parser::parse_json;
use scale_info::prelude::string::String;
use sp_core::sr25519;
use sp_runtime::{AnySignature, SaturatedConversion, traits::{IdentifyAccount, Verify}};
use sp_runtime::traits::BlockNumberProvider;
pub use sp_std::{convert::Into, str};
pub use sp_std::vec;
pub use sp_std::vec::Vec;

use convert::*;
pub use order::Order;
pub use pallet::*;
use pallet_nft_currency::NonFungibleToken;
mod order;
mod convert;

/// An index to a block.
pub type BlockNumber = u32;

// Time is measured by number of blocks.
pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;
pub const MONTHS: BlockNumber = WEEKS * 4;

#[frame_support::pallet]
pub mod pallet {
	pub use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Currency: Currency<Self::AccountId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Timestamp: UnixTime;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
		type TokenNFT: NonFungibleToken<Self::AccountId>;
		type Signature: Verify<Signer=Self::PublicKey> + Encode + Decode + Parameter;
		type PublicKey: IdentifyAccount<AccountId=Self::PublicKey> + Encode + Decode + Parameter;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn borrowers)]
	// AccountId => List of borrowing with hash id
	pub(super) type Borrowers<T: Config> =
	StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Vec<u8>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn cancel_order)]
	// Hashing order => Detail of canceled order
	pub(super) type CancelOrder<T: Config> =
	StorageMap<_, Blake2_128Concat, Vec<u8>, Order, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn rental_info)]
	// Hash Id -> Renting Info
	pub(super) type RentalInfo<T: Config> =
	StorageMap<_, Blake2_128Concat, Vec<u8>, Order, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_rental)]
	// TokenId -> Hash info of order
	pub(super) type TokenRental<T: Config> =
	StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<u8>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn due_block)]
	// Record the block stop the rental
	pub(super) type DueBlock<T: Config> =
	StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<Vec<u8>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn repayment)]
	// Record the block stop the rental
	pub(super) type Repayment<T: Config> =
	StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<Vec<u8>>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		MatchOrder(T::AccountId, T::AccountId, Vec<u8>),
		CancelOrder(Vec<u8>),
		StopRenting(Vec<u8>, T::AccountId),
		ReturnAsset(T::AccountId, T::AccountId, Vec<u8>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		NotMatchToken,
		NotMatchLender,
		NotMatchBorrower,
		TimeOver,
		NotOwner,
		NotEnoughFee,
		NoneExist,
		SignatureVerifyError1,
		SignatureVerifyError2,
		NotCaller,
		AlreadyCanceled,
		NotOwnerOfOrder,
		NotQualified,
		TimeNotLongEnough,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: BlockNumberFor<T>) {
			if DueBlock::<T>::contains_key(_n) {
				for hash_id in Self::due_block(_n).into_iter() {
					let order = Self::rental_info(hash_id.clone()).unwrap();
					let lender: T::AccountId = convert_bytes_to_accountid(order.lender);
					let borrower: T::AccountId = convert_bytes_to_accountid(order.borrower);
					// transfer asset back to lender
					T::TokenNFT::transfer_custodian(borrower.clone(), lender.clone(), order.token.clone()).expect("Cannot transfer custodian");
					RentalInfo::<T>::remove(hash_id.clone());
					Borrowers::<T>::mutate(borrower.clone(), |orders| {
						orders.retain(|x| *x != hash_id);
					});
					Self::deposit_event(Event::ReturnAsset(borrower, lender, order.token));
				}
			}
		}
	}


	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(35_678_000)]
		pub fn create_rental(origin: OriginFor<T>, lender: T::AccountId, borrower: T::AccountId, message_left: Vec<u8>, signature_left: Vec<u8>, message_right: Vec<u8>, signature_right: Vec<u8>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			if caller == lender {
				Self::verify_signature(message_right.clone(), signature_right.clone(), &borrower)?;
			} else if caller == borrower {
				Self::verify_signature(message_left.clone(), signature_left.clone(), &lender)?;
			} else {
				return Err(DispatchError::CannotLookup);
			}
			let lender_bytes = account_to_bytes(&lender).unwrap();
			let borrower_bytes = account_to_bytes(&borrower).unwrap();
			let order_left = Self::parse_to_order(lender_bytes.clone(), [0u8; 32], &message_left).unwrap();
			let order_right = Self::parse_to_order(lender_bytes.clone(), borrower_bytes.clone(), &message_right).unwrap();
			ensure!(!CancelOrder::<T>::contains_key(order_left.clone().encode()) &&
				!CancelOrder::<T>::contains_key(order_right.clone().encode()),
				Error::<T>::AlreadyCanceled);
			let fulfilled_order = Self::match_order(order_left, order_right).unwrap();

			let hash_order = fulfilled_order.clone().encode();
			let token_id = fulfilled_order.clone().token;
			RentalInfo::<T>::mutate(hash_order.clone(), |order| {
				*order = Some(fulfilled_order.clone());
			});
			Borrowers::<T>::mutate(borrower.clone(), |orders| {
				orders.push(hash_order.clone());
			});
			TokenRental::<T>::mutate(token_id.clone(), |info| {
				*info = Some(hash_order.clone());
			});

			let due_block = Self::get_due_block(fulfilled_order.due_date);

			DueBlock::<T>::mutate(due_block.clone(), |orders| {
				orders.push(hash_order.clone());
			});

			Self::transfer_custodian(&lender, &borrower, fulfilled_order.clone());
			Self::deposit_event(Event::MatchOrder(lender, borrower, token_id));
			Ok(())
		}

		#[pallet::weight(35_678_000)]
		pub fn cancel_offer(origin: OriginFor<T>, message: Vec<u8>, is_lender: bool) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let account = account_to_bytes(&caller).unwrap();
			let order;
			if is_lender {
				order = Self::parse_to_order(account, [0u8; 32], &message).unwrap();
				ensure!(account == order.lender, Error::<T>::NotOwnerOfOrder)
			} else {
				order = Self::parse_to_order([0u8; 32], account, &message).unwrap();
				ensure!(account == order.borrower, Error::<T>::NotOwnerOfOrder)
			}
			CancelOrder::<T>::mutate(order.clone().encode(), |cancel_order| {
				*cancel_order = Some(order.clone());
			});
			Self::deposit_event(Event::CancelOrder(order.encode()));
			Ok(())
		}

		#[pallet::weight(35_678_000)]
		pub fn stop_renting(origin: OriginFor<T>, token_id: Vec<u8>) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let hash_id = Self::token_rental(token_id).unwrap();
			let order = Self::rental_info(hash_id).unwrap();
			let lender: T::AccountId = convert_bytes_to_accountid(order.lender);
			let borrower: T::AccountId = convert_bytes_to_accountid(order.borrower);

			// check the order to return token
			ensure!(caller == borrower.clone(), Error::<T>::NotMatchBorrower);
			ensure!(caller == T::TokenNFT::owner_of_token(order.token.clone()),Error::<T>::NotOwner);

			// transfer to the lender
			T::TokenNFT::transfer_custodian(borrower, lender, order.token).expect("Cannot transfer custodian");
			Ok(())
		}
	}
}

// helper functions
impl<T: Config> Pallet<T> {
	fn verify_signature(data: Vec<u8>, signature: Vec<u8>, who: &T::AccountId) -> Result<(), DispatchError> {
		// sr25519 always expects a 64 byte signature.
		let signature: AnySignature = sr25519::Signature::from_slice(signature.as_ref())
			.ok_or(Error::<T>::SignatureVerifyError1)?
			.into();

		// In Polkadot, the AccountId is always the same as the 32 byte public key.
		let account_bytes: [u8; 32] = account_to_bytes(who)?;
		let public_key = sr25519::Public::from_raw(account_bytes);

		// Check if everything is good or not.
		match signature.verify(data.as_slice(), &public_key) {
			true => Ok(()),
			false => Err(Error::<T>::SignatureVerifyError2)?,
		}
	}

	fn calculate_day_renting(due_date: u64) -> u64 {
		let part = due_date - T::Timestamp::now().as_secs();
		part / 24
	}

	/// Parse the json object to Order struct
	fn parse_to_order(lender: [u8; 32], borrower: [u8; 32], message: &Vec<u8>) -> Result<Order, DispatchError> {
		let data = str::from_utf8(message).unwrap();
		let order_data = parse_json(data).unwrap().to_object().unwrap();
		let mut order = Order {
			lender: [0u8; 32],
			borrower: [0u8; 32],
			fee: 0,
			token: vec![],
			due_date: 0,
			paid_type: 0,
		};

		for data in order_data.into_iter() {
			let key = data.0;
			let k = key.iter().map(|c| *c as u8).collect::<Vec<_>>();

			if k == "lender".as_bytes().to_vec() {
				let value = data.1.to_string().unwrap().iter().map(|c| *c as u8).collect::<Vec<_>>();
				let hex_account: T::AccountId = convert_string_to_accountid(&String::from_utf8(value.clone()).unwrap());
				let account: T::AccountId = convert_bytes_to_accountid(lender.clone());
				ensure!(hex_account == account, Error::<T>::NotMatchLender);
				order.lender = lender;
			} else if k == "borrower".as_bytes().to_vec() {
				let value = data.1.to_string().unwrap().iter().map(|c| *c as u8).collect::<Vec<_>>();
				let hex_account: T::AccountId = convert_string_to_accountid(&String::from_utf8(value.clone()).unwrap());
				let account: T::AccountId = convert_bytes_to_accountid(borrower.clone());
				ensure!(hex_account == account, Error::<T>::NotMatchBorrower);
				order.borrower = borrower;
			} else if k == "fee".as_bytes().to_vec() {
				let value = data.1.to_number().unwrap().integer;
				order.fee = value;
			} else if k == "token".as_bytes().to_vec() {
				let value = String::from_utf8(data.1.to_string().unwrap().iter().map(|c| *c as u8).collect::<Vec<_>>()).unwrap();
				let token = hex_string_to_vec(value);
				order.token = token;
			} else if k == "due_date".as_bytes().to_vec() {
				let value = data.1.to_number().unwrap().integer;
				ensure!(value > T::Timestamp::now().as_secs(), Error::<T>::TimeOver);
				order.due_date = value;
			} else if k == "paid_type".as_bytes().to_vec() {
				let value = data.1.to_number().unwrap().integer;
				ensure!(value>=0 && value <= 2, Error::<T>::TimeOver);
				order.paid_type = value.saturated_into();
			}
		}
		Ok(order)
	}

	fn match_order(order_left: Order, mut order_right: Order) -> Result<Order, DispatchError> {
		ensure!(order_left.token == order_right.token, Error::<T>::NotMatchToken);
		ensure!(order_left.lender == order_right.lender, Error::<T>::NotMatchLender);
		ensure!(order_left.due_date >= order_right.due_date, Error::<T>::TimeOver);
		ensure!(order_left.fee <= order_right.fee, Error::<T>::NotEnoughFee);
		let total_blocks = Self::calculate_day_renting(order_right.due_date)/DAYS;
		log::info!("total blocks: {}", total_blocks);
		ensure!(total_blocks > DAYS, Error::<T>::NotEnoughBlocks);

		let mut total_fee = 0;
		if order_left.paid_type == 0 {
			total_fee = order_right.fee * total_renting_days;
		} else if order_left.paid_type == 1 {
			total_fee = order_left.fee * (total_renting_days / 7);
		} else if order_left.paid_type == 2 {
			total_fee = order_left.fee * (total_renting_days / 30);
		}
		order_right.fee = total_fee;

		Ok(order_right)
	}

	fn transfer_custodian(lender: &T::AccountId, borrower: &T::AccountId, order: Order) {
		if order.paid_type == 0 {}
		let _ = T::TokenNFT::transfer_custodian(lender.clone(), borrower.clone(), order.token);
		let _ = T::Currency::transfer(&borrower, &lender, order.fee.saturated_into(), ExistenceRequirement::KeepAlive);
	}

	fn get_due_block(due_date: u64) -> T::BlockNumber {
		let current_block_number = frame_system::Pallet::<T>::current_block_number();
		let total_renting_days = Self::calculate_day_renting(due_date);
		let target_block = current_block_number + (total_renting_days / DAYS).into();
		target_block
	}

	fn get_repayment_block(order_type: u8, due_date:u64) {

	}
}


