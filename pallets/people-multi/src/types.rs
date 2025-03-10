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
/// Information about the current key inclusion status in a ring.
pub struct RingStatus {
	/// The number of keys in the ring.
	pub total: u32,
	/// The number of keys that have already been baked in.
	pub included: u32,
}

/// The ring index and position in said ring they are assigned to, none if the person is waiting
/// to be onboarded.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum RingPosition {
	/// Coordinates within the onboarding queue for a person that doesn't belong to a ring yet.
	Onboarding { queue_page: PageIndex },
	/// Coordinates within the rings for a person that was registered.
	Included { ring_index: RingIndex, ring_position: u32 },
	/// The person is suspended and isn't part of any ring or onboarding queue page.
	Suspended,
}

impl RingPosition {
	/// Returns whether the person is suspended and has no position.
	pub fn suspended(&self) -> bool {
		matches!(self, Self::Suspended)
	}

	/// Returns the index of the ring if this person is included.
	pub fn ring_index(&self) -> Option<RingIndex> {
		match &self {
			Self::Included { ring_index, .. } => Some(*ring_index),
			_ => None,
		}
	}
}

/// Record of personhood.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PersonRecord<Member, AccountId> {
	// The key used for the person.
	pub key: Member,
	// The position identifier of the key.
	pub position: RingPosition,
	/// An optional privileged account that can send transaction on the behalf of the person.
	pub account: Option<AccountId>,
}

/// Describes the action to take after checking the first two pages of the onboarding queue for a
/// potential merge.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub(crate) enum QueueMergeAction<T: Config> {
	Merge {
		initial_head: PageIndex,
		new_head: PageIndex,
		first_key_page: BoundedVec<MemberOf<T>, T::OnboardingQueuePageSize>,
		second_key_page: BoundedVec<MemberOf<T>, T::OnboardingQueuePageSize>,
	},
	NoAction,
}
