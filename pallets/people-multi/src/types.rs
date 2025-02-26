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

//! Types for Proof-of-Personhood system.

use super::*;
use frame_support::{pallet_prelude::*, DefaultNoBound};
use sp_runtime::BoundedVec;

pub type RevisionIndex = u32;
pub type PageIndex = u32;
pub type KeyCount = u64;

pub type MemberOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Member;
pub type MembersOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Members;
pub type IntermediateOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Intermediate;
pub type SecretOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Secret;
pub type SignatureOf<T> = <<T as Config>::Crypto as GenerateVerifiable>::Signature;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct RingRoot<T: Config> {
	/// The ring root for the current ring.
	pub root: MembersOf<T>,
	/// The revision index of the ring.
	pub revision: RevisionIndex,
	/// An intermediate value if the ring is not full.
	pub intermediate: IntermediateOf<T>,
}

#[derive(
	PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, DefaultNoBound,
)]
#[scale_info(skip_type_params(T))]
/// This object will always place keys into keys in order, and onboard them in order.
/// Thus `included` will keep track of a single count, which is the number of keys included.
pub struct KeysForRing<T: Config> {
	/// Keys in the ring.
	pub keys: BoundedVec<MemberOf<T>, T::MaxRingSize>,
	/// The number of keys that have already been baked in.
	pub included: u32,
}

/// Record of personhood.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PersonRecord<Member> {
	// The key used for the person.
	pub key: Member,
	// The ring they are assigned to.
	pub ring_index: RingIndex,
	/// The latest root revision that this key is push into.
	pub key_index: u32,
}
