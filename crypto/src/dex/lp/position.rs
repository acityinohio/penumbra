use anyhow::Context;
use penumbra_proto::{dex as pb, serializers::bech32str, Protobuf};
use serde::{Deserialize, Serialize};

use super::{super::TradingPair, TradingFunction};

/// Data identifying a position.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "pb::Position", into = "pb::Position")]
pub struct Position {
    pub pair: TradingPair,
    pub phi: TradingFunction,
    /// A random value used to disambiguate different positions with the exact same
    /// trading function.  The chain should reject newly created positions with the
    /// same nonce as an existing position.  This ensures that [`Id`]s will
    /// be unique, and allows us to track position ownership with a
    /// sequence of stateful NFTs based on the [`Id`].
    pub nonce: [u8; 32],
}

impl Position {
    /// Get the ID of this position.
    pub fn id(&self) -> Id {
        let mut state = blake2b_simd::Params::default()
            .personal(b"penumbra_lp_id")
            .to_state();

        state.update(&self.nonce);
        state.update(&self.pair.asset_1.to_bytes());
        state.update(&self.pair.asset_2.to_bytes());
        state.update(&self.phi.fee.to_le_bytes());
        state.update(&self.phi.k.to_le_bytes());
        state.update(&self.phi.p.to_le_bytes());
        state.update(&self.phi.q.to_le_bytes());

        let hash = state.finalize();
        let mut bytes = [0; 32];
        bytes[0..32].copy_from_slice(&hash.as_bytes()[0..32]);
        Id(bytes)
    }
}

/// A hash of a [`Position`].
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Serialize, Deserialize)]
#[serde(try_from = "pb::PositionId", into = "pb::PositionId")]
pub struct Id(pub [u8; 32]);

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&bech32str::encode(
            &self.0,
            bech32str::lp_id::BECH32_PREFIX,
            bech32str::Bech32m,
        ))
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&bech32str::encode(
            &self.0,
            bech32str::lp_id::BECH32_PREFIX,
            bech32str::Bech32m,
        ))
    }
}

impl std::str::FromStr for Id {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = bech32str::decode(s, bech32str::lp_id::BECH32_PREFIX, bech32str::Bech32m)?;
        pb::PositionId { inner }.try_into()
    }
}

/// The state of a position.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
#[serde(try_from = "pb::PositionState", into = "pb::PositionState")]
pub enum State {
    /// The position has been opened, is active, has reserves and accumulated
    /// fees, and can be traded against.
    Opened,
    /// The position has been closed, is inactive and can no longer be traded
    /// against, but still has reserves and accumulated fees.
    Closed,
    /// The final reserves and accumulated fees have been withdrawn, leaving an
    /// empty, inactive position awaiting (possible) retroactive rewards.
    Withdrawn,
    /// Any retroactive rewards have been claimed. The position is now an inert,
    /// historical artefact.
    Claimed,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            State::Opened => write!(f, "opened"),
            State::Closed => write!(f, "closed"),
            State::Withdrawn => write!(f, "withdrawn"),
            State::Claimed => write!(f, "claimed"),
        }
    }
}

impl std::str::FromStr for State {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "opened" => Ok(State::Opened),
            "closed" => Ok(State::Closed),
            "withdrawn" => Ok(State::Withdrawn),
            "claimed" => Ok(State::Claimed),
            _ => Err(anyhow::anyhow!("unknown position state")),
        }
    }
}

// ==== Protobuf impls

impl Protobuf<pb::Position> for Position {}

impl TryFrom<pb::Position> for Position {
    type Error = anyhow::Error;

    fn try_from(value: pb::Position) -> Result<Self, Self::Error> {
        Ok(Self {
            phi: value
                .phi
                .ok_or_else(|| anyhow::anyhow!("missing trading function"))?
                .try_into()?,
            pair: value
                .pair
                .ok_or_else(|| anyhow::anyhow!("missing trading pair"))?
                .try_into()?,
            nonce: value
                .nonce
                .as_slice()
                .try_into()
                .context("expected 32-byte nonce")?,
        })
    }
}

impl From<Position> for pb::Position {
    fn from(value: Position) -> Self {
        Self {
            phi: Some(value.phi.into()),
            pair: Some(value.pair.into()),
            nonce: value.nonce.to_vec(),
        }
    }
}

impl Protobuf<pb::PositionId> for Id {}

impl TryFrom<pb::PositionId> for Id {
    type Error = anyhow::Error;

    fn try_from(value: pb::PositionId) -> Result<Self, Self::Error> {
        Ok(Self(
            value
                .inner
                .as_slice()
                .try_into()
                .context("expected 32-byte id")?,
        ))
    }
}

impl From<Id> for pb::PositionId {
    fn from(value: Id) -> Self {
        Self {
            inner: value.0.to_vec(),
        }
    }
}

impl Protobuf<pb::PositionState> for State {}

impl From<State> for pb::PositionState {
    fn from(v: State) -> Self {
        pb::PositionState {
            state: match v {
                State::Opened => pb::position_state::PositionStateEnum::Opened,
                State::Closed => pb::position_state::PositionStateEnum::Closed,
                State::Withdrawn => pb::position_state::PositionStateEnum::Withdrawn,
                State::Claimed => pb::position_state::PositionStateEnum::Claimed,
            } as i32,
        }
    }
}

impl TryFrom<pb::PositionState> for State {
    type Error = anyhow::Error;
    fn try_from(v: pb::PositionState) -> Result<Self, Self::Error> {
        Ok(
            match pb::position_state::PositionStateEnum::from_i32(v.state)
                .ok_or_else(|| anyhow::anyhow!("missing position state"))?
            {
                pb::position_state::PositionStateEnum::Opened => State::Opened,
                pb::position_state::PositionStateEnum::Closed => State::Closed,
                pb::position_state::PositionStateEnum::Withdrawn => State::Withdrawn,
                pb::position_state::PositionStateEnum::Claimed => State::Claimed,
            },
        )
    }
}
