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

	fn insert_lite_person<T: Config>(who: &T::AccountId) {
		let ring_vrf_key: crate::MemberOf<T> =
			Decode::decode(&mut TrailingZeroInput::zeroes()).expect("decode member");
		let info = crate::LitePersonInfo {
			ring_vrf_key,
			method: RecognitionMethod::UniqueDevice(whitelisted_caller()),
		};
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

		#[extrinsic_call]
		_(RawOrigin::Root, verifier.clone());

		assert!(!crate::AttestationAllowance::<T>::contains_key(&verifier));
		Ok(())
	}

	#[benchmark]
	fn set_attestation() -> Result<(), BenchmarkError> {
		let attester: T::AccountId = whitelisted_caller();

		let (att, _) = T::BenchmarkHelper::sign_message(b"mock");
		let mut msg = PROOF_OF_OWNERSHIP_PREFIX.to_vec();
		msg.extend_from_slice(&att.encode());
		let (_, att_sig) = T::BenchmarkHelper::sign_message(&msg[..]);

		// Must have allowance.
		crate::AttestationAllowance::<T>::insert(&attester, 1);

		let sk = T::Crypto::new_secret([12; 32]);
		let pk = T::Crypto::member_from_secret(&sk);
		let proof_of_ownership = T::Crypto::sign(&sk, &msg[..]).unwrap();

		let _ = (proof_of_ownership, att_sig, pk);

		#[block]
		{}

		Ok(())

		// TODO: implement

		// #[extrinsic_call]
		// _(RawOrigin::Signed(attester.clone()), att.clone(), att_sig, pk, proof_of_ownership);

		// assert_eq!(crate::AttestationAllowance::<T>::get(&attester), 0);
		// assert!(!crate::AttestationAllowance::<T>::contains_key(&attester));
		// Ok(())
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
