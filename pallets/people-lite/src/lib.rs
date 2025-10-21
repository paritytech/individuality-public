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

//! People lite.
//!
//! Warning: This pallet must be configured with some spam prevention mechanism.
//! The spam prevention mechanism must limit how many calls the origin
//! `LitePerson` can do.
//! The recommended approach is to use pallet-origins-restriction.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod extension;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use extension::{PeopleLiteAuth, PeopleLiteAuthData};
pub use pallet::*;
pub use weights::WeightInfo;

use alloc::boxed::Box;
use frame_support::{dispatch::PostDispatchInfo, traits::IsSubType};
use sp_runtime::traits::{Dispatchable, IdentifyAccount, Verify};
use verifiable::GenerateVerifiable;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	pub const PROOF_OF_OWNERSHIP_PREFIX: &[u8; 18] = b"pop register using";

	pub type AttestationOf<T> =
		<<<T as Config>::AttestationSignature as Verify>::Signer as IdentifyAccount>::AccountId;
	pub type MemberOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Member;
	pub type SignatureOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Signature;

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	pub struct LitePersonInfo<Member> {
		pub ring_vrf_key: Member,
		pub special_key: [u8; 32],
	}
	pub type LitePersonInfoOf<T> = LitePersonInfo<MemberOf<T>>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config<
		RuntimeOrigin: From<Origin<Self>> + OriginTrait<PalletsOrigin: TryInto<Origin<Self>>>,
		RuntimeCall: IsSubType<Call<Self>> + Dispatchable<PostInfo = PostDispatchInfo>,
	>
	{
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// The origin that can issue quotas to verifiers.
		type AttestationAllowanceManager: EnsureOrigin<Self::RuntimeOrigin>;

		/// Trait allowing cryptographic proof of membership without exposing the underlying member.
		/// Normally a Ring-VRF.
		type Crypto: GenerateVerifiable<
			Proof: Send + Sync + DecodeWithMemTracking,
			Signature: Send + Sync + DecodeWithMemTracking,
			Member: DecodeWithMemTracking,
		>;

		/// Signature used to claim attestations.
		type AttestationSignature: Verify<Signer: IdentifyAccount<AccountId: Parameter + MaxEncodedLen + Send + Sync>>
			+ Parameter
			+ Send
			+ Sync;
	}

	#[pallet::origin]
	#[derive(
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebugNoBound,
		Encode,
		Decode,
		MaxEncodedLen,
		TypeInfo,
		DecodeWithMemTracking,
	)]
	#[scale_info(skip_type_params(T))]
	pub enum Origin<T: Config> {
		/// A person litely recognized.
		LitePerson(T::AccountId),
		LitePersonRegistration(T::AccountId),
	}

	#[pallet::storage]
	pub type LitePeople<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, LitePersonInfoOf<T>>;

	/// Number of attestations available to distribute for a verifier account id.
	#[pallet::storage]
	pub(crate) type AttestationAllowance<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// Unclaimed attestations for each verifier.
	///
	/// The verifier verified the person and registered the attestation, the attestation is not yet
	/// claimed by the person.
	///
	/// If the key `(verifier, attestation)` exists then the attestation is currently waiting to be
	/// claimed by the person.
	#[pallet::storage]
	pub(crate) type UnclaimedAttestations<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, AttestationOf<T>, ()>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// All attestation allowance has been removed for the verifier.
		AllAttestationAllowanceCleared { verifier: T::AccountId },
		/// Some attestation allowance has been removed for the verifier.
		AttestationAllowancePartiallyCleared { verifier: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No attestation allowance.
		NoAttestationAllowance,
		/// Attestation is already set.
		AttestationAlreadySet,
		/// No attestation found.
		NoAttestation,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Grant some attestation allowance to an account so they can attest people.
		///
		/// The origin must be `AttestationAllowanceManager`.
		///
		/// - `account`: The account to grant attestations to.
		/// - `count`: The number of attestations to grant.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::increase_attestation_allowance())]
		pub fn increase_attestation_allowance(
			origin: OriginFor<T>,
			account: T::AccountId,
			count: u32,
		) -> DispatchResult {
			T::AttestationAllowanceManager::ensure_origin(origin)?;

			let mut available = AttestationAllowance::<T>::get(&account);

			available = available.saturating_add(count);

			AttestationAllowance::<T>::insert(&account, available);

			Ok(())
		}

		/// Clear all attestation allowance for an account.
		///
		/// The origin must be `AttestationAllowanceManager`.
		///
		/// - `account`: The account to remove all attestations from.
		/// - `limit`: The maximum number of unclaimed attestations to remove.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::clear_attestation_allowance(*limit))]
		pub fn clear_attestation_allowance(
			origin: OriginFor<T>,
			account: T::AccountId,
			limit: u32,
		) -> DispatchResult {
			T::AttestationAllowanceManager::ensure_origin(origin)?;

			AttestationAllowance::<T>::remove(&account);
			let res = UnclaimedAttestations::<T>::clear_prefix(&account, limit, None);
			if res.maybe_cursor.is_some() {
				Self::deposit_event(Event::AttestationAllowancePartiallyCleared {
					verifier: account,
				});
			} else {
				Self::deposit_event(Event::AllAttestationAllowanceCleared { verifier: account });
			}

			Ok(())
		}

		/// Attest an account.
		///
		/// The origin must be signed by an account and have some attestation allowance left.
		///
		/// - `attestation`: The attestation to set.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::set_attestation())]
		pub fn set_attestation(
			origin: OriginFor<T>,
			attestation: AttestationOf<T>,
		) -> DispatchResult {
			let verifier = ensure_signed(origin)?;

			let mut available = AttestationAllowance::<T>::get(&verifier);
			available = available.checked_sub(1).ok_or(Error::<T>::NoAttestationAllowance)?;

			ensure!(
				!UnclaimedAttestations::<T>::contains_key(&verifier, &attestation),
				Error::<T>::AttestationAlreadySet
			);

			UnclaimedAttestations::<T>::insert(&verifier, &attestation, ());
			if available > 0 {
				AttestationAllowance::<T>::insert(&verifier, available);
			} else {
				AttestationAllowance::<T>::remove(&verifier);
			}

			Ok(())
		}

		/// Cancel an attestation.
		///
		/// The origin must be signed by the account that set the attestation to cancel.
		///
		/// - `attestation`: The attestation to cancel.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_attestation())]
		pub fn cancel_attestation(
			origin: OriginFor<T>,
			attestation: AttestationOf<T>,
		) -> DispatchResult {
			let verifier = ensure_signed(origin)?;

			UnclaimedAttestations::<T>::take(&verifier, &attestation)
				.ok_or(Error::<T>::NoAttestation)?;

			let mut available = AttestationAllowance::<T>::get(&verifier);
			available = available.saturating_add(1);
			AttestationAllowance::<T>::insert(&verifier, available);
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::register())]
		pub fn register(
			origin: OriginFor<T>,
			ring_vrf_key: MemberOf<T>,
			special_key: [u8; 32],
			_proof_of_ownership: <T::Crypto as GenerateVerifiable>::Signature,
			verifier: T::AccountId,
			attestation: AttestationOf<T>,
			_attestation_signature: T::AttestationSignature,
		) -> DispatchResult {
			let Ok(Origin::LitePersonRegistration(who)) = origin.clone().into_caller().try_into()
			else {
				return Err(DispatchError::BadOrigin.into());
			};

			UnclaimedAttestations::<T>::remove(verifier, attestation);
			LitePeople::<T>::insert(&who, LitePersonInfo { ring_vrf_key, special_key });
			frame_system::Pallet::<T>::inc_sufficients(&who);

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(
			<T as Config>::WeightInfo::dispatch_as_signer()
			.saturating_add(call.get_dispatch_info().call_weight)
		)]
		pub fn dispatch_as_signer(
			mut origin: OriginFor<T>,
			call: Box<T::RuntimeCall>,
		) -> DispatchResultWithPostInfo {
			let who = match origin.clone().into_caller().try_into() {
				Ok(Origin::LitePerson(account)) => account,
				_ => return Err(DispatchError::BadOrigin.into()),
			};

			origin.set_caller_from(frame_system::RawOrigin::Signed(who));

			call.dispatch(origin)
		}
	}
}
