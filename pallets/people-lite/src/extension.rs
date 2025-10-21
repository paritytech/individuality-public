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

//! Transaction extensions for pallet-people-lite.

use crate::*;
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{
	ensure, pallet_prelude::Weight, traits::OriginTrait, CloneNoBound, DebugNoBound, EqNoBound,
	PartialEqNoBound,
};
use frame_system::{CheckNonce, ValidNonceInfo};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{DispatchInfoOf, TransactionExtension, ValidateResult},
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidityError, ValidTransaction,
	},
};

/// Custom invalidity for invalid transactions in the `PeopleLiteAuth` transaction extension.
#[repr(u8)]
pub enum CustomError {
	/// The origin is not a signed origin.
	OriginNotSigned = 170,
	/// The signed origin is not a registered lite person.
	NotLitePerson = 171,
	/// The call is not the `register` call.
	CallNotRegister = 172,
	/// The signed origin is already registered as a lite person.
	AlreadyRegistered = 173,
	/// The proof of ownership of the ring VRF key is invalid.
	InvalidProofOfOwnership = 174,
	/// There is no unclaimed attestation for the given verifier and attestation.
	NoUnclaimedAttestation = 175,
	/// The attestation signature is invalid.
	InvalidAttestationSignature = 176,
}

impl From<CustomError> for TransactionValidityError {
	fn from(e: CustomError) -> Self {
		InvalidTransaction::Custom(e as u8).into()
	}
}

/// A type alias to access system runtime call.
type RuntimeCallOf<T> = <T as frame_system::Config>::RuntimeCall;

#[derive(Clone, Eq, PartialEq, Encode, Decode, TypeInfo, DecodeWithMemTracking, Debug)]
pub enum PeopleLiteAuthData<Nonce> {
	AsLitePerson { nonce: Nonce },
	AsLitePersonRegistration { nonce: Nonce },
}
#[allow(type_alias_bounds)]
pub type PeopleLiteAuthDataOf<T: Config> = PeopleLiteAuthData<T::Nonce>;

#[derive(
	CloneNoBound,
	EqNoBound,
	PartialEqNoBound,
	Encode,
	Decode,
	TypeInfo,
	DecodeWithMemTracking,
	DebugNoBound,
)]
#[scale_info(skip_type_params(T))]
pub struct PeopleLiteAuth<T: Config>(Option<PeopleLiteAuthDataOf<T>>);

impl<T: Config> PeopleLiteAuth<T> {
	/// Creates a new `PeopleLiteAuth` transaction extension.
	pub fn new(data: Option<PeopleLiteAuthDataOf<T>>) -> Self {
		Self(data)
	}
}

/// The value passed from validate to prepare in the [`PeopleLiteAuth`] transaction extension.
pub enum PeopleLiteAuthVal<AccountId, Nonce> {
	AsLitePerson(AccountId, Nonce),
	AsLitePersonRegistration(AccountId, Nonce),
	None,
}

/// The value passed from prepare to post dispatch in the [`PeopleLiteAuth`] transaction extension.
pub enum PeopleLiteAuthPre<AccountId> {
	AsLitePerson,
	AsLitePersonRegistration(AccountId),
	None,
}

impl<T: Config> TransactionExtension<RuntimeCallOf<T>> for PeopleLiteAuth<T> {
	const IDENTIFIER: &'static str = "PeopleLiteAuth";
	type Implicit = ();

	type Val = PeopleLiteAuthVal<T::AccountId, T::Nonce>;
	type Pre = PeopleLiteAuthPre<T::AccountId>;

	fn weight(&self, _call: &RuntimeCallOf<T>) -> Weight {
		match self.0 {
			Some(PeopleLiteAuthData::AsLitePerson { .. }) =>
				<T as Config>::WeightInfo::as_lite_person_tx_ext(),
			Some(PeopleLiteAuthData::AsLitePersonRegistration { .. }) =>
				<T as Config>::WeightInfo::as_lite_person_registration_tx_ext(),
			None => Weight::zero(),
		}
	}

	fn validate(
		&self,
		mut origin: T::RuntimeOrigin,
		call: &RuntimeCallOf<T>,
		_info: &DispatchInfoOf<RuntimeCallOf<T>>,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Encode,
		_source: TransactionSource,
	) -> ValidateResult<Self::Val, RuntimeCallOf<T>> {
		match self.0 {
			Some(PeopleLiteAuthData::AsLitePerson { nonce }) => {
				// Origin must be a signed origin.
				let Some(frame_system::Origin::<T>::Signed(who)) = origin.as_system_ref().cloned()
				else {
					return Err(CustomError::OriginNotSigned.into());
				};

				ensure!(LitePeople::<T>::contains_key(&who), CustomError::NotLitePerson);

				// Validate the nonce.
				let ValidNonceInfo { requires, provides } =
					CheckNonce::<T>::validate_nonce_for_account(&who, nonce)?;
				let validity = ValidTransaction { requires, provides, ..Default::default() };

				origin.set_caller_from(Origin::LitePerson(who.clone()));
				Ok((validity, PeopleLiteAuthVal::AsLitePerson(who, nonce), origin))
			},
			Some(PeopleLiteAuthData::AsLitePersonRegistration { nonce }) => {
				// Origin must be a signed origin.
				let Some(frame_system::Origin::<T>::Signed(who)) = origin.as_system_ref().cloned()
				else {
					return Err(CustomError::OriginNotSigned.into());
				};

				// Call must be a `sign_up_with_invite` call.
				let Some(Call::register {
					ring_vrf_key,
					special_key: _,
					proof_of_ownership,
					verifier,
					attestation,
					attestation_signature,
				}) = call.is_sub_type()
				else {
					return Err(CustomError::CallNotRegister.into());
				};

				// Validate the invite.
				UnclaimedAttestations::<T>::get(verifier, attestation)
					.ok_or(CustomError::NoUnclaimedAttestation)?;
				ensure!(
					who.using_encoded(|bytes| attestation_signature.verify(bytes, attestation)),
					CustomError::InvalidAttestationSignature,
				);

				// Verify the person is not already registered.
				ensure!(!LitePeople::<T>::contains_key(&who), CustomError::AlreadyRegistered);

				// Verify proof of ownership of the ring VRF key.
				let msg =
					who.using_encoded(|bytes| [&PROOF_OF_OWNERSHIP_PREFIX[..], bytes].concat());
				ensure!(
					T::Crypto::verify_signature(proof_of_ownership, &msg[..], ring_vrf_key),
					CustomError::InvalidProofOfOwnership,
				);

				// TODO: maybe verify the proof of ownership of the special key.

				frame_system::Pallet::<T>::inc_sufficients(&who);

				// Validate the nonce.
				let ValidNonceInfo { requires, provides } =
					CheckNonce::<T>::validate_nonce_for_account(&who, nonce)?;
				let validity = ValidTransaction { requires, provides, ..Default::default() };

				origin.set_caller_from(Origin::LitePersonRegistration(who.clone()));
				Ok((validity, PeopleLiteAuthVal::AsLitePersonRegistration(who, nonce), origin))
			},
			None => Ok((ValidTransaction::default(), PeopleLiteAuthVal::None, origin)),
		}
	}

	fn prepare(
		self,
		val: Self::Val,
		_origin: &T::RuntimeOrigin,
		_call: &RuntimeCallOf<T>,
		_info: &DispatchInfoOf<RuntimeCallOf<T>>,
		_len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		match val {
			PeopleLiteAuthVal::AsLitePerson(account, nonce) => {
				CheckNonce::<T>::prepare_nonce_for_account(&account, nonce)?;
				Ok(PeopleLiteAuthPre::AsLitePerson)
			},
			PeopleLiteAuthVal::AsLitePersonRegistration(account, nonce) => {
				CheckNonce::<T>::prepare_nonce_for_account(&account, nonce)?;

				Ok(PeopleLiteAuthPre::AsLitePersonRegistration(account))
			},
			PeopleLiteAuthVal::None => Ok(PeopleLiteAuthPre::None),
		}
	}

	fn post_dispatch_details(
		pre: Self::Pre,
		_info: &DispatchInfoOf<RuntimeCallOf<T>>,
		_post_info: &sp_runtime::traits::PostDispatchInfoOf<RuntimeCallOf<T>>,
		_len: usize,
		_result: &sp_runtime::DispatchResult,
	) -> Result<Weight, TransactionValidityError> {
		match pre {
			PeopleLiteAuthPre::AsLitePerson => (),
			PeopleLiteAuthPre::AsLitePersonRegistration(account) => {
				frame_system::Pallet::<T>::dec_sufficients(&account);
			},
			PeopleLiteAuthPre::None => (),
		}

		Ok(Weight::zero())
	}
}
