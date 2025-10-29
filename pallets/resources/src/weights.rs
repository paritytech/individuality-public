// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Individuality.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.

// Individuality is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Individuality.  If not, see <http://www.gnu.org/licenses/>.

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
