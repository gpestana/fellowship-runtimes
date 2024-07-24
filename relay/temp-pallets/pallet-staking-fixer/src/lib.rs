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

//! # Temporary pallet to fix corrupted staking ledgers.
//!
//! This single-purpose pallet exposes an extrinsic which works as a proxy to
//! `Staking::restore_ledger`. There are three differences between the extrinsic exposed by this
//! pallet and the exposed by the Staking pallet, namely:
//!
//! - Access control: any signed account may call `Call::restore_ledger`. The call to
//! `Staking::restore_ledger` from this pallet will be performed with Origin::Root. It is safe to
//! expose the call to the sign origin since *only* corrupted ledgers can be restored. If the stash
//! is not corrupted, the call is a no-op (modulo paid fees).
//! - Default inputs: `Call::restore_ledger` exposes less inputs than the proxied
//! `Staking::restore_ledger` call, providing sensible defaults that are enough to restore all
//! corrupted ledgers.
//! - Extra safety check: if the resulting restored ledger from `Staking::restore_ledger` has a
//! larger total staked than the stash's free balance, the ledger is unstaked.
//!
//! Note: this pallet is temporary and can be removed when all the ledgers in Polkadot and Kusama
//! are restored.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Currency;
use frame_system::ensure_signed;
use pallet_staking::WeightInfo;
use sp_staking::StakingAccount;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::{pallet_prelude::*, RawOrigin};

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_staking::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Ledger of `stash` has been recovered.
		Restored { stash: T::AccountId },
		/// Ledger of `stash` has been recovered. The resulting recoving ended up in unbonding and
		/// the ledger to unstake.
		RestoredUnstaked { stash: T::AccountId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Restores the state of a staking ledger that is in an inconsistent state.
		///
		/// This call functions as a proxy to `Staking::restore_ledger`, but exposes it to signed
		/// origins. In addition, this call checks if, after the ledger has been successfully
		/// restored, the ledger locks are higher or equal than the stash's free balance. If not, it
		/// forces the unstake of the ledger.
		///
		/// Safety note: Only ledgers associated `stash` that are corrupted will be mutated. Thus it
		/// is safe to expose `Staking::restore_ledger` through the signed origin.
		#[pallet::call_index(0)]
		#[pallet::weight(
            <T::WeightInfo>::restore_ledger() +
            <T::WeightInfo>::force_unstake(maybe_slashing_spans.unwrap_or_default())
        )]
		pub fn restore_ledger(
			origin: OriginFor<T>,
			stash: T::AccountId,
			maybe_slashing_spans: Option<u32>,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;

			// calls `Staking::restore_ledger` as `Root`.
			pallet_staking::Pallet::<T>::restore_ledger(
				RawOrigin::Root.into(),
				stash.clone(),
				None,
				None,
				None,
			)?;

			let ledger = pallet_staking::Pallet::<T>::ledger(StakingAccount::Stash(stash.clone()))?;

			// check if stash's free balance covers the current ledger's total amount. If not,
			// force unstake the ledger.
			let weight = if ledger.total > T::Currency::free_balance(&stash) {
				let slashing_spans = maybe_slashing_spans.unwrap_or_default();

				pallet_staking::Pallet::<T>::force_unstake(
					RawOrigin::Root.into(),
					stash.clone(),
					slashing_spans,
				)?;

				Self::deposit_event(Event::<T>::RestoredUnstaked { stash });

				<T::WeightInfo>::restore_ledger() + <T::WeightInfo>::force_unstake(slashing_spans)
			} else {
				Self::deposit_event(Event::<T>::Restored { stash });

				<T::WeightInfo>::restore_ledger()
			};

			Ok(Some(weight).into())
		}
	}
}
