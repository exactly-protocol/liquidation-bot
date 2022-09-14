pub mod account;
pub mod config;
pub mod exactly_events;
pub mod market;
pub mod protocol;

mod fixed_point_math;

pub use account::*;
pub use exactly_events::*;
pub use market::Market;
pub use protocol::Protocol;

use ethers::prelude::abigen;

abigen!(
    Market,
    "node_modules/@exactly-protocol/protocol/deployments/rinkeby/MarketWETH.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    Previewer,
    "node_modules/@exactly-protocol/protocol/deployments/rinkeby/Previewer.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    ExactlyOracle,
    "node_modules/@exactly-protocol/protocol/deployments/rinkeby/ExactlyOracle.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    Auditor,
    "node_modules/@exactly-protocol/protocol/deployments/rinkeby/Auditor.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    InterestRateModel,
    "node_modules/@exactly-protocol/protocol/deployments/rinkeby/InterestRateModel.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    PriceFeed,
    "node_modules/@chainlink/contracts/abi/v0.8/AggregatorV2V3Interface.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    AggregatorProxy,
    "node_modules/@chainlink/contracts/abi/v0.7/AggregatorProxyInterface.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    Liquidator,
    "deployments/rinkeby/Liquidator.json",
    event_derives(serde::Deserialize, serde::Serialize)
);