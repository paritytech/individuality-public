// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

use crate::{
	mock::{exec_as_lite_person_tx, exec_signed_tx, new_test_ext, RuntimeCall, Test},
	pallet::{AttestationAllowance, LitePeople},
	MemberOf, Pallet, RecognitionMethod, MSG_PREFIX,
};
use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError;
use verifiable::GenerateVerifiable;

type SecretOfTest = <<Test as crate::Config>::Crypto as GenerateVerifiable>::Secret;

fn attest_msg(who: u64, key: MemberOf<Test>) -> Vec<u8> {
	[&MSG_PREFIX[..], &who.encode()[..], &key.encode()[..]].concat()
}

fn secret_from_seed(seed: u8) -> SecretOfTest {
	<Test as crate::Config>::Crypto::new_secret([seed; 32])
}

fn member_from_secret(sec: &SecretOfTest) -> MemberOf<Test> {
	<Test as crate::Config>::Crypto::member_from_secret(sec)
}

fn sign_attest_with_secret(sec: &SecretOfTest, who: u64) -> crate::SignatureOf<Test> {
	let key = member_from_secret(sec);
	<Test as crate::Config>::Crypto::sign(sec, &attest_msg(who, key)[..]).expect("sign ok")
}

#[test]
fn increase_attestation_allowance_fails_for_wrong_origin() {
	new_test_ext().execute_with(|| {
		let verifier: u64 = 42;
		let count: u32 = 5;

		// Pre-condition: no allowance.
		assert_eq!(AttestationAllowance::<Test>::get(verifier), 0);

		// Signed (non-root) origin tries to increase allowance -> should fail with BadOrigin.
		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::increase_attestation_allowance {
			account: verifier,
			count,
		});
		let err = exec_signed_tx(1, call).expect_err("must fail for non-root origin");

		assert_eq!(err.unwrap_dispatch().error, DispatchError::BadOrigin);
		assert_eq!(AttestationAllowance::<Test>::get(verifier), 0);
	});
}

#[test]
fn increase_attestation_allowance_succeeds() {
	new_test_ext().execute_with(|| {
		let verifier: u64 = 7;
		let count: u32 = 10;

		// Root increases allowance -> should succeed.
		assert_ok!(Pallet::<Test>::increase_attestation_allowance(
			frame_system::RawOrigin::Root.into(),
			verifier,
			count
		));

		assert_eq!(AttestationAllowance::<Test>::get(verifier), count);
	});
}

#[test]
fn clear_attestation_allowance_fails_for_wrong_origin() {
	new_test_ext().execute_with(|| {
		let verifier: u64 = 11;

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::clear_attestation_allowance {
			account: verifier,
		});
		let err = exec_signed_tx(1, call).expect_err("must fail for non-root origin");

		assert_eq!(err.unwrap_dispatch().error, DispatchError::BadOrigin);
	});
}

#[test]
fn clear_attestation_allowance_succeeds() {
	new_test_ext().execute_with(|| {
		let verifier: u64 = 12;

		// Seed allowance.
		AttestationAllowance::<Test>::insert(verifier, 3);

		// Root clears everything.
		assert_ok!(Pallet::<Test>::clear_attestation_allowance(
			frame_system::RawOrigin::Root.into(),
			verifier,
		));

		// Allowance removed attestations cleared.
		assert_eq!(AttestationAllowance::<Test>::get(verifier), 0);
		assert!(!AttestationAllowance::<Test>::contains_key(verifier));
	});
}

#[test]
fn set_attestation_fails_for_wrong_origin() {
	new_test_ext().execute_with(|| {
		let user: u64 = 300;

		let secret = secret_from_seed(11);
		let ring_vrf_key: MemberOf<Test> = member_from_secret(&secret);
		let proof = sign_attest_with_secret(&secret, user);
		let candidate_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(user);
		// Root is not allowed for a signed-only call.
		let err = Pallet::<Test>::attest(
			frame_system::RawOrigin::Root.into(),
			user,
			candidate_signature,
			ring_vrf_key,
			proof,
		)
		.expect_err("root is not a valid origin for attest");

		assert_eq!(err, DispatchError::BadOrigin.into());
	});
}

#[test]
fn set_attestation_fails_for_no_attestation_allowance() {
	new_test_ext().execute_with(|| {
		let who: u64 = 13;
		let user: u64 = 300;

		let secret = secret_from_seed(11);
		let ring_vrf_key: MemberOf<Test> = member_from_secret(&secret);
		let proof = sign_attest_with_secret(&secret, user);
		let candidate_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(user);
		// No allowance seeded -> must fail with NoAttestationAllowance.
		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::attest {
			candidate: user,
			candidate_signature,
			ring_vrf_key,
			proof_of_ownership: proof,
		});
		let err = exec_signed_tx(who, call).expect_err("must fail without allowance");

		assert_eq!(
			err.unwrap_dispatch().error,
			crate::Error::<Test>::NoAttestationAllowance.into()
		);
		assert_eq!(AttestationAllowance::<Test>::get(who), 0);
	});
}

#[test]
fn set_attestation_succeeds() {
	new_test_ext().execute_with(|| {
		let who: u64 = 15;
		let user: u64 = 300;

		// Seed exactly one allowance.
		AttestationAllowance::<Test>::insert(who, 1);

		let secret = secret_from_seed(11);
		let ring_vrf_key: MemberOf<Test> = member_from_secret(&secret);
		let proof = sign_attest_with_secret(&secret, user);
		let candidate_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(user);

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::attest {
			candidate: user,
			candidate_signature,
			ring_vrf_key,
			proof_of_ownership: proof,
		});
		assert_ok!(exec_signed_tx(who, call));

		// Attestation recorded; allowance consumed (removed).
		assert_eq!(AttestationAllowance::<Test>::get(who), 0);
		assert!(!AttestationAllowance::<Test>::contains_key(who));
	});
}

#[test]
fn register_fails_for_invalid_proof_of_ownership() {
	new_test_ext().execute_with(|| {
		let who: u64 = 50;
		let verifier: u64 = 51;

		// Seed allowance.
		AttestationAllowance::<Test>::insert(verifier, 3);

		// Valid ring key derived from `secret_ok`, but we sign with `secret_wrong`.
		let secret_ok = secret_from_seed(1);
		let secret_wrong = secret_from_seed(2);
		let ring_ok: MemberOf<Test> = member_from_secret(&secret_ok);

		let bad_proof = sign_attest_with_secret(&secret_wrong, who);
		let candidate_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(who);

		assert_noop!(
			crate::Pallet::<Test>::attest(
				Some(verifier).into(),
				who,
				candidate_signature,
				ring_ok,
				bad_proof
			),
			crate::Error::<Test>::InvalidProofOfOwnership
		);
	});
}

#[test]
fn register_fails_for_invalid_attestation_signature() {
	new_test_ext().execute_with(|| {
		let who: u64 = 60;
		let verifier: u64 = 61;

		// Seed allowance.
		AttestationAllowance::<Test>::insert(verifier, 3);

		let secret = secret_from_seed(11);
		let ring_vrf_key: MemberOf<Test> = member_from_secret(&secret);
		let proof = sign_attest_with_secret(&secret, who);

		// Deliberately wrong attestation signature (doesn't match `attestation`).
		let bad_attestation_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(who + 1);

		assert_noop!(
			crate::Pallet::<Test>::attest(
				Some(verifier).into(),
				who,
				bad_attestation_signature,
				ring_vrf_key,
				proof
			),
			crate::Error::<Test>::InvalidAttestationSignature
		);
	});
}

#[test]
fn register_fails_for_already_registered() {
	new_test_ext().execute_with(|| {
		let who: u64 = 70;
		let verifier: u64 = 71;

		// Seed allowance.
		AttestationAllowance::<Test>::insert(verifier, 3);

		// First register using a valid (member, secret, signature) tuple.
		let secret = secret_from_seed(7);
		let ring_vrf_key: MemberOf<Test> = member_from_secret(&secret);
		let proof = sign_attest_with_secret(&secret, who);
		let candidate_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(who);

		// Pre-register the account by calling the pallet directly with the proper origin.
		let set_call = RuntimeCall::PeopleLite(crate::Call::<Test>::attest {
			ring_vrf_key,
			proof_of_ownership: proof,
			candidate: who,
			candidate_signature,
		});
		assert_ok!(exec_signed_tx(verifier, set_call));
		assert!(LitePeople::<Test>::contains_key(who));

		// Try to register again via the tx extension -> must fail "AlreadyRegistered".
		let secret = secret_from_seed(8);
		let ring_vrf_key: MemberOf<Test> = member_from_secret(&secret);
		let proof = sign_attest_with_secret(&secret, who);
		let candidate_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(who);
		assert_noop!(
			crate::Pallet::<Test>::attest(
				Some(verifier).into(),
				who,
				candidate_signature,
				ring_vrf_key,
				proof
			),
			crate::Error::<Test>::AlreadyRegistered
		);
	});
}

#[test]
fn dispatch_as_signer_fails_for_wrong_origin() {
	new_test_ext().execute_with(|| {
		let nested = RuntimeCall::System(frame_system::Call::<Test>::remark_with_event {
			remark: b"nope".to_vec(),
		});

		// Call pallet directly with Root -> must fail (requires LitePerson origin).
		let err = Pallet::<Test>::dispatch_as_signer(
			frame_system::RawOrigin::Root.into(),
			Box::new(nested),
		)
		.expect_err("Root must not be allowed");
		assert_eq!(err.error, DispatchError::BadOrigin);
	});
}

#[test]
fn dispatch_as_signer_succeeds() {
	new_test_ext().execute_with(|| {
		frame_system::Pallet::<Test>::set_block_number(1); // to have events.

		let who: u64 = 100;

		// Make the account a lite person (minimal state required by the tx extension).
		LitePeople::<Test>::insert(
			who,
			crate::pallet::LitePersonInfo {
				ring_vrf_key: [1u8; 32],
				method: RecognitionMethod::UniqueDevice(0),
			},
		);

		// Nested call will be executed as Signed(who).
		let nested = RuntimeCall::System(frame_system::Call::<Test>::remark_with_event {
			remark: b"hello".to_vec(),
		});

		let outer = RuntimeCall::PeopleLite(crate::Call::<Test>::dispatch_as_signer {
			call: Box::new(nested),
		});

		frame_system::Pallet::<Test>::inc_sufficients(&who);

		// First use of this account -> nonce 0 for AsLitePerson.
		assert_ok!(exec_as_lite_person_tx(who, outer, 0));

		// Verify the System::Remarked event contains `sender = who`.
		let found = frame_system::Pallet::<Test>::events().iter().any(|rec| {
			matches!(
				rec.event,
				crate::mock::RuntimeEvent::System(frame_system::Event::Remarked { ref sender, .. })
				if *sender == who
			)
		});
		assert!(found, "expected System::Remarked event from the lite person signer");
	});
}

#[test]
fn full_flow_manager_attester_user_register_and_dispatch() {
	new_test_ext().execute_with(|| {
		frame_system::Pallet::<Test>::set_block_number(1); // to have events.

		let attester: u64 = 210;
		let user: u64 = 211;

		// 1) Manager increases allowance for attester.
		assert_ok!(Pallet::<Test>::increase_attestation_allowance(
			frame_system::RawOrigin::Root.into(),
			attester,
			1,
		));
		assert_eq!(AttestationAllowance::<Test>::get(attester), 1);

		// 2) Verifier registers the user.
		let secret = secret_from_seed(11);
		let ring_vrf_key: MemberOf<Test> = member_from_secret(&secret);
		let proof = sign_attest_with_secret(&secret, user);
		let candidate_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(user);

		let set_call = RuntimeCall::PeopleLite(crate::Call::<Test>::attest {
			ring_vrf_key,
			proof_of_ownership: proof,
			candidate: user,
			candidate_signature,
		});
		assert_ok!(exec_signed_tx(attester, set_call));

		assert!(LitePeople::<Test>::contains_key(user));

		// 3) User dispatches as signer (now a lite person) - first tx -> nonce 0.
		let nested = RuntimeCall::System(frame_system::Call::<Test>::remark_with_event {
			remark: b"full-flow-ok".to_vec(),
		});
		let outer = RuntimeCall::PeopleLite(crate::Call::<Test>::dispatch_as_signer {
			call: Box::new(nested),
		});
		assert_ok!(exec_as_lite_person_tx(user, outer, 0));

		// Confirm the remark event came from the user.
		let found = frame_system::Pallet::<Test>::events().iter().any(|rec| {
			matches!(
				rec.event,
				crate::mock::RuntimeEvent::System(frame_system::Event::Remarked { ref sender, .. })
				if *sender == user
			)
		});
		assert!(found, "expected System::Remarked event from the registered lite person");
	});
}
