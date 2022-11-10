use ethers::{
    abi::{Error, RawLog},
    prelude::EthLogDecode,
    types::H256,
};
use std::str::FromStr;

use crate::generate_abi::{
    auditor::{
        AdminChangedFilter, InitializedFilter, RoleAdminChangedFilter, RoleGrantedFilter,
        RoleRevokedFilter, UpgradedFilter,
    },
    price_feed::{AnswerUpdatedFilter, NewRoundFilter},
    AccumulatorAccrualFilter, AdjustFactorSetFilter, BackupFeeRateSetFilter,
    BorrowAtMaturityFilter, BorrowFilter, DampSpeedSetFilter, DepositAtMaturityFilter,
    DepositFilter, EarningsAccumulatorSmoothFactorSetFilter, FixedEarningsUpdateFilter,
    FloatingDebtUpdateFilter, InterestRateModelSetFilter, LiquidateFilter,
    LiquidationIncentiveSetFilter, MarketEnteredFilter, MarketExitedFilter, MarketListedFilter,
    MarketUpdateFilter, MaxFuturePoolsSetFilter, PausedFilter, PenaltyRateSetFilter,
    PriceFeedSetFilter, ProtocolContactsSetFilter, RepayAtMaturityFilter, RepayFilter,
    ReserveFactorSetFilter, ResumedFilter, SeizeFilter, StakingLimitRemovedFilter,
    StakingLimitSetFilter, StakingPausedFilter, StakingResumedFilter, StoppedFilter,
    TreasurySetFilter, UnpausedFilter, WithdrawAtMaturityFilter, WithdrawFilter,
    WithdrawalCredentialsSetFilter, WithdrawalFilter,
};
use crate::generate_abi::{market_protocol::ApprovalFilter, ElrewardsReceivedFilter};
use crate::generate_abi::{
    market_protocol::TransferFilter, ElrewardsVaultSetFilter, ElrewardsWithdrawalLimitSetFilter,
    FeeDistributionSetFilter, FeeSetFilter, RecoverToVaultFilter, ScriptResultFilter,
    SharesBurntFilter, SubmittedFilter, TransferSharesFilter, UnbufferedFilter,
};
use aggregator_mod::NewTransmissionFilter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactlyEvents {
    Transfer(TransferFilter),
    Deposit(DepositFilter),
    Withdraw(WithdrawFilter),
    Approval(ApprovalFilter),
    DepositAtMaturity(DepositAtMaturityFilter),
    WithdrawAtMaturity(WithdrawAtMaturityFilter),
    BorrowAtMaturity(BorrowAtMaturityFilter),
    RepayAtMaturity(RepayAtMaturityFilter),
    Liquidate(LiquidateFilter),
    Seize(SeizeFilter),
    EarningsAccumulatorSmoothFactorSet(EarningsAccumulatorSmoothFactorSetFilter),
    MaxFuturePoolsSet(MaxFuturePoolsSetFilter),
    TreasurySet(TreasurySetFilter),
    RoleGranted(RoleGrantedFilter),
    RoleAdminChanged(RoleAdminChangedFilter),
    RoleRevoked(RoleRevokedFilter),
    Paused(PausedFilter),
    Unpaused(UnpausedFilter),
    MarketUpdate(MarketUpdateFilter),
    FixedEarningsUpdate(FixedEarningsUpdateFilter),
    AccumulatorAccrual(AccumulatorAccrualFilter),
    FloatingDebtUpdate(FloatingDebtUpdateFilter),
    Borrow(BorrowFilter),
    Repay(RepayFilter),
    BackupFeeRateSet(BackupFeeRateSetFilter),

    // Auditor events
    MarketListed(MarketListedFilter),
    MarketEntered(MarketEnteredFilter),
    MarketExited(MarketExitedFilter),
    LiquidationIncentiveSet(LiquidationIncentiveSetFilter),
    AdjustFactorSet(AdjustFactorSetFilter),
    Upgraded(UpgradedFilter),
    Initialized(InitializedFilter),
    AdminChanged(AdminChangedFilter),

    // PoolAccounting events
    InterestRateModelSet(InterestRateModelSetFilter),
    PenaltyRateSet(PenaltyRateSetFilter),
    ReserveFactorSet(ReserveFactorSetFilter),
    DampSpeedSet(DampSpeedSetFilter),

    // ExactlyOracle events
    PriceFeedSetFilter(PriceFeedSetFilter),
    // PriceFeed
    AnswerUpdated(AnswerUpdatedFilter),
    NewRound(NewRoundFilter),
    NewTransmission(NewTransmissionFilter),

    UpdateLidoPrice(Option<H256>),

    Ignore(Option<H256>),
}

macro_rules! map_filter {
    ($ext_filter:ident, $exactly_filter:expr, $log:ident) => {
        if let Ok(_) = $ext_filter::decode_log($log) {
            return Ok($exactly_filter);
        }
    };
}

impl EthLogDecode for ExactlyEvents {
    fn decode_log(log: &RawLog) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Ok(decoded) = RoleGrantedFilter::decode_log(log) {
            return Ok(ExactlyEvents::RoleGranted(decoded));
        }
        if let Ok(decoded) = RoleAdminChangedFilter::decode_log(log) {
            return Ok(ExactlyEvents::RoleAdminChanged(decoded));
        }
        if let Ok(decoded) = RoleRevokedFilter::decode_log(log) {
            return Ok(ExactlyEvents::RoleRevoked(decoded));
        }
        if let Ok(decoded) = TransferFilter::decode_log(log) {
            return Ok(ExactlyEvents::Transfer(decoded));
        }
        if let Ok(decoded) = DepositFilter::decode_log(log) {
            return Ok(ExactlyEvents::Deposit(decoded));
        }
        if let Ok(decoded) = WithdrawFilter::decode_log(log) {
            return Ok(ExactlyEvents::Withdraw(decoded));
        }
        if let Ok(decoded) = ApprovalFilter::decode_log(log) {
            return Ok(ExactlyEvents::Approval(decoded));
        }
        if let Ok(decoded) = DepositAtMaturityFilter::decode_log(log) {
            return Ok(ExactlyEvents::DepositAtMaturity(decoded));
        }
        if let Ok(decoded) = WithdrawAtMaturityFilter::decode_log(log) {
            return Ok(ExactlyEvents::WithdrawAtMaturity(decoded));
        }
        if let Ok(decoded) = BorrowAtMaturityFilter::decode_log(log) {
            return Ok(ExactlyEvents::BorrowAtMaturity(decoded));
        }
        if let Ok(decoded) = RepayAtMaturityFilter::decode_log(log) {
            return Ok(ExactlyEvents::RepayAtMaturity(decoded));
        }
        if let Ok(decoded) = LiquidateFilter::decode_log(log) {
            return Ok(ExactlyEvents::Liquidate(decoded));
        }
        if let Ok(decoded) = SeizeFilter::decode_log(log) {
            return Ok(ExactlyEvents::Seize(decoded));
        }
        if let Ok(decoded) = EarningsAccumulatorSmoothFactorSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::EarningsAccumulatorSmoothFactorSet(decoded));
        }
        if let Ok(decoded) = MaxFuturePoolsSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::MaxFuturePoolsSet(decoded));
        }
        if let Ok(decoded) = PausedFilter::decode_log(log) {
            return Ok(ExactlyEvents::Paused(decoded));
        }
        if let Ok(decoded) = UnpausedFilter::decode_log(log) {
            return Ok(ExactlyEvents::Unpaused(decoded));
        }
        if let Ok(decoded) = MarketUpdateFilter::decode_log(log) {
            return Ok(ExactlyEvents::MarketUpdate(decoded));
        }
        if let Ok(decoded) = FixedEarningsUpdateFilter::decode_log(log) {
            return Ok(ExactlyEvents::FixedEarningsUpdate(decoded));
        }
        if let Ok(decoded) = AccumulatorAccrualFilter::decode_log(log) {
            return Ok(ExactlyEvents::AccumulatorAccrual(decoded));
        }
        if let Ok(decoded) = FloatingDebtUpdateFilter::decode_log(log) {
            return Ok(ExactlyEvents::FloatingDebtUpdate(decoded));
        }
        if let Ok(decoded) = TreasurySetFilter::decode_log(log) {
            return Ok(ExactlyEvents::TreasurySet(decoded));
        }
        if let Ok(decoded) = BorrowFilter::decode_log(log) {
            return Ok(ExactlyEvents::Borrow(decoded));
        }
        if let Ok(decoded) = RepayFilter::decode_log(log) {
            return Ok(ExactlyEvents::Repay(decoded));
        }
        if let Ok(decoded) = BackupFeeRateSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::BackupFeeRateSet(decoded));
        }

        // Auditor events
        if let Ok(decoded) = MarketListedFilter::decode_log(log) {
            return Ok(ExactlyEvents::MarketListed(decoded));
        }
        if let Ok(decoded) = MarketEnteredFilter::decode_log(log) {
            return Ok(ExactlyEvents::MarketEntered(decoded));
        }
        if let Ok(decoded) = MarketExitedFilter::decode_log(log) {
            return Ok(ExactlyEvents::MarketExited(decoded));
        }
        if let Ok(decoded) = LiquidationIncentiveSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::LiquidationIncentiveSet(decoded));
        }
        if let Ok(decoded) = AdjustFactorSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::AdjustFactorSet(decoded));
        }
        if let Ok(decoded) = AdminChangedFilter::decode_log(log) {
            return Ok(ExactlyEvents::AdminChanged(decoded));
        }
        if let Ok(decoded) = UpgradedFilter::decode_log(log) {
            return Ok(ExactlyEvents::Upgraded(decoded));
        }
        if let Ok(decoded) = InitializedFilter::decode_log(log) {
            return Ok(ExactlyEvents::Initialized(decoded));
        }

        // PoolAccounting events
        if let Ok(decoded) = InterestRateModelSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::InterestRateModelSet(decoded));
        }
        if let Ok(decoded) = PenaltyRateSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::PenaltyRateSet(decoded));
        }
        if let Ok(decoded) = ReserveFactorSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::ReserveFactorSet(decoded));
        }
        if let Ok(decoded) = DampSpeedSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::DampSpeedSet(decoded));
        }

        // ExactlyOracle events
        if let Ok(decoded) = PriceFeedSetFilter::decode_log(log) {
            return Ok(ExactlyEvents::PriceFeedSetFilter(decoded));
        }

        // PriceFeed
        if let Ok(decoded) = AnswerUpdatedFilter::decode_log(log) {
            return Ok(ExactlyEvents::AnswerUpdated(decoded));
        }

        if let Ok(decoded) = NewRoundFilter::decode_log(log) {
            return Ok(ExactlyEvents::NewRound(decoded));
        }

        if let Ok(decoded) = NewTransmissionFilter::decode_log(log) {
            return Ok(ExactlyEvents::NewTransmission(decoded));
        }

        // Lido events to update price
        let exactly_event = ExactlyEvents::UpdateLidoPrice(log.topics.get(0).copied());
        map_filter!(ElrewardsReceivedFilter, exactly_event, log);
        map_filter!(ElrewardsVaultSetFilter, exactly_event, log);
        map_filter!(ElrewardsWithdrawalLimitSetFilter, exactly_event, log);
        map_filter!(FeeDistributionSetFilter, exactly_event, log);
        map_filter!(FeeSetFilter, exactly_event, log);
        map_filter!(RecoverToVaultFilter, exactly_event, log);
        map_filter!(ScriptResultFilter, exactly_event, log);
        map_filter!(SharesBurntFilter, exactly_event, log);
        map_filter!(UnbufferedFilter, exactly_event, log);
        map_filter!(WithdrawalFilter, exactly_event, log);

        let exactly_event = ExactlyEvents::Ignore(log.topics.get(0).copied());
        map_filter!(SubmittedFilter, exactly_event, log);
        map_filter!(TransferSharesFilter, exactly_event, log);
        map_filter!(ResumedFilter, exactly_event, log);
        map_filter!(ProtocolContactsSetFilter, exactly_event, log);
        map_filter!(StakingLimitRemovedFilter, exactly_event, log);
        map_filter!(StakingLimitSetFilter, exactly_event, log);
        map_filter!(StakingPausedFilter, exactly_event, log);
        map_filter!(StakingResumedFilter, exactly_event, log);
        map_filter!(StoppedFilter, exactly_event, log);
        map_filter!(WithdrawalCredentialsSetFilter, exactly_event, log);

        let ignored_events: Vec<H256> = [
            "0xe8ec50e5150ae28ae37e493ff389ffab7ffaec2dc4dccfca03f12a3de29d12b2",
            "0xd0d9486a2c673e2a4b57fc82e4c8a556b3e2b82dd5db07e2c04a920ca0f469b6",
            "0xd0b1dac935d85bd54cf0a33b0d41d39f8cf53a968465fc7ea2377526b8ac712c",
            "0x25d719d88a4512dd76c7442b910a83360845505894eb444ef299409e180f8fb9", // ConfigSet(uint32,uint64,address[],address[],uint8,uint64,bytes)
            "0x3ea16a923ff4b1df6526e854c9e3a995c43385d70e73359e10623c74f0b52037", // RoundRequested(address,bytes16,uint32,uint8)
        ]
        .iter()
        .map(|x| H256::from_str(x).unwrap())
        .collect();
        if log.topics.iter().any(|topic| {
            ignored_events
                .iter()
                .any(|ignored_topic| topic == ignored_topic)
        }) {
            return Ok(ExactlyEvents::Ignore(log.topics.get(0).copied()));
        };

        println!("Missing event: {:?}", log);
        Err(Error::InvalidData)
    }
}

mod aggregator_mod {
    use ethers::{
        prelude::{EthDisplay, EthEvent},
        types::{Address, Bytes, I256},
    };

    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        EthEvent,
        EthDisplay,
        serde::Deserialize,
        serde::Serialize,
    )]
    #[ethevent(
        name = "NewTransmission",
        abi = "NewTransmission(uint32,int192,address,int192[],bytes,bytes32)"
    )]
    pub struct NewTransmissionFilter {
        #[ethevent(indexed)]
        pub aggregator_round_id: u32,
        pub answer: I256,
        pub transmitter: Address,
        pub observations: Vec<I256>,
        pub observers: Bytes,
        pub raw_report_context: [u8; 32],
    }
}