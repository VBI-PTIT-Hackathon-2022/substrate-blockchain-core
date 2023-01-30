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


	#[pallet::storage]
	#[pallet::getter(fn cancel_orders)]
	// Hashing order => Detail of canceled order
	pub(super) type CancelOrder<T: Config> =
		StorageMap<_, Blake2_128Concat, Vec<u8>, Order, OptionQuery>;


	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		MatchOrder(T::AccountId, T::AccountId, Vec<u8>,u64),
		CancelOrder(Vec<u8>, T::AccountId, bool),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		NotMatchToken,
		NotMatchSeller,
		NotMatchBuyer,
		NotOwner,
		NotEnoughPrice,
		SignatureVerifyError1,
		SignatureVerifyError2,
		AlreadyCanceled,
		NotOwnerOfOrder,
		NotPaidType,
		CannotTransferOwnership,
	}


	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(35_678_000)]
		pub fn create_trading(
			origin: OriginFor<T>,
			seller: T::AccountId,
			buyer: T::AccountId,
			message_left: Vec<u8>,
			signature_left: Vec<u8>,
			message_right: Vec<u8>,
			signature_right: Vec<u8>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			if caller == seller {
				Self::verify_signature(message_right.clone(), signature_right.clone(), &buyer)?;
			} else if caller == buyer {
				Self::verify_signature(message_left.clone(), signature_left.clone(), &seller)?;
			} else {
				return Err(DispatchError::CannotLookup);
			}
			let seller_bytes = account_to_bytes(&seller)?;
			let buyer_bytes = account_to_bytes(&buyer)?;
			let order_left = Self::parse_to_order(seller_bytes.clone(), [0u8; 32], &message_left)?;
			let order_right =
				Self::parse_to_order(seller_bytes.clone(), buyer_bytes.clone(), &message_right)?;
			ensure!(
				!CancelOrder::<T>::contains_key(order_left.clone().encode())
					&& !CancelOrder::<T>::contains_key(order_right.clone().encode()),
				Error::<T>::AlreadyCanceled
			);
			let mut fulfilled_order;
			if caller == seller {
				fulfilled_order = Self::match_trading( true, order_left, order_right)?;
			} else {
				fulfilled_order = Self::match_trading( false, order_left, order_right)?;
			}

			Self::transfer_ownership(&seller, &buyer, fulfilled_order.clone())?;
			Self::deposit_event(Event::MatchOrder(seller, buyer, fulfilled_order.token.clone(), fulfilled_order.price));
			Ok(())
		}

		#[pallet::weight(35_678_000)]
		pub fn cancel_order(
			origin: OriginFor<T>,
			message: Vec<u8>,
			is_seller: bool,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let account = account_to_bytes(&caller)?;
			let order;
			if is_seller {
				order = Self::parse_to_order(account, [0u8; 32], &message)?;
				ensure!(account == order.seller, Error::<T>::NotOwnerOfOrder);
			} else {
				order = Self::parse_to_order([0u8; 32], account, &message)?;
				ensure!(account == order.buyer, Error::<T>::NotOwnerOfOrder);
			}
			CancelOrder::<T>::mutate(order.clone().encode(), |cancel_order| {
				*cancel_order = Some(order.clone());
			});
			Self::deposit_event(Event::CancelOrder(message, caller, is_seller));
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
		seller: [u8; 32],
		buyer: [u8; 32],
		message: &Vec<u8>,
	) -> Result<Order, DispatchError> {
		let data = str::from_utf8(message).unwrap();
		let order_data = parse_json(data).unwrap().to_object().unwrap();
		let mut order = Order {
			seller: [0u8; 32],
			buyer: [0u8; 32],
			price: 0,
			token: vec![],
			trading_type: 0,
		};

		for data in order_data.into_iter() {
			let key = data.0;
			let k = key.iter().map(|c| *c as u8).collect::<Vec<_>>();

			if k == "seller".as_bytes().to_vec() {
				let value =
					data.1.to_string().unwrap().iter().map(|c| *c as u8).collect::<Vec<_>>();
				let hex_account: T::AccountId =
					convert_string_to_accountid(&String::from_utf8(value.clone()).unwrap());
				let account: T::AccountId = convert_bytes_to_accountid(seller.clone());
				log::info!("Seller: {:?} {:?}",hex_account, account);
				ensure!(hex_account == account, Error::<T>::NotMatchSeller);
				order.seller = seller;
			} else if k == "buyer".as_bytes().to_vec() && buyer != [0u8;32] {
				let value =
					data.1.to_string().unwrap().iter().map(|c| *c as u8).collect::<Vec<_>>();
				let hex_account: T::AccountId =
					convert_string_to_accountid(&String::from_utf8(value.clone()).unwrap());
				let account: T::AccountId = convert_bytes_to_accountid(buyer.clone());
				log::info!("Buyer: {:?} {:?}",hex_account, account);
				ensure!(hex_account == account, Error::<T>::NotMatchBuyer);
				order.buyer = buyer;
			} else if k == "price".as_bytes().to_vec() {
				let value = data.1.to_number().unwrap().integer;
				order.price = value;
			} else if k == "token".as_bytes().to_vec() {
				let value = String::from_utf8(
					data.1.to_string().unwrap().iter().map(|c| *c as u8).collect::<Vec<_>>(),
				)
				.unwrap();
				let token = hex_string_to_vec(value);
				order.token = token;
			} else if k == "trading_type".as_bytes().to_vec() {
				let value = data.1.to_number().unwrap().integer;
				ensure!(value < 2, Error::<T>::NotPaidType);
				order.trading_type = value.saturated_into();
			}
		}
		Ok(order)
	}

	fn match_trading(
		is_seller:bool,
		order_left: Order,
		order_right: Order,
	) -> Result<Order, DispatchError> {
		ensure!(order_left.token == order_right.token, Error::<T>::NotMatchToken);
		ensure!(order_left.seller == order_right.seller, Error::<T>::NotMatchSeller);
		ensure!(is_seller || order_left.price <= order_right.price, Error::<T>::NotEnoughPrice);

		Ok(order_right)
	}

	fn transfer_ownership(
		seller: &T::AccountId,
		buyer: &T::AccountId,
		order: Order,
	) -> DispatchResult {
		ensure!(
			!T::TokenNFT::transfer_ownership(seller.clone(), buyer.clone(), order.token.clone())
				.is_err(),
			Error::<T>::CannotTransferOwnership
		);
		let _ = T::Currency::transfer(
			&buyer,
			&seller,
			order.price.saturated_into(),
			ExistenceRequirement::KeepAlive,
		)
		.unwrap();
		Ok(())
	}

}
