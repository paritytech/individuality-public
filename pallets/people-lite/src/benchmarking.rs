// This file is part of Substrate.

// Copyright (C) 2020-2022 Parity Technologies (UK) Ltd.
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

//! People lite pallet benchmarks

use super::*;
use alloc::vec::Vec;
use codec::{Decode, Encode};
use frame_benchmarking::v2::{benchmarks, *};
use frame_support::dispatch::{DispatchInfo, RawOrigin};
use frame_system::RawOrigin as SystemOrigin;
use sp_runtime::traits::{
	AsSystemOriginSigner, AsTransactionAuthorizedOrigin, DispatchTransaction, TrailingZeroInput,
};

#[benchmarks(
	where T: Config + core::marker::Send + core::marker::Sync,
	<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo> + From<Call<T>>,
	<<T as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: AsSystemOriginSigner<T::AccountId> + AsTransactionAuthorizedOrigin + Clone,
)]
mod benches {
	use super::*;
	use frame_benchmarking::v2::{account, whitelisted_caller};

	fn insert_lite_person<T: Config>(who: &T::AccountId) {
		let ring_vrf_key: crate::MemberOf<T> =
			Decode::decode(&mut TrailingZeroInput::zeroes()).expect("decode member");
		let info = crate::LitePersonInfo { ring_vrf_key, special_key: [0u8; 32] };
		crate::LitePeople::<T>::insert(who, info);
	}

	#[benchmark]
	fn as_lite_person_tx_ext() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		insert_lite_person::<T>(&caller);

		let call: <T as frame_system::Config>::RuntimeCall =
			frame_system::Call::<T>::remark { remark: Vec::new() }.into();
		let len = call.encode().len();
		frame_system::Pallet::<T>::inc_sufficients(&caller);

		let tx_ext =
			crate::PeopleLiteAuth::<T>::new(Some(crate::PeopleLiteAuthData::AsLitePerson {
				nonce: 0u32.into(),
			}));

		let origin = SystemOrigin::Signed(caller.clone()).into();

		#[block]
		{
			tx_ext
				.test_run(origin, &call, &Default::default(), len, 0, |_| Ok(Default::default()))
				.unwrap()
				.unwrap();
		}

		Ok(())
	}

	#[benchmark]
	fn as_lite_person_registration_tx_ext() -> Result<(), BenchmarkError> {
		let who: T::AccountId = whitelisted_caller();
		let verifier: T::AccountId = account("verifier", 0, 0);
		let att: crate::pallet::AttestationOf<T> = account("att", 0, 0);

		// Seed the invite (unclaimed attestation).
		crate::pallet::UnclaimedAttestations::<T>::insert(&verifier, &att, ());

		// Build a valid Ring-VRF ownership proof for the registration message.
		let secret = <T as crate::Config>::Crypto::new_secret([42u8; 32]);
		let ring_vrf_key = <T as crate::Config>::Crypto::member_from_secret(&secret);
		let msg =
			who.using_encoded(|b| [&crate::pallet::PROOF_OF_OWNERSHIP_PREFIX[..], b].concat());
		let proof_of_ownership = <T as crate::Config>::Crypto::sign(&secret, &msg[..])
			.map_err(|_| BenchmarkError::Weightless)?;

		// Craft an attestation signature that verifies in the test runtime (UintAuthorityId).
		// This decodes the attestation AccountId into the signature type.
		let attestation_signature: <T as crate::Config>::AttestationSignature =
			Decode::decode(&mut &att.encode()[..]).map_err(|_| BenchmarkError::Weightless)?;

		// Construct the register call.
		let call: <T as frame_system::Config>::RuntimeCall = crate::Call::<T>::register {
			ring_vrf_key,
			special_key: [0u8; 32],
			proof_of_ownership,
			verifier: verifier.clone(),
			attestation: att.clone(),
			attestation_signature,
		}
		.into();
		let len = call.encode().len();

		// Build the extension in the registration mode.
		let auth = crate::PeopleLiteAuth::<T>::new(Some(
			crate::PeopleLiteAuthData::AsLitePersonRegistration { nonce: 0u32.into() },
		));

		let origin = SystemOrigin::Signed(who.clone()).into();

		#[block]
		{
			auth.test_run(origin, &call, &Default::default(), len, 0, |_| Ok(Default::default()))
				.unwrap()
				.unwrap();
		}

		Ok(())
	}

	#[benchmark]
	fn increase_attestation_allowance() -> Result<(), BenchmarkError> {
		let verifier: T::AccountId = whitelisted_caller();
		let count: u32 = 50;

		#[extrinsic_call]
		_(RawOrigin::Root, verifier.clone(), count);

		assert_eq!(crate::AttestationAllowance::<T>::get(&verifier), count);
		Ok(())
	}

	#[benchmark]
	fn clear_attestation_allowance(n: Linear<1, 1000>) -> Result<(), BenchmarkError> {
		let verifier: T::AccountId = whitelisted_caller();

		// Seed some allowance and `n` unclaimed attestations so we measure the linear clear.
		crate::AttestationAllowance::<T>::insert(&verifier, 999_u32);
		for i in 0..n {
			let att: crate::AttestationOf<T> = account("att", i, 0);
			crate::UnclaimedAttestations::<T>::insert(&verifier, &att, ());
		}

		#[extrinsic_call]
		_(RawOrigin::Root, verifier.clone(), n);

		assert_eq!(crate::UnclaimedAttestations::<T>::iter_prefix(&verifier).count(), 0);
		assert!(!crate::AttestationAllowance::<T>::contains_key(&verifier));
		Ok(())
	}

	#[benchmark]
	fn set_attestation() -> Result<(), BenchmarkError> {
		let attester: T::AccountId = whitelisted_caller();
		let att: crate::AttestationOf<T> = account("att", 0, 0);

		// Must have allowance.
		crate::AttestationAllowance::<T>::insert(&attester, 1);

		#[extrinsic_call]
		_(RawOrigin::Signed(attester.clone()), att.clone());

		assert!(crate::UnclaimedAttestations::<T>::contains_key(&attester, &att));
		assert_eq!(crate::AttestationAllowance::<T>::get(&attester), 0);
		assert!(!crate::AttestationAllowance::<T>::contains_key(&attester));
		Ok(())
	}

	#[benchmark]
	fn cancel_attestation() -> Result<(), BenchmarkError> {
		let attester: T::AccountId = whitelisted_caller();
		let att: crate::AttestationOf<T> = account("att", 1, 0);

		// Seed an existing unclaimed attestation.
		crate::UnclaimedAttestations::<T>::insert(&attester, &att, ());
		assert_eq!(crate::AttestationAllowance::<T>::get(&attester), 0);

		#[extrinsic_call]
		_(RawOrigin::Signed(attester.clone()), att.clone());

		assert!(!crate::UnclaimedAttestations::<T>::contains_key(&attester, &att));
		assert_eq!(crate::AttestationAllowance::<T>::get(&attester), 1);
		Ok(())
	}

	#[benchmark]
	fn register() -> Result<(), BenchmarkError> {
		let who: T::AccountId = whitelisted_caller();
		let verifier: T::AccountId = account("verifier", 0, 0);
		let att: crate::AttestationOf<T> = account("att", 2, 0);

		// Seed the required unclaimed attestation.
		crate::UnclaimedAttestations::<T>::insert(&verifier, &att, ());

		// Minimal, generic values for keys/signatures via zero-Decode.
		let ring_vrf_key: crate::MemberOf<T> =
			Decode::decode(&mut TrailingZeroInput::zeroes()).expect("decode member");
		let proof: crate::SignatureOf<T> =
			Decode::decode(&mut TrailingZeroInput::zeroes()).expect("decode signature");
		let att_sig: <T as crate::Config>::AttestationSignature =
			Decode::decode(&mut TrailingZeroInput::zeroes()).expect("decode attestation sig");

		#[extrinsic_call]
		_(
			crate::Origin::<T>::LitePersonRegistration(who.clone()),
			ring_vrf_key,
			[0u8; 32],
			proof,
			verifier.clone(),
			att.clone(),
			att_sig,
		);

		assert!(crate::LitePeople::<T>::contains_key(&who));
		assert!(!crate::UnclaimedAttestations::<T>::contains_key(&verifier, &att));
		Ok(())
	}

	#[benchmark]
	fn dispatch_as_signer() -> Result<(), BenchmarkError> {
		let who: T::AccountId = whitelisted_caller();
		insert_lite_person::<T>(&who);

		let nested: <T as frame_system::Config>::RuntimeCall =
			frame_system::Call::<T>::remark_with_event { remark: Vec::new() }.into();

		#[extrinsic_call]
		_(crate::Origin::<T>::LitePerson(who.clone()), Box::new(nested));

		// TODO: assert event
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
