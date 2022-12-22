use std::collections::HashMap;
use std::sync::Arc;

use ethers::abi::Address;
use ethers::prelude::{abigen, Middleware, Signer, SignerMiddleware, U256};

use ethers::types::I256;
use serde::{Deserialize, Serialize};

use super::fixed_point_math::{FixedPointMath, FixedPointMathGen};

const INTERVAL: u32 = 4 * 7 * 86_400;

abigen!(
    ERC20,
    "node_modules/@exactly-protocol/protocol/deployments/goerli/DAI.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct PriceRate {
    pub address: Address,
    pub conversion_selector: [u8; 4],
    pub base_unit: U256,
    pub main_price: U256,
    pub rate: U256,
    pub event_emitter: Option<Address>,
}

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct PriceDouble {
    pub price_feed_one: Address,
    pub price_feed_two: Address,
    pub base_unit: U256,
    pub decimals: U256,
    pub price_one: U256,
    pub price_two: U256,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PriceFeedType {
    Single(PriceRate),
    Double(PriceDouble),
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct PriceFeedController {
    pub address: Address,
    pub main_price_feed: Option<Box<PriceFeedController>>,
    pub event_emitters: Vec<Address>,
    pub wrapper: Option<PriceFeedType>,
}

impl PriceFeedController {
    pub fn main_price_feed(address: Address, event_emitters: Option<Vec<Address>>) -> Self {
        Self {
            address,
            main_price_feed: None,
            event_emitters: event_emitters.unwrap_or_default(),
            wrapper: None,
        }
    }
}

#[derive(Eq, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct FixedPool {
    pub borrowed: U256,
    pub supplied: U256,
    pub unassigned_earnings: U256,
    pub last_accrual: U256,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Market {
    pub contract: Address,
    pub interest_rate_model: Address,
    pub price: U256,
    pub penalty_rate: U256,
    pub adjust_factor: U256,
    pub decimals: u8,
    pub floating_assets: U256,
    pub floating_deposit_shares: U256,
    pub floating_debt: U256,
    pub floating_borrow_shares: U256,
    pub floating_utilization: U256,
    pub last_floating_debt_update: U256,
    pub max_future_pools: u8,
    pub fixed_pools: HashMap<U256, FixedPool>,
    pub smart_pool_fee_rate: U256,
    pub earnings_accumulator: U256,
    pub last_accumulator_accrual: U256,
    pub earnings_accumulator_smooth_factor: U256,
    pub price_feed: Option<PriceFeedController>,
    pub listed: bool,
    pub floating_full_utilization: u128,
    pub floating_a: U256,
    pub floating_b: i128,
    pub floating_max_utilization: U256,
    pub treasury_fee_rate: U256,
    pub asset: Address,
    pub base_market: bool,
}

impl Eq for Market {}

impl PartialEq for Market {
    fn eq(&self, other: &Self) -> bool {
        self.contract == other.contract
    }
}

impl Market {
    pub fn new(address: Address) -> Self {
        Self {
            contract: address,
            interest_rate_model: Default::default(),
            price: Default::default(),
            penalty_rate: Default::default(),
            adjust_factor: Default::default(),
            decimals: Default::default(),
            floating_assets: Default::default(),
            floating_deposit_shares: Default::default(),
            floating_debt: Default::default(),
            floating_borrow_shares: Default::default(),
            floating_utilization: Default::default(),
            last_floating_debt_update: Default::default(),
            max_future_pools: Default::default(),
            fixed_pools: Default::default(),
            smart_pool_fee_rate: Default::default(),
            earnings_accumulator: Default::default(),
            last_accumulator_accrual: Default::default(),
            earnings_accumulator_smooth_factor: Default::default(),
            price_feed: Default::default(),
            listed: Default::default(),
            floating_full_utilization: Default::default(),
            floating_a: Default::default(),
            floating_b: Default::default(),
            floating_max_utilization: Default::default(),
            treasury_fee_rate: Default::default(),
            asset: Default::default(),
            base_market: false,
        }
    }

    pub fn contract<M: 'static + Middleware, S: 'static + Signer>(
        &self,
        client: Arc<SignerMiddleware<M, S>>,
    ) -> crate::generate_abi::market_protocol::MarketProtocol<SignerMiddleware<M, S>> {
        crate::generate_abi::market_protocol::MarketProtocol::new(self.contract, client)
    }

    pub fn total_assets(&self, timestamp: U256) -> U256 {
        let latest = ((timestamp - (timestamp % INTERVAL)) / INTERVAL).as_u32();
        let mut smart_pool_earnings = U256::zero();
        for i in latest..=latest + self.max_future_pools as u32 {
            let maturity = U256::from(INTERVAL * i);
            if let Some(fixed_pool) = self.fixed_pools.get(&maturity) {
                if maturity > fixed_pool.last_accrual {
                    smart_pool_earnings += if timestamp < maturity {
                        fixed_pool.unassigned_earnings.mul_div_down(
                            timestamp - fixed_pool.last_accrual,
                            maturity - fixed_pool.last_accrual,
                        )
                    } else {
                        fixed_pool.unassigned_earnings
                    }
                }
            }
        }
        self.floating_assets
            + smart_pool_earnings
            + self.accumulated_earnings(timestamp)
            + (self.total_floating_borrow_assets(timestamp) - self.floating_debt)
                .mul_wad_down(U256::exp10(18) - self.treasury_fee_rate)
    }

    pub fn accumulated_earnings(&self, timestamp: U256) -> U256 {
        let elapsed = timestamp - self.last_accumulator_accrual;
        if elapsed > U256::zero() {
            self.earnings_accumulator.mul_div_down(
                elapsed,
                elapsed
                    + self
                        .earnings_accumulator_smooth_factor
                        .mul_wad_down(U256::from(INTERVAL * self.max_future_pools as u32)),
            )
        } else {
            U256::zero()
        }
    }

    fn floating_borrow_rate(&self, utilization_before: U256, utilization_after: U256) -> U256 {
        let precision_threshold: U256 = U256::exp10(13) * 75u8; // 7.5e14

        let alpha = self.floating_max_utilization - utilization_before;
        let delta = utilization_after - utilization_before;
        let r = if delta.div_wad_down(alpha) < precision_threshold {
            I256::from_raw(
                (self.floating_a.div_wad_down(alpha)
                    + self.floating_a.mul_div_down(
                        U256::exp10(18) * 4u8,
                        self.floating_max_utilization
                            - ((utilization_after + utilization_before) / 2u8),
                    )
                    + self
                        .floating_a
                        .div_wad_down(self.floating_max_utilization - utilization_after))
                    / 6u8,
            )
        } else {
            self.floating_a.mul_div_down(
                alpha
                    .div_wad_down(self.floating_max_utilization - utilization_after)
                    .ln_wad(),
                I256::from_raw(delta),
            )
        } + I256::from(self.floating_b);
        r.into_raw()
    }

    pub fn total_floating_borrow_assets(&self, timestamp: U256) -> U256 {
        let new_floating_utilization = if self.floating_assets > U256::zero() {
            self.floating_debt.div_wad_up(self.floating_assets)
        } else {
            U256::zero()
        };
        let new_debt = self.floating_debt.mul_wad_down(
            self.floating_borrow_rate(
                U256::min(self.floating_utilization, new_floating_utilization),
                U256::max(self.floating_utilization, new_floating_utilization),
            )
            .mul_div_down(
                timestamp - self.last_floating_debt_update,
                U256::from(365 * 24 * 60 * 60),
            ),
        );
        self.floating_debt + new_debt
    }
}
