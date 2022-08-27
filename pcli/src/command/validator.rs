use std::{fs::File, io::Write};

use anyhow::{Context, Result};
use penumbra_component::stake::{validator, validator::Validator, FundingStream, FundingStreams};
use penumbra_crypto::IdentityKey;
use penumbra_proto::{stake::Validator as ProtoValidator, Message};
use penumbra_wallet::plan;
use rand_core::OsRng;

use crate::App;

#[derive(Debug, clap::Subcommand)]
pub enum ValidatorCmd {
    /// Display the validator identity key derived from this wallet's spend seed.
    Identity,
    /// Manage your validator's definition.
    #[clap(subcommand)]
    Definition(DefinitionCmd),
}

#[derive(Debug, clap::Subcommand)]
pub enum DefinitionCmd {
    /// Create a ValidatorDefinition transaction to create or update a validator.
    Upload {
        /// The JSON file containing the ValidatorDefinition to upload.
        #[clap(long)]
        file: String,
        /// The transaction fee (paid in upenumbra).
        #[clap(long, default_value = "0")]
        fee: u64,
        /// Optional. Only spend funds originally received by the given address index.
        #[clap(long)]
        source: Option<u64>,
    },
    /// Generates a template validator definition for editing.
    ///
    /// The validator identity field will be prepopulated with the validator
    /// identity key derived from this wallet's seed phrase.
    Template {
        /// The JSON file to write the template to [default: stdout].
        #[clap(long)]
        file: Option<String>,
    },
    /// Fetches the definition for your validator.
    Fetch {
        /// The JSON file to write the definition to [default: stdout].
        #[clap(long)]
        file: Option<String>,
    },
}

impl ValidatorCmd {
    pub fn needs_sync(&self) -> bool {
        match self {
            ValidatorCmd::Identity => false,
            ValidatorCmd::Definition(DefinitionCmd::Upload { .. }) => true,
            ValidatorCmd::Definition(
                DefinitionCmd::Template { .. } | DefinitionCmd::Fetch { .. },
            ) => false,
        }
    }

    // TODO: move use of sk into custody service
    pub async fn exec(&self, app: &mut App) -> Result<()> {
        let sk = app.wallet.spend_key.clone();
        let fvk = sk.full_viewing_key().clone();
        match self {
            ValidatorCmd::Identity => {
                let ik = IdentityKey(fvk.spend_verification_key().clone());

                println!("{}", ik);
            }
            ValidatorCmd::Definition(DefinitionCmd::Upload { file, fee, source }) => {
                // The definitions are stored in a JSON document,
                // however for ease of use it's best for us to generate
                // the signature here based on the configured wallet.
                //
                // TODO: eventually we'll probably want to support defining the
                // identity key in the JSON file.
                //
                // We could also support defining multiple validators in a single
                // file.
                let definition_file =
                    File::open(&file).with_context(|| format!("cannot open file {:?}", file))?;
                let new_validator: Validator = serde_json::from_reader(definition_file)
                    .map_err(|_| anyhow::anyhow!("Unable to parse validator definition"))?;

                // Sign the validator definition with the wallet's spend key.
                let protobuf_serialized: ProtoValidator = new_validator.clone().into();
                let v_bytes = protobuf_serialized.encode_to_vec();
                let auth_sig = sk.spend_auth_key().sign(&mut OsRng, &v_bytes);
                let vd = validator::Definition {
                    validator: new_validator,
                    auth_sig,
                };
                // Construct a new transaction and include the validator definition.
                let plan =
                    plan::validator_definition(&app.fvk, &mut app.view, OsRng, vd, *fee, *source)
                        .await?;
                app.build_and_submit_transaction(plan).await?;
                // Only commit the state if the transaction was submitted
                // successfully, so that we don't store pending notes that will
                // never appear on-chain.
                println!("Uploaded validator definition");
            }
            ValidatorCmd::Definition(DefinitionCmd::Template { file }) => {
                let (address, _dtk) = fvk.incoming().payment_address(0u64.into());
                let identity_key = IdentityKey(fvk.spend_verification_key().clone());
                // Generate a random consensus key.
                // TODO: not great because the private key is discarded here and this isn't obvious to the user
                let consensus_key =
                    tendermint::PrivateKey::Ed25519(ed25519_consensus::SigningKey::new(OsRng))
                        .public_key();

                let template = Validator {
                    identity_key,
                    consensus_key,
                    name: String::new(),
                    website: String::new(),
                    description: String::new(),
                    // Default enabled to "false" so operators are required to manually
                    // enable their validators when ready.
                    enabled: false,
                    funding_streams: FundingStreams::try_from(vec![FundingStream {
                        address,
                        rate_bps: 100,
                    }])?,
                    sequence_number: 0,
                };

                if let Some(file) = file {
                    File::create(file)
                        .with_context(|| format!("cannot create file {:?}", file))?
                        .write_all(&serde_json::to_vec_pretty(&template)?)
                        .context("could not write file")?;
                } else {
                    println!("{}", serde_json::to_string_pretty(&template)?);
                }
            }
            ValidatorCmd::Definition(DefinitionCmd::Fetch { file }) => {
                let identity_key = IdentityKey(fvk.spend_verification_key().clone());
                super::query::ValidatorCmd::Definition {
                    file: file.clone(),
                    identity_key: identity_key.to_string(),
                }
                .exec(app)
                .await?;
            }
        }

        Ok(())
    }
}
