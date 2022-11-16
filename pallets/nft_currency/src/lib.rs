#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::{dispatch::{DispatchError, DispatchResult, result::Result}, ensure, traits::{Get, Randomness}};
use frame_support::{pallet_prelude::{StorageMap, StorageValue}};
use frame_system::ensure_signed;
pub use sp_std::{convert::Into, vec::Vec};

pub use nft::NonFungibleToken;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;
pub mod nft;
#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::traits::Randomness;
	use frame_system::pallet_prelude::*;

	pub use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		// type Administrator : EnsureOrigin<Self::Origin>;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn token_uri)]
	// uri of the nft
	pub(super) type TokenUri<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<u8>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_tokens)]
	// total count of the token
	pub(super) type TotalTokens<T> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn owner_of)]
	// Mapping Token Id => Account Id: to check who is the owner of the token
	pub(super) type OwnerOf<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, T::AccountId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn custodian_of)]
	// Mapping Token Id => Account Id: to check who is the owner of the token
	pub(super) type CustodianOf<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, T::AccountId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn list_owned)]
	// To check all the token that the account owns
	pub(super) type ListOwned<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Vec<u8>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn is_approve_for_all)]
	// To check all the token that the account owns;
	// (from,to) => bool
	pub(super) type Approval<T: Config> = StorageMap<_, Blake2_128Concat, (T::AccountId, T::AccountId), bool, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_approval)]
	pub(super) type TokenApproval<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<T::AccountId>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		Mint(T::AccountId, Vec<u8>),
		Transfer(T::AccountId, T::AccountId, Vec<u8>),
		SetURI(Vec<u8>,Vec<u8>),
		Approve(T::AccountId, T::AccountId, Vec<u8>),
		ApproveForAll(T::AccountId, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		NoneValue,
		StorageOverflow,
		Invalid,
		NotOwner,
		NoneExist,
		NotOwnerNorApproved,
		NotCustodian,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(33_963_000 + T::DbWeight::get().reads_writes(4, 3))]
		pub fn mint_to(_origin: OriginFor<T>, to: T::AccountId) -> DispatchResult {
			let token_id = <Self as NonFungibleToken<_>>::mint(to.clone())?;
			Self::deposit_event(Event::Mint(to, token_id));
			Ok(())
		}

		#[pallet::weight(35_678_000 + T::DbWeight::get().reads_writes(3, 3))]
		pub fn transfer_ownership(origin: OriginFor<T>, to: T::AccountId, token_id: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(who == Self::owner_of(token_id.clone()).unwrap(), Error::<T>::NotOwner);
			<Self as NonFungibleToken<_>>::transfer_ownership(who.clone(), to.clone(), token_id.clone()).expect("Cannot transfer token");
			Self::deposit_event(Event::Transfer(who, to, token_id));
			Ok(())
		}

		#[pallet::weight(54_275_000 + T::DbWeight::get().reads_writes(4, 3))]
		pub fn safe_transfer_ownership(origin: OriginFor<T>, from: T::AccountId, to: T::AccountId, token_id: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let account = (from.clone(), who.clone());
			ensure!(who == Self::owner_of(token_id.clone()).unwrap() ||
				Self::is_approve_for_all(account).unwrap(),
				Error::<T>::NotOwnerNorApproved);

			<Self as NonFungibleToken<_>>::transfer_ownership(from.clone(), to.clone(), token_id.clone())?;
			Self::deposit_event(Event::Transfer(from, to, token_id));
			Ok(())
		}

		#[pallet::weight(38_030_000 + T::DbWeight::get().reads_writes(2, 1))]
		pub fn approve(origin: OriginFor<T>, to: T::AccountId, token_id: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(who == Self::owner_of(token_id.clone()).unwrap()|| who == Self::custodian_of(token_id.clone()).unwrap(),Error::<T>::NotOwner);
			<Self as NonFungibleToken<_>>::approve(who.clone(), to.clone(), token_id.clone())?;
			Self::deposit_event(Event::Approve(who, to, token_id));
			Ok(())
		}

		#[pallet::weight(26_615_000 + T::DbWeight::get().reads_writes(1, 1))]
		pub fn approve_for_all(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as NonFungibleToken<_>>::set_approve_for_all(who.clone(), account.clone())?;
			Self::deposit_event(Event::ApproveForAll(who, account));
			Ok(())
		}

		#[pallet::weight(17_653_000 + T::DbWeight::get().reads_writes(2, 1))]
		pub fn set_token_uri(origin: OriginFor<T>, token_id: Vec<u8>, token_uri: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(who == Self::owner_of(token_id.clone()).unwrap(),Error::<T>::NotOwner);
			<Self as NonFungibleToken<_>>::set_token_uri(token_id.clone(), token_uri.clone())?;
			Self::deposit_event(Event::SetURI(token_id,token_uri));
			Ok(())
		}
	}
}

// helper functions
impl<T: Config> Pallet<T> {
	fn gen_token_id() -> Vec<u8> {
		let nonce = TotalTokens::<T>::get();
		let n = nonce.encode();
		let (rand, _) = T::Randomness::random(&n);
		rand.encode()
	}
}


impl<T: Config> NonFungibleToken<T::AccountId> for Pallet<T> {
	fn token_uri(token_id: Vec<u8>) -> Vec<u8> {
		TokenUri::<T>::get(token_id).unwrap()
	}

	fn custodian_of_token(token_id: Vec<u8>) -> T::AccountId {
		let account = CustodianOf::<T>::get(token_id).unwrap();
		account
	}

	fn owner_of_token(token_id: Vec<u8>) -> T::AccountId {
		let account = OwnerOf::<T>::get(token_id).unwrap();
		account
	}

	fn mint(owner: T::AccountId) -> Result<Vec<u8>, DispatchError> {
		let token_id = Self::gen_token_id();
		TotalTokens::<T>::mutate(|value| *value += 1);
		OwnerOf::<T>::mutate(token_id.clone(), |account| {
			*account = Some(owner.clone());
		});
		ListOwned::<T>::mutate(owner.clone(), |list_token| {
			list_token.push(token_id.clone());
		});

		TokenApproval::<T>::mutate(token_id.clone(), |approval| {
			approval.push(owner);
		});

		Ok(token_id)
	}

	fn transfer_ownership(from: T::AccountId, to: T::AccountId, token_id: Vec<u8>) -> DispatchResult {
		OwnerOf::<T>::mutate(token_id.clone(), |owner| *owner = Some(to.clone()));
		ListOwned::<T>::mutate(to, |list_token| {
			list_token.push(token_id.clone());
		});
		ListOwned::<T>::mutate(from, |list_token| {
			if let Some(ind) = list_token.iter().position(|id| *id == token_id) {
				list_token.swap_remove(ind);
				return Ok(());
			}
			Err(())
		}).expect("Error in ListOwned");
		Ok(())
	}

	fn transfer_custodian(from: T::AccountId, to: T::AccountId, token_id: Vec<u8>) -> DispatchResult {
		ensure!(Self::owner_of(token_id.clone()).unwrap() == from || Self::custodian_of(token_id.clone()).unwrap() == from ,Error::<T>::NotCustodian);
		if to == Self::owner_of_token(token_id.clone()) {
			CustodianOf::<T>::remove(token_id.clone());
		} else {
			CustodianOf::<T>::mutate(token_id.clone(), |custodian| *custodian = Some(to));
		}
		Ok(())
	}

	fn is_approve_for_all(account_approve: (T::AccountId, T::AccountId)) -> bool {
		Approval::<T>::get(account_approve).unwrap()
	}

	fn approve(from: T::AccountId, to: T::AccountId, token_id: Vec<u8>) -> DispatchResult {
		let owner = OwnerOf::<T>::get(token_id.clone()).unwrap();
		ensure!(from==owner, "Not Owner nor approved");
		TokenApproval::<T>::mutate(token_id.clone(), |list_account| {
			list_account.push(to);
		});
		Ok(())
	}

	fn set_approve_for_all(from: T::AccountId, to: T::AccountId) -> DispatchResult {
		let account = (from, to);
		Approval::<T>::mutate(account, |approved| {
			*approved = Some(true);
		});
		Ok(())
	}

	fn set_token_uri(token_id: Vec<u8>, token_uri: Vec<u8>) -> DispatchResult {
		TokenUri::<T>::mutate(token_id, |uri| *uri = Some(token_uri));
		Ok(())
	}
}
