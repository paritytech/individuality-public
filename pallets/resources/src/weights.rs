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
	fn register_lite_person() -> Weight;
	fn register_person() -> Weight;
	fn touch_person_authorization() -> Weight;
	fn remove_expired_username_reservation() -> Weight;
	fn update_identifier_key() -> Weight;
}

impl WeightInfo for () {
	fn register_lite_person() -> Weight {
		Weight::default()
	}
	fn register_person() -> Weight {
		Weight::default()
	}
	fn touch_person_authorization() -> Weight {
		Weight::default()
	}
	fn remove_expired_username_reservation() -> Weight {
		Weight::default()
	}
	fn update_identifier_key() -> Weight {
		Weight::default()
	}
}
