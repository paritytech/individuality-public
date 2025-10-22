// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

// TODO: generate weights.

pub trait WeightInfo {
	fn as_lite_person_tx_ext() -> Weight;
	fn increase_attestation_allowance() -> Weight;
	fn clear_attestation_allowance() -> Weight;
	fn attest() -> Weight;
	fn cancel_attestation() -> Weight;
	fn register() -> Weight;
	fn dispatch_as_signer() -> Weight;
}

impl WeightInfo for () {
	fn as_lite_person_tx_ext() -> Weight {
		Weight::default()
	}
	fn attest() -> Weight {
		Weight::default()
	}
	fn cancel_attestation() -> Weight {
		Weight::default()
	}
	fn clear_attestation_allowance() -> Weight {
		Weight::default()
	}
	fn increase_attestation_allowance() -> Weight {
		Weight::default()
	}
	fn register() -> Weight {
		Weight::default()
	}
	fn dispatch_as_signer() -> Weight {
		Weight::default()
	}
}
