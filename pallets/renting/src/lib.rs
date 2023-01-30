#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{ExistenceRequirement, UnixTime};
use frame_support::{
	dispatch::{result::Result, DispatchError, DispatchResult},
	ensure, log,
	pallet_prelude::*,
	traits::{Currency, Randomness},
};
use frame_system::{ensure_signed, pallet_prelude::*};
use lite_json::json_parser::parse_json;
use scale_info::prelude::string::String;
use sp_core::sr25519;
use sp_runtime::traits::BlockNumberProvider;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	AnySignature, SaturatedConversion,
};
pub use sp_std::vec;
pub use sp_std::vec::Vec;
pub use sp_std::{convert::Into, str};

use convert::*;
pub use order::Order;
pub use pallet::*;
use pallet_collectible::NonFungibleToken;
mod convert;
mod order;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

/// An index to a block.

// Time is measured by number of blocks.
pub const MILLISECS_PER_BLOCK: u32 = 6000;
pub const MINUTES: u32 = 60000 / (MILLISECS_PER_BLOCK);
pub const HOURS: u32 = MINUTES * 60;
pub const DAYS: u32 = MINUTES * 3;
pub const WEEKS: u32 = DAYS * 7;
pub const MONTHS: u32 = WEEKS * 4;

#[frame_support::pallet]
pub mod pallet {
	pub use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Currency: Currency<Self::AccountId>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type Timestamp: UnixTime;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
		type TokenNFT: NonFungibleToken<Self::AccountId>;
		type Signature: Verify<Signer = Self::PublicKey> + Encode + Decode + Parameter;
		type PublicKey: IdentifyAccount<AccountId = Self::PublicKey> + Encode + Decode + Parameter;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn borrowers)]
	// AccountId, token Id => Order detail
	pub(super) type Borrowers<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		Vec<u8>,
		Order,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn cancel_orders)]
	// Hashing order => Detail of canceled order
	pub(super) type CancelOrder<T: Config> =
		StorageMap<_, Blake2_128Concat, Vec<u8>, Order, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn due_block)]
	// Record the block stop the rental
	pub(super) type DueBlock<T: Config> =
		StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<Order>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn repayment)]
	// Record the block to pay for the rental
	pub(super) type Repayment<T: Config> =
		StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<Order>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		MatchOrder(T::AccountId, T::AccountId, Vec<u8>),
		CancelOrder(Vec<u8>, T::AccountId, bool),
		StopRenting(Vec<u8>, T::AccountId),
		ReturnAsset(T::AccountId, T::AccountId, Vec<u8>),
		RepaymentRental(T::AccountId, T::AccountId, Vec<u8>),
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
		NotPaidType,
		TimeNotLongEnough,
		CannotTransferCustodian,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: BlockNumberFor<T>) {
			if DueBlock::<T>::contains_key(_n) {
				for order in Self::due_block(_n).into_iter() {
					let lender: T::AccountId = convert_bytes_to_accountid(order.lender);
					let borrower: T::AccountId = convert_bytes_to_accountid(order.borrower);
					// transfer asset back to lender
					T::TokenNFT::transfer_custodian(
						borrower.clone(),
						lender.clone(),
						order.token.clone(),
					)
					.expect("Cannot transfer custodian");

					Borrowers::<T>::remove(borrower.clone(), order.token.clone());

					Self::deposit_event(Event::ReturnAsset(borrower, lender, order.token));
				}
				DueBlock::<T>::remove(_n);
			}
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			if Repayment::<T>::contains_key(_n) {
				for order in Self::repayment(_n).into_iter() {
					let lender: T::AccountId = convert_bytes_to_accountid(order.lender);
					let borrower: T::AccountId = convert_bytes_to_accountid(order.borrower);
					if Borrowers::<T>::try_get(borrower.clone(), order.token.clone()).is_err() {
						continue;
					}
					if T::Currency::transfer(
						&borrower,
						&lender,
						order.fee.saturated_into(),
						ExistenceRequirement::KeepAlive,
					)
					.is_err()
					{
						T::TokenNFT::transfer_custodian(
							borrower.clone(),
							lender.clone(),
							order.token.clone(),
						)
						.expect("Cannot transfer custodian");
						Self::deposit_event(Event::ReturnAsset(
							borrower.clone(),
							lender.clone(),
							order.token.clone(),
						));
						Borrowers::<T>::remove(borrower.clone(), order.token.clone());
					} else {
						Self::deposit_event(Event::RepaymentRental(
							borrower.clone(),
							lender.clone(),
							order.token.clone(),
						));
					}
				}
				Repayment::<T>::remove(_n);
			}
			T::DbWeight::get().reads_writes(16, 15)
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(35_678_000)]
		pub fn create_rental(
			origin: OriginFor<T>,
			lender: T::AccountId,
			borrower: T::AccountId,
			message_left: Vec<u8>,
			signature_left: Vec<u8>,
			message_right: Vec<u8>,
			signature_right: Vec<u8>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			if caller == lender {
				Self::verify_signature(message_right.clone(), signature_right.clone(), &borrower)?;
			} else if caller == borrower {
				Self::verify_signature(message_left.clone(), signature_left.clone(), &lender)?;
			} else {
				return Err(DispatchError::CannotLookup);
			}
			let lender_bytes = account_to_bytes(&lender)?;
			let borrower_bytes = account_to_bytes(&borrower)?;
			let order_left = Self::parse_to_order(lender_bytes.clone(), [0u8; 32], &message_left)?;
			let order_right =
				Self::parse_to_order(lender_bytes.clone(), borrower_bytes.clone(), &message_right)?;
			ensure!(
				!CancelOrder::<T>::contains_key(order_left.clone().encode())
					&& !CancelOrder::<T>::contains_key(order_right.clone().encode()),
				Error::<T>::AlreadyCanceled
			);
			let mut fulfilled_order;
			if caller == lender {
				fulfilled_order = Self::match_order(lender.clone(), true, order_left, order_right)?;
			} else {
				fulfilled_order = Self::match_order(lender.clone(),false, order_left, order_right)?;
			}


			let token_id = fulfilled_order.token.clone();

			Borrowers::<T>::mutate(borrower.clone(), token_id.clone(), |order_detail| {
				*order_detail = fulfilled_order.clone();
			});

			let due_block = Self::get_due_block(fulfilled_order.clone());
			DueBlock::<T>::mutate(due_block.clone(), |orders| {
				orders.push(fulfilled_order.clone());
			});

			Self::transfer_custodian(&lender, &borrower, fulfilled_order.clone())?;
			Self::deposit_event(Event::MatchOrder(lender, borrower, token_id));
			Ok(())
		}

		#[pallet::weight(35_678_000)]
		pub fn cancel_order(
			origin: OriginFor<T>,
			message: Vec<u8>,
			is_lender: bool,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let account = account_to_bytes(&caller)?;
			let order;
			if is_lender {
				order = Self::parse_to_order(account, [0u8; 32], &message)?;
				ensure!(account == order.lender, Error::<T>::NotOwnerOfOrder);
			} else {
				order = Self::parse_to_order([0u8; 32], account, &message)?;
				ensure!(account == order.borrower, Error::<T>::NotOwnerOfOrder);
			}
			CancelOrder::<T>::mutate(order.clone().encode(), |cancel_order| {
				*cancel_order = Some(order.clone());
			});
			Self::deposit_event(Event::CancelOrder(message, caller,is_lender));
			Ok(())
		}

		/// Borrower stop renting NFT, the fee cannot refund
		#[pallet::weight(35_678_000)]
		pub fn stop_renting(origin: OriginFor<T>, token_id: Vec<u8>) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let order = Self::borrowers(caller.clone(), token_id.clone());
			let lender: T::AccountId = convert_bytes_to_accountid(order.lender);
			let borrower: T::AccountId = convert_bytes_to_accountid(order.borrower);

			// check the order to return token
			ensure!(caller == borrower.clone(), Error::<T>::NotMatchBorrower);
			ensure!(
				caller == T::TokenNFT::custodian_of_token(order.token.clone()),
				Error::<T>::NotOwner
			);

			// transfer to the lender
			T::TokenNFT::transfer_custodian(borrower.clone(), lender, order.token.clone())
				.expect("Cannot transfer custodian");
			Borrowers::<T>::remove(borrower.clone(), order.token.clone());
			// update storage
			Self::deposit_event(Event::StopRenting(token_id, caller));
			Ok(())
		}
	}
}

// helper functions
impl<T: Config> Pallet<T> {
	fn verify_signature(
		data: Vec<u8>,
		signature: Vec<u8>,
		who: &T::AccountId,
	) -> Result<(), DispatchError> {
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

	/// Parse the json object to Order struct
	fn parse_to_order(
		lender: [u8; 32],
		borrower: [u8; 32],
		message: &Vec<u8>,
	) -> Result<Order, DispatchError> {
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
				let value =
					data.1.to_string().unwrap().iter().map(|c| *c as u8).collect::<Vec<_>>();
				let hex_account: T::AccountId =
					convert_string_to_accountid(&String::from_utf8(value.clone()).unwrap());
				let account: T::AccountId = convert_bytes_to_accountid(lender.clone());
				ensure!(hex_account == account, Error::<T>::NotMatchLender);
				order.lender = lender;
			} else if k == "borrower".as_bytes().to_vec() && borrower != [0u8;32] {
				let value =
					data.1.to_string().unwrap().iter().map(|c| *c as u8).collect::<Vec<_>>();
				let hex_account: T::AccountId =
					convert_string_to_accountid(&String::from_utf8(value.clone()).unwrap());
				let account: T::AccountId = convert_bytes_to_accountid(borrower.clone());
				ensure!(hex_account == account, Error::<T>::NotMatchBorrower);
				order.borrower = borrower;
			} else if k == "fee".as_bytes().to_vec() {
				let value = data.1.to_number().unwrap().integer;
				order.fee = value;
			} else if k == "token".as_bytes().to_vec() {
				let value = String::from_utf8(
					data.1.to_string().unwrap().iter().map(|c| *c as u8).collect::<Vec<_>>(),
				)
				.unwrap();
				let token = hex_string_to_vec(value);
				order.token = token;
			} else if k == "due_date".as_bytes().to_vec() {
				let value = data.1.to_number().unwrap().integer;
				ensure!(value > T::Timestamp::now().as_secs(), Error::<T>::TimeOver);
				order.due_date = value;
			} else if k == "paid_type".as_bytes().to_vec() {
				let value = data.1.to_number().unwrap().integer;
				ensure!(value <= 2, Error::<T>::NotPaidType);
				order.paid_type = value.saturated_into();
			}
		}
		Ok(order)
	}

	fn match_order(
		lender: T::AccountId,
		is_lender : bool,
		order_left: Order,
		mut order_right: Order,
	) -> Result<Order, DispatchError> {
		ensure!(order_left.token == order_right.token, Error::<T>::NotMatchToken);
		ensure!(order_left.lender == order_right.lender, Error::<T>::NotMatchLender);
		ensure!(order_left.due_date >= order_right.due_date, Error::<T>::TimeOver);
		ensure!(is_lender || order_left.fee <= order_right.fee, Error::<T>::NotEnoughFee);

		let order = order_right.clone();
		ensure!(
			Self::check_borrowers(lender, order.token, order.due_date),
			Error::<T>::NotQualified
		);

		let total_renting_days = Self::calculate_day_renting(order_right.due_date);
		ensure!(total_renting_days > 1, Error::<T>::TimeNotLongEnough);

		if order_right.paid_type == 0 {
			order_right.fee = order_right.fee * total_renting_days;
		} else if order_right.paid_type == 1 {
			order_right.fee = order_right.fee;
		} else if order_right.paid_type == 2 {
			order_right.fee = order_right.fee * 7;
		}

		Ok(order_right)
	}

	fn transfer_custodian(
		lender: &T::AccountId,
		borrower: &T::AccountId,
		order: Order,
	) -> DispatchResult {
		ensure!(
			!T::TokenNFT::transfer_custodian(lender.clone(), borrower.clone(), order.token.clone())
				.is_err(),
			Error::<T>::CannotTransferCustodian
		);
		let _ = T::Currency::transfer(
			&borrower,
			&lender,
			order.fee.saturated_into(),
			ExistenceRequirement::KeepAlive,
		)
		.unwrap();
		Ok(())
	}

	fn calculate_day_renting(due_date: u64) -> u64 {
		let part = due_date - T::Timestamp::now().as_secs();
		part / 86400
	}

	fn get_due_block(order: Order) -> T::BlockNumber {
		let mut current_block_number = frame_system::Pallet::<T>::current_block_number();
		let total_renting_days = Self::calculate_day_renting(order.due_date) as u32;
		log::info!("total_renting_days: {}", total_renting_days);
		let target_block = current_block_number + (total_renting_days * DAYS).into();

		if order.paid_type == 1 {
			loop {
				current_block_number += DAYS.into();
				Repayment::<T>::mutate(current_block_number.clone(), |orders| {
					orders.push(order.clone())
				});
				log::info!("block {:?}",current_block_number);
				if current_block_number >= target_block {
					break;
				}
			}
		} else if order.paid_type == 2 {
			loop {
				current_block_number += WEEKS.into();
				Repayment::<T>::mutate(current_block_number.clone(), |orders| {
					orders.push(order.clone())
				});
				log::info!("block {:?}",current_block_number);
				if current_block_number >= target_block {
					break;
				}
			}
		}
		target_block
	}

	fn check_borrowers(user: T::AccountId, token_id: Vec<u8>, check_date: u64) -> bool {
		if !(Self::borrowers(user.clone(), token_id.clone()).lender == [0u8; 32]) {
			let order = Self::borrowers(user, token_id);
			log::info!("Check date: {:?} {:?} ", order.due_date, check_date);
			return if order.due_date >= check_date { true } else { false };
		}
		return true;
	}
}
