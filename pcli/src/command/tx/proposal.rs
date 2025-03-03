use penumbra_transaction::action::{ProposalKind, Vote};

#[derive(Debug, clap::Subcommand)]
pub enum ProposalCmd {
    /// Make a template file for a new proposal.
    Template {
        /// The file to output the template to.
        #[clap(long)]
        file: Option<camino::Utf8PathBuf>,
        /// The kind of the proposal to template [one of: signaling, emergency, parameter-change, or dao-spend].
        #[clap(long)]
        kind: ProposalKind,
    },
    /// Submit a new governance proposal.
    Submit {
        /// The proposal to vote on, in JSON format.
        #[clap(long)]
        file: camino::Utf8PathBuf,
        /// The transaction fee (paid in upenumbra).
        #[clap(long, default_value = "0")]
        fee: u64,
        /// Optional. Only spend funds originally received by the given address index.
        #[clap(long)]
        source: Option<u64>,
    },
    /// Withdraw a governance proposal that you previously submitted.
    Withdraw {
        /// The transaction fee (paid in upenumbra).
        #[clap(long, default_value = "0")]
        fee: u64,
        /// The proposal id to withdraw.
        proposal_id: u64,
        /// A short description of the reason for the proposal being withdrawn, meant to be
        /// displayed to users.
        #[clap(long)]
        reason: String,
        /// Optional. Only spend funds originally received by the given address index.
        #[clap(long)]
        source: Option<u64>,
    },
    /// Vote on a governance proposal (in your role as a delegator).
    ///
    /// To vote on a proposal as a validator, use `pcli validator vote`.
    Vote {
        /// The transaction fee (paid in upenumbra).
        #[clap(long, default_value = "0")]
        fee: u64,
        /// The proposal id to vote on.
        #[clap(long = "on")]
        proposal_id: u64,
        /// The vote to cast.
        vote: Vote,
        /// Optional. Only spend funds originally received by the given address index.
        #[clap(long)]
        source: Option<u64>,
    },
}

impl ProposalCmd {
    pub fn needs_sync(&self) -> bool {
        match self {
            ProposalCmd::Template { .. } => false,
            ProposalCmd::Submit { .. } => true,
            ProposalCmd::Withdraw { .. } => true,
            ProposalCmd::Vote { .. } => true,
        }
    }
}
