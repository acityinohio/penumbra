use ark_ff::UniformRand;
use decaf377_rdsa::{Signature, SpendAuth};
use penumbra_crypto::{
    proofs::transparent::SpendProof, Address, FieldExt, Fq, Fr, FullViewingKey, Note, Value,
    STAKING_TOKEN_ASSET_ID,
};
use penumbra_proto::{transaction as pb, Protobuf};
use penumbra_tct as tct;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::action::{spend, Spend};

/// A planned [`Spend`](Spend).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "pb::SpendPlan", into = "pb::SpendPlan")]
pub struct SpendPlan {
    pub note: Note,
    pub position: tct::Position,
    pub randomizer: Fr,
    pub value_blinding: Fr,
}

impl SpendPlan {
    /// Create a new [`SpendPlan`] that spends the given `position`ed `note`.
    pub fn new<R: CryptoRng + RngCore>(
        rng: &mut R,
        note: Note,
        position: tct::Position,
    ) -> SpendPlan {
        SpendPlan {
            note,
            position,
            randomizer: Fr::rand(rng),
            value_blinding: Fr::rand(rng),
        }
    }

    /// Create a dummy [`SpendPlan`].
    pub fn dummy<R: CryptoRng + RngCore>(rng: &mut R) -> SpendPlan {
        let dummy_address = Address::dummy(rng);
        let note_blinding = Fq::rand(rng);
        let dummy_note = Note::from_parts(
            dummy_address,
            Value {
                amount: 0,
                asset_id: *STAKING_TOKEN_ASSET_ID,
            },
            note_blinding,
        )
        .expect("dummy note is valid");

        Self::new(rng, dummy_note, 0u64.into())
    }

    /// Convenience method to construct the [`Spend`] described by this [`SpendPlan`].
    pub fn spend(
        &self,
        fvk: &FullViewingKey,
        auth_sig: Signature<SpendAuth>,
        auth_path: tct::Proof,
    ) -> Spend {
        Spend {
            body: self.spend_body(fvk),
            auth_sig,
            proof: self.spend_proof(fvk, auth_path),
        }
    }

    /// Construct the [`spend::Body`] described by this [`SpendPlan`].
    pub fn spend_body(&self, fvk: &FullViewingKey) -> spend::Body {
        spend::Body {
            value_commitment: self.note.value().commit(self.value_blinding),
            nullifier: fvk.derive_nullifier(self.position, &self.note.commit()),
            rk: fvk.spend_verification_key().randomize(&self.randomizer),
        }
    }

    /// Construct the [`SpendProof`] required by the [`spend::Body`] described by this [`SpendPlan`].
    pub fn spend_proof(
        &self,
        fvk: &FullViewingKey,
        note_commitment_proof: tct::Proof,
    ) -> SpendProof {
        SpendProof {
            note_commitment_proof,
            g_d: self.note.diversified_generator(),
            pk_d: *self.note.transmission_key(),
            ck_d: *self.note.clue_key(),
            value: self.note.value(),
            v_blinding: self.value_blinding,
            note_blinding: self.note.note_blinding(),
            spend_auth_randomizer: self.randomizer,
            ak: *fvk.spend_verification_key(),
            nk: *fvk.nullifier_key(),
        }
    }
}

impl Protobuf<pb::SpendPlan> for SpendPlan {}

impl From<SpendPlan> for pb::SpendPlan {
    fn from(msg: SpendPlan) -> Self {
        Self {
            note: Some(msg.note.into()),
            position: u64::from(msg.position),
            randomizer: msg.randomizer.to_bytes().to_vec().into(),
            value_blinding: msg.value_blinding.to_bytes().to_vec().into(),
        }
    }
}

impl TryFrom<pb::SpendPlan> for SpendPlan {
    type Error = anyhow::Error;
    fn try_from(msg: pb::SpendPlan) -> Result<Self, Self::Error> {
        Ok(Self {
            note: msg
                .note
                .ok_or_else(|| anyhow::anyhow!("missing note"))?
                .try_into()?,
            position: msg.position.into(),
            randomizer: Fr::from_bytes(msg.randomizer.as_ref().try_into()?)?,
            value_blinding: Fr::from_bytes(msg.value_blinding.as_ref().try_into()?)?,
        })
    }
}
