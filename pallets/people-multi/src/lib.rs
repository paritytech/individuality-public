// This file is part of Substrate.

// Copyright (C) 2017-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Proof-of-Personhood system.

//! This system is used to anonymously track

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]
#![allow(clippy::borrowed_box)] // SHAWN TODO: Maybe fix this.
extern crate alloc;
use alloc::{boxed::Box, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod extension;
pub mod types;
pub mod weights;

use codec::{Decode, Encode, MaxEncodedLen};
use core::cmp::{self};
use frame_support::{
	dispatch::{
		extract_actual_weight, DispatchInfo, DispatchResultWithPostInfo, GetDispatchInfo,
		PostDispatchInfo,
	},
	traits::{EnsureOriginWithArg, IsSubType, OriginTrait},
};
use individuality_support::traits::{
	AddOnlyPeopleTrait, Context, ContextualAlias, PersonalId, RingIndex,
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{BadOrigin, Dispatchable, Zero},
	ArithmeticError, RuntimeDebug,
};
use verifiable::{Alias, GenerateVerifiable};

pub use pallet::*;
pub use types::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, traits::Contains};
	use frame_system::pallet_prelude::{BlockNumberFor, *};
	use sp_arithmetic::traits::Saturating;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<
		RuntimeOrigin: From<Origin>
		                   + From<<Self::RuntimeOrigin as OriginTrait>::PalletsOrigin>
		                   + OriginTrait<
			PalletsOrigin: From<Origin>
			                   + TryInto<
				Origin,
				Error = <Self::RuntimeOrigin as OriginTrait>::PalletsOrigin,
			>,
		>,
		RuntimeCall: Parameter
		                 + GetDispatchInfo
		                 + IsSubType<Call<Self>>
		                 + Dispatchable<
			RuntimeOrigin = Self::RuntimeOrigin,
			Info = DispatchInfo,
			PostInfo = PostDispatchInfo,
		>,
	>
	{
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// The runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Trait allowing cryptographic proof of membership without exposing the underlying member.
		/// Normally a Ring-VRF.
		type Crypto: GenerateVerifiable<Proof: Send + Sync, Signature: Send + Sync>;

		/// Contexts which may validly have an account alias behind it for everyone.
		type AccountContexts: Contains<Context>;

		/// Number of chunks per page.
		#[pallet::constant]
		type ChunkPageSize: Get<u32>;

		/// Maximum number of people included in a ring before a new one is created.
		#[pallet::constant]
		type MaxRingSize: Get<u32>;

		/// Maximum number of people queued before onboarding to a ring.
		#[pallet::constant]
		type OnboardingSize: Get<u32>;
	}

	/// The current individuals we recognise.
	#[pallet::storage]
	pub type Root<T> = StorageMap<_, Blake2_128Concat, RingIndex, RingRoot<T>>;

	/// Keeps track of the ring index currently being populated.
	#[pallet::storage]
	pub type CurrentRingIndex<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// These are the keys that will be placed into each ring index.
	#[pallet::storage]
	pub type RingKeys<T: Config> =
		StorageMap<_, Blake2_128Concat, RingIndex, KeysForRing<T>, ValueQuery>;

	/// This is a duplicate of the (total keys, included keys) found in Ring Keys for Nova Team <3.
	/// Not used in any pallet logic, but must be maintained and up to date.
	#[pallet::storage]
	pub type RingKeysMeta<T: Config> =
		StorageMap<_, Blake2_128Concat, RingIndex, (u32, u32), ValueQuery>;

	/// The current individuals we recognise in a specific ring: lookup from the crypto (public) key
	/// into the immutable ID of the individual.
	#[pallet::storage]
	pub type Keys<T> = CountedStorageMap<_, Blake2_128Concat, MemberOf<T>, PersonalId>;

	/// The current individuals we recognise: immutable ID of the individual into various
	/// information about their status.
	#[pallet::storage]
	pub type People<T> = StorageMap<_, Blake2_128Concat, PersonalId, PersonRecord<MemberOf<T>>>;

	/// Conversion of a contextual alias to an account ID.
	#[pallet::storage]
	pub type AliasToAccount<T> = StorageMap<
		_,
		Blake2_128Concat,
		ContextualAlias,
		<T as frame_system::Config>::AccountId,
		OptionQuery,
	>;

	/// Conversion of an account ID to a contextual alias.
	#[pallet::storage]
	pub type AccountToAlias<T> = StorageMap<
		_,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId,
		ContextualAlias,
		OptionQuery,
	>;

	/// Paginated collection of static chunks used by the verifiable crypto.
	#[pallet::storage]
	pub type Chunks<T> = StorageMap<
		_,
		Twox64Concat,
		PageIndex,
		BoundedVec<
			<<T as Config>::Crypto as GenerateVerifiable>::StaticChunk,
			<T as Config>::ChunkPageSize,
		>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An individual has had their personhood recognised and indexed.
		PersonhoodRecognized { who: PersonalId, key: MemberOf<T> },
		/// An individual has had their personhood recognised and indexed.
		PersonhoodResumed { who: PersonalId, key: MemberOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The supplied identifier does not represent a person.
		NotPerson,
		/// The given person has no associated key.
		NoKey,
		/// The context is not a member of those allowed to have account aliases held.
		InvalidContext,
		/// The account is not known.
		InvalidAccount,
		/// The account is already in use under another alias.
		AccountInUse,
		/// The proof is invalid.
		InvalidProof,
		/// The signature is invalid.
		InvalidSignature,
		/// There are not yet any members of our personhood set.
		NoMembers,
		/// The root cannot be finalized as there are still unpushed members.
		Incomplete,
		/// The root is still fresh.
		StillFresh,
		/// Too many members have been pushed.
		TooManyMembers,
		/// Key already in use by another person.
		KeyAlreadyInUse,
		/// The old key was not found when expected.
		KeyNotFound,
		/// Could not push member into the ring.
		CouldNotPush,
		/// The record is already using this key.
		SameKey,
	}

	#[pallet::origin]
	#[derive(Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
	pub enum Origin {
		PersonalIdentity(PersonalId),
		PersonalAlias(ContextualAlias),
	}

	// TODO: ValidateUnsigned or SignedExtension to allow signing with our crypto.
	// Will need proper consideration of fees & sequencing/replay.

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn integrity_test() {
			assert!(
				<T as Config>::ChunkPageSize::get() > 0,
				"chunk page size must hold at least one element"
			);
			assert!(<T as Config>::MaxRingSize::get() > 0, "rings must hold at least one person");
			assert!(
				<T as Config>::OnboardingSize::get() > 0,
				"onboarding size must be more than one person"
			);
			assert!(
				<T as Config>::OnboardingSize::get() <= <T as Config>::MaxRingSize::get(),
				"onboarding size must less than or equal to max ring size"
			);
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		// pub chunks: Vec<<<T as Config>::Crypto as GenerateVerifiable>::StaticChunk>,
		pub encoded_chunks: Vec<u8>,
		#[serde(skip)]
		pub _phantom_data: core::marker::PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			use individuality_support::genesis::{get_chunks_from_vrf_key, SMALL_VK};
			Self { encoded_chunks: get_chunks_from_vrf_key(SMALL_VK), _phantom_data: PhantomData }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let chunks: Vec<<<T as Config>::Crypto as GenerateVerifiable>::StaticChunk> =
				Decode::decode(&mut &(self.encoded_chunks.clone())[..])
					.expect("couldn't decode chunks");
			assert_eq!(chunks.len(), 1 << 9);
			let page_size = <T as Config>::ChunkPageSize::get();

			let mut page_idx = 0;
			let mut chunk_idx = 0;
			while chunk_idx < chunks.len() {
				let chunk_idx_end = cmp::min(chunk_idx + page_size as usize, chunks.len());
				let chunk_page: BoundedVec<
					<<T as Config>::Crypto as GenerateVerifiable>::StaticChunk,
					<T as Config>::ChunkPageSize,
				> = chunks[chunk_idx..chunk_idx_end]
					.to_vec()
					.try_into()
					.expect("page size was checked against the array length; qed");
				Chunks::<T>::insert(page_idx, chunk_page);
				page_idx += 1;
				chunk_idx = chunk_idx_end;
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Build a ring that is fully populated.
		///
		/// This is a task!!
		#[pallet::weight(Weight::zero())]
		#[pallet::call_index(100)]
		pub fn build_ring(_origin: OriginFor<T>, ring_index: RingIndex) -> DispatchResult {
			// Get the keys for this ring, and make sure that the ring is full before we build it.
			let mut keys = RingKeys::<T>::get(ring_index);
			let keys_len = keys.keys.len() as u32;
			let not_included_count = keys_len.saturating_sub(keys.included);

			// If everything is already included, nothing to do.
			ensure!(!not_included_count.is_zero(), Error::<T>::StillFresh);

			// Here we check we have enough items in the queue, and that we can support another
			// queue. TODO: Make this number configurable on how many we update at a time.
			let queue_full = not_included_count >= T::OnboardingSize::get() &&
				(T::MaxRingSize::get() - keys_len) >= T::OnboardingSize::get();

			let should_build = keys.keys.is_full() || queue_full;
			ensure!(should_build, Error::<T>::Incomplete);

			// Get the current ring, and check it should be rebuilt.
			// Return the next revision.
			let (next_revision, mut intermediate) =
				if let Some(existing_root) = Root::<T>::get(ring_index) {
					// We should build a new ring. Return the new revision number we should use.
					(
						existing_root.revision.checked_add(1).ok_or(ArithmeticError::Overflow)?,
						existing_root.intermediate,
					)
				} else {
					// No ring has been built at this index, so we start at revision 0.
					(0, T::Crypto::start_members())
				};

			// Push each member.
			for key in keys.keys.iter().skip(keys.included as usize) {
				let did_push =
					T::Crypto::push_member(&mut intermediate, key.clone(), |chunk_idx| {
						// TODO SHAWN: Is there some way for me to optimize this out of the
						// loop?
						let chunk_page_size = T::ChunkPageSize::get();
						Chunks::<T>::get(chunk_idx as u32 / chunk_page_size)
							.and_then(|page| {
								page.get(chunk_idx % (chunk_page_size as usize)).cloned()
							})
							.ok_or(())
					})
					.is_ok();
				ensure!(did_push, Error::<T>::CouldNotPush);
			}

			// By the end of the loop, we have included all the keys in the vector.
			keys.included = keys_len;

			// This is for the Nova Team <3
			RingKeysMeta::<T>::insert(ring_index, (keys_len, keys.included));

			// We create the root after pushing all members.
			let root = T::Crypto::finish_members(intermediate.clone());
			let ring_root = RingRoot { root, revision: next_revision, intermediate };
			Root::<T>::insert(ring_index, ring_root);
			RingKeys::<T>::insert(ring_index, keys);

			Ok(())
		}

		// NOTE: This might be removed in the future. Use the `AsPerson` transaction
		// extension to dispatch call with signature instead.
		// TODO: weight here is a bit complicated. see `pallet_utility::as_derivative` for hints.
		#[pallet::weight(T::WeightInfo::as_personal_identity())]
		#[pallet::call_index(1)]
		pub fn as_personal_identity(
			origin: OriginFor<T>,
			index: PersonalId,
			call: Box<T::RuntimeCall>,
			signature: <T::Crypto as GenerateVerifiable>::Signature,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin.clone())?;
			let record = People::<T>::get(index).ok_or(Error::<T>::NotPerson)?;
			let key = record.key;
			let ok = call.using_encoded(|msg| T::Crypto::verify_signature(&signature, msg, &key));
			ensure!(ok, Error::<T>::InvalidSignature);
			Self::derivative_call(origin, Origin::PersonalIdentity(index), *call)?;
			// TODO: This is how we handle tasks for now.
			Ok(Pays::No.into())
		}

		// NOTE: This might be removed in the future. Use the `AsPerson` transaction
		// extension to dispatch call with signature instead.
		/// Generally this should not be used since it does not protect against replay attacks. If
		/// used then it is important to use mortal transactions with a short lifespan.
		// TODO: weight here is a bit complicated. see `pallet_utility::as_derivative` for hints.
		#[pallet::weight(T::WeightInfo::as_personal_alias())]
		#[pallet::call_index(2)]
		pub fn as_personal_alias(
			origin: OriginFor<T>,
			context: Context,
			call: Box<T::RuntimeCall>,
			proof: <T::Crypto as GenerateVerifiable>::Proof,
			ring: RingIndex,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin.clone())?;
			let members = Root::<T>::get(ring).ok_or(Error::<T>::NoMembers)?.root;
			let alias = call
				.using_encoded(|msg| T::Crypto::validate(&proof, &members, &context, msg))
				.map_err(|_| Error::<T>::InvalidProof)?;
			let local_origin = Origin::PersonalAlias(ContextualAlias { alias, context });
			Self::derivative_call(origin, local_origin, *call)?;
			// TODO: This is how we handle tasks for now.
			Ok(Pays::No.into())
		}

		// TODO: these two and their associated storage/errors/events should be in an independent
		//   pallet in order to allow for use on other chains.

		// NOTE: This might be removed in the future. Use the `AsPerson` transaction
		// extension to dispatch call with signature instead.
		// Note this is not feeless even if the `call` is feeless. This is because we cannot
		// easily fetch the feelessness of `call` from within our feeless condition.
		// TODO: weight here is a bit complicated. see `pallet_utility::as_derivative` for hints.
		#[pallet::weight(T::WeightInfo::as_personal_alias())]
		#[pallet::call_index(3)]
		pub fn under_alias(
			origin: OriginFor<T>,
			call: Box<T::RuntimeCall>,
		) -> DispatchResultWithPostInfo {
			let account = ensure_signed(origin.clone())?;
			let ca = AccountToAlias::<T>::get(&account).ok_or(Error::<T>::InvalidAccount)?;
			let local_origin = Origin::PersonalAlias(ca);
			Self::derivative_call(origin, local_origin, *call)?;
			// TODO: This is how we handle tasks for now.
			Ok(Pays::No.into())
		}

		/// This transaction is refunded if successful and no alias was previously set.
		#[pallet::weight(Weight::zero())]
		#[pallet::call_index(4)]
		pub fn set_alias_account(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResultWithPostInfo {
			let ca = Self::ensure_personal_alias(origin)?;
			ensure!(T::AccountContexts::contains(&ca.context), Error::<T>::InvalidContext);
			ensure!(!AccountToAlias::<T>::contains_key(&account), Error::<T>::AccountInUse);
			let pays = if let Some(old_account) = AliasToAccount::<T>::get(&ca) {
				frame_system::Pallet::<T>::dec_sufficients(&old_account);
				AccountToAlias::<T>::remove(&old_account);
				Pays::Yes
			} else {
				Pays::No
			};
			frame_system::Pallet::<T>::inc_sufficients(&account);
			AccountToAlias::<T>::insert(&account, &ca);
			AliasToAccount::<T>::insert(&ca, &account);

			Ok(pays.into())
		}

		#[pallet::weight(Weight::zero())]
		#[pallet::call_index(5)]
		pub fn unset_alias_account(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let account = ensure_signed(origin)?;
			let alias = AccountToAlias::<T>::take(&account).ok_or(Error::<T>::InvalidAccount)?;

			AliasToAccount::<T>::remove(alias);
			frame_system::Pallet::<T>::dec_sufficients(&account);

			// TODO: This is how we handle tasks for now.
			Ok(Pays::No.into())
		}

		#[pallet::weight(Weight::zero())]
		#[pallet::call_index(6)]
		pub fn reset_root(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let _: Vec<_> = Root::<T>::drain().collect();
			let _: Vec<_> = Keys::<T>::drain().collect();
			let _: Vec<_> = People::<T>::drain().collect();
			let _: Vec<_> = AliasToAccount::<T>::drain().collect();
			let _: Vec<_> = AccountToAlias::<T>::drain().collect();

			Ok(Pays::No.into())
		}
	}

	impl<T: Config> Pallet<T> {
		fn derivative_call(
			mut origin: OriginFor<T>,
			local_origin: Origin,
			call: T::RuntimeCall,
		) -> DispatchResultWithPostInfo {
			origin.set_caller_from(<T::RuntimeOrigin as OriginTrait>::PalletsOrigin::from(
				local_origin,
			));
			let info = call.get_dispatch_info();
			let result = call.dispatch(origin);
			let weight = T::WeightInfo::as_personal_alias()
				.saturating_add(extract_actual_weight(&result, &info));
			result.map(|_| Some(weight).into()).map_err(|mut err| {
				err.post_info = Some(weight).into();
				err
			})
		}

		/// Ensure that the origin `o` represents a person.
		/// Returns `Ok` with the base identity of the person on success.
		pub fn ensure_personal_identity(
			origin: T::RuntimeOrigin,
		) -> Result<PersonalId, DispatchError> {
			Ok(ensure_personal_identity(origin.into_caller())?)
		}

		/// Ensure that the origin `o` represents a person.
		/// Returns `Ok` with the alias of the person together with the context in which it can
		/// be used on success.
		pub fn ensure_personal_alias(
			origin: T::RuntimeOrigin,
		) -> Result<ContextualAlias, DispatchError> {
			Ok(ensure_personal_alias(origin.into_caller())?)
		}

		// This function always returns the ring index and
		pub fn available_ring() -> (RingIndex, KeysForRing<T>) {
			let mut current_ring_index = CurrentRingIndex::<T>::get();
			let mut current_keys = RingKeys::<T>::get(current_ring_index);

			debug_assert!(
				!current_keys.keys.is_full(),
				"Something bad happened inside the STF, where the current keys are full, but we should have incremented in that case."
			);

			// This condition shouldn't be reached, but we handle the error just in case.
			if current_keys.keys.is_full() {
				current_ring_index.saturating_inc();
				CurrentRingIndex::<T>::put(current_ring_index);
				current_keys = RingKeys::<T>::get(current_ring_index);
			}

			debug_assert!(
				!current_keys.keys.is_full(),
				"Something bad happened inside the STF, where the current key and next key are both full. Nothing we can do here."
			);

			(current_ring_index, current_keys)
		}

		// This allows us to associate a key with a person.
		pub fn do_insert_key(who: PersonalId, key: MemberOf<T>) -> DispatchResult {
			// If the key is already in use by another person then error.
			ensure!(!Keys::<T>::contains_key(&key), Error::<T>::KeyAlreadyInUse);

			// Check if the person has already been onboarded.
			if let Some(old_record) = People::<T>::get(who) {
				// If already recognized with same key then nothing to do.
				ensure!(old_record.key != key, Error::<T>::SameKey);
			}

			let (mut current_ring_index, mut current_keys) = Self::available_ring();

			// This should also never happen, but we handle the error just in case.
			current_keys
				.keys
				.try_push(key.clone())
				.map_err(|_| Error::<T>::TooManyMembers)?;
			// Update the list of keys.
			RingKeys::<T>::insert(current_ring_index, &current_keys);
			let keys_len = current_keys.keys.len() as u32;

			let record = PersonRecord {
				key: key.clone(),
				ring_index: current_ring_index,
				key_index: keys_len,
			};
			Self::deposit_event(Event::<T>::PersonhoodRecognized { who, key });
			Keys::<T>::insert(&record.key, who);
			People::<T>::insert(who, &record);
			// For the Nova team <3
			RingKeysMeta::<T>::mutate(current_ring_index, |(total, _included): &mut (u32, u32)| {
				*total = keys_len;
			});

			// If the ring is full, we start building the next ring.
			if current_keys.keys.is_full() {
				current_ring_index.saturating_inc();
				CurrentRingIndex::<T>::put(current_ring_index);
			}

			Ok(())
		}
	}

	impl<T: Config> AddOnlyPeopleTrait for Pallet<T> {
		type Member = MemberOf<T>;
		type Signature = SignatureOf<T>;
		fn recognize_personhood(
			who: PersonalId,
			maybe_key: Option<MemberOf<T>>,
		) -> Result<(), DispatchError> {
			// If no key is being provided, then bail now.
			let Some(key) = maybe_key else { return Err(Error::<T>::NoKey.into()) };
			Self::do_insert_key(who, key)
		}

		fn verify_signature(signer: PersonalId, msg: &[u8], signature: &Self::Signature) -> bool {
			People::<T>::get(signer).map_or(false, |record| {
				<<T as Config>::Crypto as GenerateVerifiable>::verify_signature(
					signature,
					msg,
					&record.key,
				)
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		type Secret = PersonalId;

		#[cfg(feature = "runtime-benchmarks")]
		fn mock_key(_who: PersonalId) -> (Self::Member, Self::Secret) {
			// SHAWN TODO: implement this.
			unimplemented!()
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn mock_signature(
			_referrer: PersonalId,
			_msg: &[u8],
			_secret: &Self::Secret,
		) -> Self::Signature {
			// SHAWN TODO: implement this.
			unimplemented!()
		}
	}

	/// Ensure that the origin `o` represents a signed extrinsic (i.e. transaction).
	/// Returns `Ok` with the account that signed the extrinsic or an `Err` otherwise.
	pub fn ensure_personal_identity<OuterOrigin>(o: OuterOrigin) -> Result<PersonalId, BadOrigin>
	where
		OuterOrigin: TryInto<Origin, Error = OuterOrigin>,
	{
		match o.try_into() {
			Ok(Origin::PersonalIdentity(m)) => Ok(m),
			_ => Err(BadOrigin),
		}
	}

	/// Ensure that the origin `o` represents a signed extrinsic (i.e. transaction).
	/// Returns `Ok` with the account that signed the extrinsic or an `Err` otherwise.
	pub fn ensure_personal_alias<OuterOrigin>(o: OuterOrigin) -> Result<ContextualAlias, BadOrigin>
	where
		OuterOrigin: TryInto<Origin, Error = OuterOrigin>,
	{
		match o.try_into() {
			Ok(Origin::PersonalAlias(ca)) => Ok(ca),
			_ => Err(BadOrigin),
		}
	}

	/// Guard to ensure that the given origin is a person. The underlying identity of the person is
	/// provided on success.
	pub struct EnsurePersonalIdentity<T>(PhantomData<T>);
	impl<T: Config> EnsureOrigin<OriginFor<T>> for EnsurePersonalIdentity<T> {
		type Success = PersonalId;

		fn try_origin(o: OriginFor<T>) -> Result<Self::Success, OriginFor<T>> {
			ensure_personal_identity(o.clone().into_caller()).map_err(|_| o)
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<OriginFor<T>, ()> {
			todo!("account 0 that is always the first person");
		}
	}

	frame_support::impl_ensure_origin_with_arg_ignoring_arg! {
		impl<{ T: Config, A }>
			EnsureOriginWithArg< OriginFor<T>, A> for EnsurePersonalIdentity<T>
		{}
	}

	/// Guard to ensure that the given origin is a person. The contextual alias of the person is
	/// provided on success.
	pub struct EnsurePersonalAlias<T>(PhantomData<T>);
	impl<T: Config> EnsureOrigin<OriginFor<T>> for EnsurePersonalAlias<T> {
		type Success = ContextualAlias;

		fn try_origin(o: OriginFor<T>) -> Result<Self::Success, OriginFor<T>> {
			ensure_personal_alias(o.clone().into_caller()).map_err(|_| o)
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<OriginFor<T>, ()> {
			// Ok(Origin::PersonalAlias(ContextualAlias { alias: [0; 32], context: [0; 32]
			// }).into())
			todo!("alias [0; 32] with context [0; 32]");
		}
	}

	frame_support::impl_ensure_origin_with_arg_ignoring_arg! {
		impl<{ T: Config, A }>
			EnsureOriginWithArg< OriginFor<T>, A> for EnsurePersonalAlias<T>
		{}
	}

	/// Guard to ensure that the given origin is a person. The alias of the person within the
	/// context provided as an argument is returned on success.
	pub struct EnsurePersonalAliasInContext<T>(PhantomData<T>);
	impl<T: Config> EnsureOriginWithArg<OriginFor<T>, Context> for EnsurePersonalAliasInContext<T> {
		type Success = Alias;

		fn try_origin(o: OriginFor<T>, arg: &Context) -> Result<Self::Success, OriginFor<T>> {
			match ensure_personal_alias(o.clone().into_caller()) {
				Ok(ca) if &ca.context == arg => Ok(ca.alias),
				_ => Err(o),
			}
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin(context: &Context) -> Result<OriginFor<T>, ()> {
			Ok(Origin::PersonalAlias(ContextualAlias { alias: [0; 32], context: *context }).into())
		}
	}
}
