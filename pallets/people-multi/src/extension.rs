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

//! People signed extensions.

use crate::*;
use codec::{Decode, Encode};
use core::fmt;
use frame_support::{
	ensure, pallet_prelude::TransactionSource, weights::Weight, CloneNoBound, DefaultNoBound,
	EqNoBound, PartialEqNoBound,
};
use individuality_support::{
	traits::Context,
	utils::{prepare_nonce_for_account, validate_nonce_for_account, ValidNonceInfo},
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{DispatchInfoOf, PostDispatchInfoOf, TransactionExtension, ValidateResult},
	transaction_validity::{InvalidTransaction, TransactionValidityError, ValidTransaction},
};

/// Information required to transform an origin into a personal alias or personal identity.
#[derive(Encode, Decode, TypeInfo, EqNoBound, CloneNoBound, PartialEqNoBound)]
#[scale_info(skip_type_params(T))]
pub enum AsPersonInfo<T: Config + Send + Sync> {
	/// The signed origin will be transformed using account to alias.
	AsPersonalAliasWithAccount(T::Nonce),
	/// The nonce origin will be transformed using proof.
	AsPersonalAliasWithProof(<T::Crypto as GenerateVerifiable>::Proof, RingIndex, Context),
	/// The nonce origin will be transformed using signature.
	AsPersonalIdentityWithProof(<T::Crypto as GenerateVerifiable>::Signature, PersonalId),
}

/// Transaction extension to transform an origin into a personal alias or personal identity.
#[derive(Encode, Decode, TypeInfo, EqNoBound, CloneNoBound, PartialEqNoBound, DefaultNoBound)]
#[scale_info(skip_type_params(T))]
pub struct AsPerson<T: Config + Send + Sync>(Option<AsPersonInfo<T>>);

impl<T: Config + Send + Sync> fmt::Debug for AsPerson<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "AsPerson")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
		Ok(())
	}
}

impl<T: Config + Send + Sync> AsPerson<T> {
	pub fn new(explicit: Option<AsPersonInfo<T>>) -> Self {
		Self(explicit)
	}

	fn weight() -> Weight {
		// TODO: actual weight.
		Weight::zero()
	}
}

/// Info returned by validate to prepare in the [`AsPerson`] transactinon extension.
pub enum Val<T: Config + Send + Sync> {
	NotUsing,
	UsingAliasWithAccount(T::AccountId, T::Nonce),
	UsingAliasWithProof,
	UsingIdentityWithProof,
}

impl<T: Config + Send + Sync> TransactionExtension<<T as frame_system::Config>::RuntimeCall>
	for AsPerson<T>
{
	const IDENTIFIER: &'static str = "AsPerson";
	type Implicit = ();

	type Val = Val<T>;

	// Is using tx ext
	type Pre = bool;

	fn weight(&self, _call: &<T as frame_system::Config>::RuntimeCall) -> Weight {
		Self::weight()
	}

	fn validate(
		&self,
		origin: <T as frame_system::Config>::RuntimeOrigin,
		_call: &<T as frame_system::Config>::RuntimeCall,
		_info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		_len: usize,
		_self_implicit: Self::Implicit,
		inherited_implication: &impl Encode,
		_source: TransactionSource,
	) -> ValidateResult<Self::Val, <T as frame_system::Config>::RuntimeCall> {
		match &self.0 {
			Some(AsPersonInfo::AsPersonalAliasWithAccount(nonce)) => {
				let Some(frame_system::Origin::<T>::Signed(who)) = origin.as_system_ref() else {
					return Err(InvalidTransaction::BadSigner.into());
				};
				let who = who.clone();

				let ca = AccountToAlias::<T>::get(&who).ok_or(InvalidTransaction::BadSigner)?;
				let local_origin = Origin::PersonalAlias(ca);
				let mut origin = origin;
				origin.set_caller_from(local_origin);

				let ValidNonceInfo { requires, provides } =
					validate_nonce_for_account::<T>(&who, *nonce)?;
				let validity = ValidTransaction { requires, provides, ..Default::default() };

				Ok((validity, Val::UsingAliasWithAccount(who, *nonce), origin))
			},
			Some(AsPersonInfo::AsPersonalAliasWithProof(proof, ring, context)) => {
				ensure!(
					matches!(origin.as_system_ref(), Some(frame_system::RawOrigin::None)),
					InvalidTransaction::BadSigner
				);

				let members =
					Root::<T>::get(ring).map(|m| m.root).ok_or(InvalidTransaction::Call)?;

				let msg = inherited_implication.using_encoded(sp_io::hashing::blake2_256);

				let alias = T::Crypto::validate(proof, &members, &context[..], &msg[..])
					.map_err(|_| InvalidTransaction::BadProof)?;

				let ca = ContextualAlias { alias, context: *context };
				let local_origin = Origin::PersonalAlias(ca);
				let mut origin = origin;
				origin.set_caller_from(local_origin);

				Ok((ValidTransaction::default(), Val::UsingAliasWithProof, origin))
			},
			Some(AsPersonInfo::AsPersonalIdentityWithProof(signature, index)) => {
				ensure!(
					matches!(origin.as_system_ref(), Some(frame_system::RawOrigin::None)),
					InvalidTransaction::BadSigner
				);

				let key = People::<T>::get(index)
					.map(|record| record.key)
					.ok_or(InvalidTransaction::BadSigner)?;

				let msg = inherited_implication.using_encoded(sp_io::hashing::blake2_256);

				if !T::Crypto::verify_signature(signature, &msg[..], &key) {
					return Err(InvalidTransaction::BadProof.into());
				}

				let local_origin = Origin::PersonalIdentity(*index);
				let mut origin = origin;
				origin.set_caller_from(local_origin);

				Ok((ValidTransaction::default(), Val::UsingIdentityWithProof, origin))
			},
			None => Ok((ValidTransaction::default(), Val::NotUsing, origin)),
		}
	}

	fn prepare(
		self,
		val: Self::Val,
		_origin: &<T as frame_system::Config>::RuntimeOrigin,
		_call: &<T as frame_system::Config>::RuntimeCall,
		_info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		_len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		match val {
			Val::NotUsing => Ok(false),
			Val::UsingAliasWithAccount(who, nonce) => {
				prepare_nonce_for_account::<T>(&who, nonce)?;
				Ok(true)
			},
			Val::UsingAliasWithProof => Ok(true),
			Val::UsingIdentityWithProof => Ok(true),
		}
	}

	fn post_dispatch_details(
		pre: Self::Pre,
		_info: &DispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<<T as frame_system::Config>::RuntimeCall>,
		_len: usize,
		_result: &sp_runtime::DispatchResult,
	) -> Result<Weight, TransactionValidityError> {
		if pre {
			// Is using tx ext
			Ok(Weight::zero())
		} else {
			// Not using tx ext: some refund.
			Ok(Self::weight())
		}
	}
}
