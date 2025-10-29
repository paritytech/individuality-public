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

use frame_support::weights::Weight;

/// Weight functions needed for pallet origins restriction.
pub trait WeightInfo {
	fn clean_usage() -> Weight;
	fn restrict_origin_tx_ext() -> Weight;
}

// For tests
impl WeightInfo for () {
	fn clean_usage() -> Weight { Weight::zero() }
	fn restrict_origin_tx_ext() -> Weight { Weight::zero() }
}
