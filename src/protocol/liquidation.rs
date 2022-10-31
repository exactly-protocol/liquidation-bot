use super::fixed_point_math::{math, FixedPointMath, FixedPointMathGen};
use ethers::prelude::{Address, Middleware, Multicall, Signer, SignerMiddleware, U256};
use eyre::Result;
use serde::Deserialize;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use std::{collections::HashMap, time::Duration};
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tokio::time;

use super::{Account, Auditor, LiquidationIncentive, Liquidator, MarketAccount, Previewer};

#[derive(Default, Debug)]
pub struct Repay {
    pub price: U256,
    pub decimals: u8,
    pub market_to_seize: Option<Address>,
    pub market_to_seize_value: U256,
    pub market_to_repay: Option<Address>,
    pub market_to_liquidate_debt: U256,
    pub total_value_collateral: U256,
    pub total_adjusted_collateral: U256,
    pub total_value_debt: U256,
    pub total_adjusted_debt: U256,
    pub repay_asset_address: Address,
    pub collateral_asset_address: Address,
}

#[derive(Debug)]
pub enum LiquidationAction {
    Update,
    Insert,
}

impl Default for LiquidationAction {
    fn default() -> Self {
        LiquidationAction::Update
    }
}

#[derive(Default, Debug)]
pub struct LiquidationData {
    pub liquidations: HashMap<Address, (Account, Repay)>,
    pub eth_price: U256,
    pub gas_price: U256,
    pub liquidation_incentive: LiquidationIncentive,
    pub action: LiquidationAction,
    pub markets: Vec<Address>,
    pub assets: HashMap<Address, Address>,
    pub price_feeds: HashMap<Address, Address>,
}

pub struct Liquidation<M, S> {
    pub client: Arc<SignerMiddleware<M, S>>,
    token_pairs: Arc<HashMap<(Address, Address), BinaryHeap<Reverse<u32>>>>,
    tokens: Arc<HashSet<Address>>,
    liquidator: Liquidator<SignerMiddleware<M, S>>,
    previewer: Previewer<SignerMiddleware<M, S>>,
    auditor: Auditor<SignerMiddleware<M, S>>,
    market_weth_address: Address,
    backup: u32,
    liquidate_unprofitable: bool,
}

impl<M: 'static + Middleware, S: 'static + Signer> Liquidation<M, S> {
    pub fn new(
        client: Arc<SignerMiddleware<M, S>>,
        token_pairs: &str,
        liquidator: Liquidator<SignerMiddleware<M, S>>,
        previewer: Previewer<SignerMiddleware<M, S>>,
        auditor: Auditor<SignerMiddleware<M, S>>,
        weth_address: Address,
        backup: u32,
        liquidate_unprofitable: bool,
    ) -> Self {
        let (token_pairs, tokens) = parse_token_pairs(token_pairs);
        let token_pairs = Arc::new(token_pairs);
        let tokens = Arc::new(tokens);
        Self {
            client,
            token_pairs,
            tokens,
            liquidator,
            previewer,
            auditor,
            market_weth_address: weth_address,
            backup,
            liquidate_unprofitable,
        }
    }

    pub fn get_tokens(&self) -> Arc<HashSet<Address>> {
        Arc::clone(&self.tokens)
    }

    pub fn get_token_pairs(&self) -> Arc<HashMap<(Address, Address), BinaryHeap<Reverse<u32>>>> {
        Arc::clone(&self.token_pairs)
    }

    pub async fn run(
        this: Arc<Mutex<Self>>,
        mut receiver: Receiver<LiquidationData>,
    ) -> Result<()> {
        let mut liquidations = HashMap::new();
        let mut liquidations_iter = None;
        let mut eth_price = U256::zero();
        let mut gas_price = U256::zero();
        let mut liquidation_incentive = None;
        let mut markets = Vec::new();
        let mut price_feeds = HashMap::new();
        let mut assets = HashMap::new();
        let backup = this.lock().await.backup;
        let d = Duration::from_millis(1);
        loop {
            match time::timeout(d, receiver.recv()).await {
                Ok(Some(data)) => {
                    match data.action {
                        LiquidationAction::Update => {
                            liquidations = data
                                .liquidations
                                .into_iter()
                                .map(|(account_address, (account, repay))| {
                                    let age = if backup > 0 {
                                        liquidations
                                            .get(&account_address)
                                            .map(|(_, _, age)| *age)
                                            .unwrap_or(0)
                                            + 1
                                    } else {
                                        0
                                    };
                                    (account_address, (account, repay, age))
                                })
                                .collect();
                        }
                        LiquidationAction::Insert => {
                            let mut new_liquidations = data.liquidations;
                            for (k, v) in new_liquidations.drain() {
                                let liquidation = liquidations.entry(k).or_insert((v.0, v.1, 0));
                                if backup > 0 {
                                    liquidation.2 += 1;
                                }
                            }
                        }
                    }
                    liquidations_iter = Some(liquidations.iter());
                    eth_price = data.eth_price;
                    gas_price = data.gas_price;
                    liquidation_incentive = Some(data.liquidation_incentive);
                    markets = data.markets;
                    price_feeds = data.price_feeds;
                    assets = data.assets;
                }
                Ok(None) => {}
                Err(_) => {
                    if let Some(liquidation) = &mut liquidations_iter {
                        if let Some((_, (account, repay, age))) = liquidation.next() {
                            if backup == 0 || *age > backup {
                                if backup > 0 {
                                    println!("backup liquidation - {}", age);
                                }
                                let _ = this
                                    .lock()
                                    .await
                                    .liquidate(
                                        account,
                                        repay,
                                        liquidation_incentive.as_ref().unwrap(),
                                        gas_price,
                                        eth_price,
                                        &markets,
                                        &price_feeds,
                                        &assets,
                                    )
                                    .await;
                            } else {
                                println!("backup - not old enough: {}", age);
                            }
                        } else {
                            liquidations_iter = None;
                        }
                    }
                }
            }
        }
    }

    async fn liquidate(
        &self,
        account: &Account,
        repay: &Repay,
        _liquidation_incentive: &LiquidationIncentive,
        last_gas_price: U256,
        _eth_price: U256,
        markets: &Vec<Address>,
        price_feeds: &HashMap<Address, Address>,
        assets: &HashMap<Address, Address>,
    ) -> Result<()> {
        println!("Liquidating account {:?}", account);
        if let Some(address) = &repay.market_to_repay {
            let response = self
                .is_profitable_async(
                    account.address,
                    last_gas_price,
                    markets,
                    price_feeds,
                    assets,
                )
                .await;

            let (profitable, max_repay, pool_pair, fee) = match response {
                Some(response) => response,
                None => return Ok(()),
            };

            if !profitable && !self.liquidate_unprofitable {
                println!("not profitable to liquidate");
                println!(
                    "repay$: {:?}",
                    max_repay.mul_div_up(repay.price, U256::exp10(repay.decimals as usize))
                );
                return Ok(());
            }

            println!("Liquidating on market {:#?}", address);
            println!("seizing                    {:#?}", repay.market_to_seize);

            // liquidate using liquidator contract
            let func = self
                .liquidator
                .liquidate(
                    *address,
                    repay.market_to_seize.unwrap_or(Address::zero()),
                    account.address,
                    max_repay,
                    pool_pair,
                    fee,
                )
                .gas(6_666_666u128);

            let tx = func.send().await;
            println!("tx: {:?}", &tx);
            let tx = tx?;
            println!("waiting receipt");
            let receipt = tx.confirmations(1).await?;
            println!("Liquidation tx {:?}", receipt);
        }
        println!("done liquidating");
        Ok(())
    }

    pub async fn is_profitable_async(
        &self,
        account: Address,
        last_gas_price: U256,
        markets: &Vec<Address>,
        price_feeds: &HashMap<Address, Address>,
        assets: &HashMap<Address, Address>,
    ) -> Option<(bool, U256, Address, u32)> {
        let mut multicall =
            Multicall::<SignerMiddleware<M, S>>::new(Arc::clone(&self.client), None)
                .await
                .unwrap();
        multicall.add_call(self.previewer.exactly(account));
        multicall.add_call(
            self.auditor
                .account_liquidity(account, Address::zero(), U256::zero()),
        );
        multicall.add_call(self.auditor.liquidation_incentive());

        let mut price_multicall =
            Multicall::<SignerMiddleware<M, S>>::new(Arc::clone(&self.client), None)
                .await
                .unwrap();
        markets.iter().for_each(|market| {
            price_multicall.add_call(self.auditor.asset_price(price_feeds[market]));
        });

        let response = tokio::try_join!(multicall.call(), price_multicall.call_raw());

        let (data, prices) = if let Ok(response) = response {
            response
        } else {
            return None;
        };

        let (market_account, (adjusted_collateral, adjusted_debt), liquidation_incentive): (
            Vec<MarketAccount>,
            (U256, U256),
            LiquidationIncentive,
        ) = data;

        let prices: HashMap<Address, U256> = markets
            .iter()
            .zip(prices)
            .map(|(market, price)| (*market, price.into_uint().unwrap()))
            .collect();

        if adjusted_debt.is_zero() {
            return None;
        }
        let hf = adjusted_collateral.div_wad_down(adjusted_debt);
        if hf > math::WAD {
            return None;
        }
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let repay = Self::pick_markets(&market_account, &prices, timestamp.into(), assets);
        Self::is_profitable(
            &repay,
            &liquidation_incentive,
            last_gas_price,
            prices[&self.market_weth_address],
            &self.token_pairs,
            &self.tokens,
        )
    }

    pub fn pick_markets(
        market_account: &Vec<MarketAccount>,
        prices: &HashMap<Address, U256>,
        timestamp: U256,
        assets: &HashMap<Address, Address>,
    ) -> Repay {
        let mut repay = Repay::default();
        for market in market_account {
            if market.is_collateral {
                let collateral_value = market.floating_deposit_assets.mul_div_down(
                    prices[&market.market],
                    U256::exp10(market.decimals as usize),
                );
                let adjusted_collateral =
                    collateral_value.mul_wad_down(market.adjust_factor.into());
                repay.total_value_collateral += collateral_value;
                repay.total_adjusted_collateral += adjusted_collateral;
                if adjusted_collateral >= repay.market_to_seize_value {
                    repay.market_to_seize_value = adjusted_collateral;
                    repay.market_to_seize = Some(market.market);
                    repay.collateral_asset_address = assets[&market.market];
                }
            };
            let mut market_debt_assets = U256::zero();
            for fixed_position in &market.fixed_borrow_positions {
                let borrowed = fixed_position.position.principal + fixed_position.position.fee;
                market_debt_assets += borrowed;
                if U256::from(fixed_position.maturity) < timestamp {
                    market_debt_assets += borrowed.mul_wad_down(
                        (timestamp - U256::from(fixed_position.maturity))
                            * U256::from(market.penalty_rate),
                    )
                }
            }
            market_debt_assets += market.floating_borrow_assets;
            let market_debt_value = market_debt_assets.mul_div_up(
                prices[&market.market],
                U256::exp10(market.decimals as usize),
            );
            let adjusted_debt = market_debt_value.div_wad_up(market.adjust_factor.into());
            repay.total_value_debt += market_debt_value;
            repay.total_adjusted_debt += adjusted_debt;
            if adjusted_debt >= repay.market_to_liquidate_debt {
                repay.market_to_liquidate_debt = adjusted_debt;
                repay.market_to_repay = Some(market.market);
                repay.price = prices[&market.market];
                repay.decimals = market.decimals;
                repay.repay_asset_address = assets[&market.market];
            }
        }
        repay
    }

    pub fn is_profitable(
        repay: &Repay,
        liquidation_incentive: &LiquidationIncentive,
        last_gas_price: U256,
        eth_price: U256,
        token_pairs: &HashMap<(Address, Address), BinaryHeap<Reverse<u32>>>,
        tokens: &HashSet<Address>,
    ) -> Option<(bool, U256, Address, u32)> {
        let max_repay = Self::max_repay_assets(repay, liquidation_incentive, U256::MAX)
            .mul_wad_down(math::WAD + U256::exp10(14))
            + math::WAD.mul_div_up(U256::exp10(repay.decimals as usize), repay.price);
        let (pool_pair, fee): (Address, u32) = Self::get_flash_pair(repay, token_pairs, tokens);
        let profit = Self::max_profit(repay, max_repay, liquidation_incentive);
        let cost = Self::max_cost(
            repay,
            max_repay,
            liquidation_incentive,
            U256::from(fee),
            last_gas_price,
            U256::from(1500u128),
            eth_price,
        );
        let profitable = profit > cost && profit - cost > math::WAD / U256::exp10(16);
        Some((profitable, max_repay, pool_pair, fee))
    }

    fn get_flash_pair(
        repay: &Repay,
        token_pairs: &HashMap<(Address, Address), BinaryHeap<Reverse<u32>>>,
        tokens: &HashSet<Address>,
    ) -> (Address, u32) {
        let collateral = repay.collateral_asset_address;
        let repay = repay.repay_asset_address;

        let mut lowest_fee = u32::MAX;
        let mut pair_contract = Address::zero();

        if collateral != repay {
            if let Some(pair) = token_pairs.get(&ordered_addresses(collateral, repay)) {
                return (collateral, pair.peek().unwrap().0);
            }
            return (collateral, 0);
        }

        for token in tokens {
            if *token != collateral {
                if let Some(pair) = token_pairs.get(&ordered_addresses(*token, collateral)) {
                    if let Some(rate) = pair.peek() {
                        if rate.0 < lowest_fee {
                            lowest_fee = rate.0;
                            pair_contract = *token;
                        }
                    }
                }
            }
        }
        (pair_contract, lowest_fee)
    }

    fn max_repay_assets(
        repay: &Repay,
        liquidation_incentive: &LiquidationIncentive,
        max_liquidator_assets: U256,
    ) -> U256 {
        let close_factor = Self::calculate_close_factor(repay, liquidation_incentive);
        U256::min(
            U256::min(
                repay
                    .total_value_debt
                    .mul_wad_up(U256::min(math::WAD, close_factor)),
                repay.market_to_seize_value.div_wad_up(
                    math::WAD + liquidation_incentive.liquidator + liquidation_incentive.lenders,
                ),
            )
            .mul_div_up(U256::exp10(repay.decimals as usize), repay.price),
            if max_liquidator_assets
                < U256::from_str("115792089237316195423570985008687907853269984665640564039457") //// U256::MAX / WAD
                    .unwrap()
            {
                max_liquidator_assets.div_wad_down(math::WAD + liquidation_incentive.lenders)
            } else {
                max_liquidator_assets
            },
        )
        .min(repay.market_to_liquidate_debt)
    }

    fn max_profit(
        repay: &Repay,
        max_repay: U256,
        liquidation_incentive: &LiquidationIncentive,
    ) -> U256 {
        max_repay
            .mul_div_up(repay.price, U256::exp10(repay.decimals as usize))
            .mul_wad_down(U256::from(
                liquidation_incentive.liquidator + liquidation_incentive.lenders,
            ))
    }

    fn max_cost(
        repay: &Repay,
        max_repay: U256,
        liquidation_incentive: &LiquidationIncentive,
        swap_fee: U256,
        gas_price: U256,
        gas_cost: U256,
        eth_price: U256,
    ) -> U256 {
        let max_repay = max_repay.mul_div_down(repay.price, U256::exp10(repay.decimals as usize));
        max_repay.mul_wad_down(U256::from(liquidation_incentive.lenders))
            + max_repay.mul_wad_down(swap_fee * U256::from(U256::exp10(12)))
            + (gas_price * gas_cost).mul_wad_down(eth_price)
    }

    pub fn calculate_close_factor(
        repay: &Repay,
        liquidation_incentive: &LiquidationIncentive,
    ) -> U256 {
        let target_health = U256::exp10(16usize) * 125u32;
        let adjust_factor = repay
            .total_adjusted_collateral
            .mul_wad_down(repay.total_value_debt)
            .div_wad_up(
                repay
                    .total_adjusted_debt
                    .mul_wad_up(repay.total_value_collateral),
            );
        let close_factor = (target_health
            - repay
                .total_adjusted_collateral
                .div_wad_up(repay.total_adjusted_debt))
        .div_wad_up(
            target_health
                - adjust_factor.mul_wad_down(
                    math::WAD
                        + liquidation_incentive.liquidator
                        + liquidation_incentive.lenders
                        + U256::from(liquidation_incentive.liquidator)
                            .mul_wad_down(liquidation_incentive.lenders.into()),
                ),
        );
        close_factor
    }

    pub fn set_liquidator(&mut self, liquidator: Liquidator<SignerMiddleware<M, S>>) {
        self.liquidator = liquidator;
    }
}

fn ordered_addresses(token0: Address, token1: Address) -> (Address, Address) {
    if token0 < token1 {
        (token0, token1)
    } else {
        (token1, token0)
    }
}

#[derive(Deserialize, Debug)]
pub struct TokenPair {
    pub token0: String,
    pub token1: String,
    pub fee: u32,
}

fn parse_token_pairs(
    token_pairs: &str,
) -> (
    HashMap<(Address, Address), BinaryHeap<Reverse<u32>>>,
    HashSet<Address>,
) {
    let mut tokens = HashSet::new();
    let json_pairs: Vec<(String, String, u32)> = serde_json::from_str(token_pairs).unwrap();
    let mut pairs = HashMap::new();
    for (token0, token1, fee) in json_pairs {
        let token0 = Address::from_str(&token0).unwrap();
        let token1 = Address::from_str(&token1).unwrap();
        tokens.insert(token0);
        tokens.insert(token1);
        pairs
            .entry(ordered_addresses(token0, token1))
            .or_insert(BinaryHeap::new())
            .push(Reverse(fee));
    }
    (pairs, tokens)
}

#[cfg(test)]
mod services_test {

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parse_token_pairs() {
        let tokens = r#"[[
                            "0x0000000000000000000000000000000000000000", 
                            "0x0000000000000000000000000000000000000001", 
                            3000
                          ],
                          [
                            "0x0000000000000000000000000000000000000000", 
                            "0x0000000000000000000000000000000000000001", 
                            1000
                          ],
                          [
                            "0x0000000000000000000000000000000000000000", 
                            "0x0000000000000000000000000000000000000001", 
                            2000
                          ]]"#;
        let (pairs, _) = parse_token_pairs(tokens);
        assert_eq!(
            pairs
                .get(
                    &(ordered_addresses(
                        Address::from_str(&"0x0000000000000000000000000000000000000001").unwrap(),
                        Address::from_str(&"0x0000000000000000000000000000000000000000").unwrap()
                    ))
                )
                .unwrap()
                .peek()
                .unwrap()
                .0,
            1000
        );
    }
}