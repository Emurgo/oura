use std::collections::HashMap;
use std::fmt::Display;

use merge::Merge;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use strum_macros::Display;

// We're duplicating the Era struct from Pallas for two reasons: a) we need it
// to be serializable and we don't want to impose serde dependency on Pallas and
// b) we prefer not to add dependencies to Pallas outside of the sources that
// actually use it on an attempt to make the pipeline agnostic of particular
// implementation details.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Display)]
pub enum Era {
    Undefined,
    Unknown,
    Byron,
    Shelley,
    Allegra,
    Mary,
    Alonzo,
    Babbage,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetadatumRendition {
    MapJson(JsonValue),
    ArrayJson(JsonValue),
    IntScalar(i128),
    TextScalar(String),
    BytesHex(String),
}

impl Display for MetadatumRendition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadatumRendition::MapJson(x) => x.fmt(f),
            MetadatumRendition::ArrayJson(x) => x.fmt(f),
            MetadatumRendition::IntScalar(x) => x.fmt(f),
            MetadatumRendition::TextScalar(x) => x.fmt(f),
            MetadatumRendition::BytesHex(x) => x.fmt(f),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MetadataRecord {
    pub label: String,

    #[serde(flatten)]
    pub content: MetadatumRendition,
}

impl From<MetadataRecord> for EventData {
    fn from(x: MetadataRecord) -> Self {
        EventData::Metadata(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CIP25AssetRecord {
    pub version: String,
    pub policy: String,
    pub asset: String,
    pub name: Option<String>,
    pub image: Option<String>,
    pub media_type: Option<String>,
    pub description: Option<String>,
    pub raw_json: JsonValue,
}

impl From<CIP25AssetRecord> for EventData {
    fn from(x: CIP25AssetRecord) -> Self {
        EventData::CIP25Asset(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CIP15AssetRecord {
    pub voting_key: String,
    pub stake_pub: String,
    pub reward_address: String,
    pub nonce: i64,
    pub raw_json: JsonValue,
}

impl From<CIP15AssetRecord> for EventData {
    fn from(x: CIP15AssetRecord) -> Self {
        EventData::CIP15Asset(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TxInputRecord {
    pub tx_id: String,
    pub index: u64,
}

impl From<TxInputRecord> for EventData {
    fn from(x: TxInputRecord) -> Self {
        EventData::TxInput(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputAssetRecord {
    pub policy: String,
    pub asset: String,
    pub asset_ascii: Option<String>,
    pub amount: u64,
}

impl From<OutputAssetRecord> for EventData {
    fn from(x: OutputAssetRecord) -> Self {
        EventData::OutputAsset(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TxOutputRecord {
    pub address: String,
    pub amount: u64,
    pub assets: Option<Vec<OutputAssetRecord>>,
    pub datum_hash: Option<String>,
    pub inline_datum: Option<PlutusDatumRecord>,
    pub inlined_script: Option<ScriptRefRecord>,
}

impl From<TxOutputRecord> for EventData {
    fn from(x: TxOutputRecord) -> Self {
        EventData::TxOutput(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MintRecord {
    pub policy: String,
    pub asset: String,
    pub quantity: i64,
}

impl From<MintRecord> for EventData {
    fn from(x: MintRecord) -> Self {
        EventData::Mint(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WithdrawalRecord {
    pub reward_account: String,
    pub coin: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct TransactionRecord {
    pub hash: String,
    pub fee: u64,
    pub ttl: Option<u64>,
    pub validity_interval_start: Option<u64>,
    pub network_id: Option<u32>,
    pub input_count: usize,
    pub collateral_input_count: usize,
    pub has_collateral_output: bool,
    pub output_count: usize,
    pub mint_count: usize,
    pub certificate_count: usize,
    pub total_output: u64,
    pub required_signers_count: usize,

    // include_details
    pub required_signers: Option<Vec<RequiredSignerRecord>>,
    pub update: Option<UpdateRecord>,
    pub metadata: Option<Vec<MetadataRecord>>,
    pub inputs: Option<Vec<TxInputRecord>>,
    pub outputs: Option<Vec<TxOutputRecord>>,
    pub collateral_inputs: Option<Vec<TxInputRecord>>,
    pub collateral_output: Option<TxOutputRecord>,
    pub certs: Option<Vec<CertificateRecord>>,
    pub mint: Option<Vec<MintRecord>>,
    pub vkey_witnesses: Option<Vec<VKeyWitnessRecord>>,
    pub native_witnesses: Option<Vec<NativeWitnessRecord>>,
    pub plutus_witnesses: Option<Vec<PlutusWitnessRecord>>,
    pub plutus_redeemers: Option<Vec<PlutusRedeemerRecord>>,
    pub plutus_data: Option<Vec<PlutusDatumRecord>>,
    pub withdrawals: Option<Vec<WithdrawalRecord>>,
    pub size: u32,
}

impl From<TransactionRecord> for EventData {
    fn from(x: TransactionRecord) -> Self {
        EventData::Transaction(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Merge, Default)]
pub struct EventContext {
    pub block_hash: Option<String>,
    pub block_number: Option<u64>,
    pub slot: Option<u64>,
    pub timestamp: Option<u64>,
    pub tx_idx: Option<usize>,
    pub tx_hash: Option<String>,
    pub input_idx: Option<usize>,
    pub output_idx: Option<usize>,
    pub output_address: Option<String>,
    pub certificate_idx: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StakeCredential {
    AddrKeyhash(String),
    Scripthash(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ScriptRefRecord {
    PlutusV1 {
        script_hash: String,
        script_hex: String,
    },
    PlutusV2 {
        script_hash: String,
        script_hex: String,
    },
    PlutusV3 {
        script_hash: String,
        script_hex: String,
    },
    NativeScript {
        policy_id: String,
        script_json: JsonValue,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum CertificateRecord {
    StakeRegistration(StakeRegistrationRecord),
    StakeDeregistration(StakeDeregistrationRecord),
    StakeDelegation(StakeDelegationRecord),
    PoolRegistration(PoolRegistrationRecord),
    PoolRetirement(PoolRetirementRecord),
    GenesisKeyDelegation(GenesisKeyDelegationRecord),
    MoveInstantaneousRewardsCert(MoveInstantaneousRewardsCertRecord),
    RegCert(RegCertRecord),
    UnRegCert(UnRegCertRecord),
    VoteDeleg(VoteDelegCertRecord),
    StakeVoteDeleg(StakeVoteDelegCertRecord),
    StakeRegDeleg(StakeRegDelegCertRecord),
    VoteRegDeleg(VoteRegDelegCertRecord),
    StakeVoteRegDeleg(StakeVoteRegDelegCertRecord),
    AuthCommitteeHot(AuthCommitteeHotCertRecord),
    ResignCommitteeCold(ResignCommitteeColdCertRecord),
    RegDRepCert(RegDRepCertRecord),
    UnRegDRepCert(UnRegDRepCertRecord),
    UpdateDRepCert(UpdateDRepCertRecord),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct StakeRegistrationRecord {
    pub credential: StakeCredential,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct StakeDeregistrationRecord {
    pub credential: StakeCredential,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct StakeDelegationRecord {
    pub credential: StakeCredential,
    pub pool_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct PoolRegistrationRecord {
    pub operator: String,
    pub vrf_keyhash: String,
    pub pledge: u64,
    pub cost: u64,
    pub margin: RationalNumberRecord,
    pub reward_account: String,
    pub pool_owners: Vec<String>,
    pub relays: Vec<String>,
    pub pool_metadata: Option<String>,
    pub pool_metadata_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct PoolRetirementRecord {
    pub pool: String,
    pub epoch: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct GenesisKeyDelegationRecord {
    pub genesis_hash: String,
    pub genesis_delegate_hash: String,
    pub vrf_key_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DRep {
    KeyHash(String),
    ScriptHash(String),
    Abstain,
    NoConfidence,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MoveInstantaneousRewardsCertRecord {
    pub from_reserves: bool,
    pub from_treasury: bool,
    pub to_stake_credentials: Option<Vec<(StakeCredential, i64)>>,
    pub to_other_pot: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegCertRecord {
    pub credential: StakeCredential,
    pub coin: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnRegCertRecord {
    pub credential: StakeCredential,
    pub coin: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VoteDelegCertRecord {
    pub credential: StakeCredential,
    pub drep: DRep,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StakeVoteDelegCertRecord {
    pub credential: StakeCredential,
    pub pool_keyhash: String,
    pub drep: DRep,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StakeRegDelegCertRecord {
    pub credential: StakeCredential,
    pub pool_keyhash: String,
    pub coin: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VoteRegDelegCertRecord {
    pub credential: StakeCredential,
    pub drep: DRep,
    pub coin: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StakeVoteRegDelegCertRecord {
    pub credential: StakeCredential,
    pub pool_keyhash: String,
    pub drep: DRep,
    pub coin: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuthCommitteeHotCertRecord {
    pub committee_cold_credential: StakeCredential,
    pub committee_hot_credential: StakeCredential,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResignCommitteeColdCertRecord {
    pub committee_cold_credential: StakeCredential,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegDRepCertRecord {
    pub credential: StakeCredential,
    pub coin: u64,
    pub anchor: Option<AnchorRecord>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnRegDRepCertRecord {
    pub credential: StakeCredential,
    pub coin: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UpdateDRepCertRecord {
    pub credential: StakeCredential,
    pub anchor: Option<AnchorRecord>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnchorRecord {
    pub url: String,
    pub data_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RationalNumberRecord {
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnitIntervalRecord(pub u64, pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositiveIntervalRecord(pub u64, pub u64);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExUnitsRecord {
    pub mem: u32,
    pub steps: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExUnitPricesRecord {
    pub mem_price: PositiveIntervalRecord,
    pub step_price: PositiveIntervalRecord,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NonceRecord {
    pub variant: NonceVariantRecord,
    pub hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NonceVariantRecord {
    NeutralNonce,
    Nonce,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LanguageVersionRecord {
    PlutusV1,
    PlutusV2,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CostModelRecord(pub Vec<i64>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CostModelsRecord(pub HashMap<LanguageVersionRecord, CostModelRecord>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VKeyWitnessRecord {
    pub vkey_hex: String,
    pub signature_hex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RequiredSignerRecord(pub String);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct NativeWitnessRecord {
    pub policy_id: String,
    pub script_json: JsonValue,
}

impl From<NativeWitnessRecord> for EventData {
    fn from(x: NativeWitnessRecord) -> Self {
        EventData::NativeWitness(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PlutusWitnessRecord {
    pub script_hash: String,
    pub script_hex: String,
}

impl From<PlutusWitnessRecord> for EventData {
    fn from(x: PlutusWitnessRecord) -> Self {
        EventData::PlutusWitness(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PlutusRedeemerRecord {
    pub purpose: String,
    pub ex_units_mem: u32,
    pub ex_units_steps: u64,
    pub input_idx: u32,
    pub plutus_data: JsonValue,
}

impl From<PlutusRedeemerRecord> for EventData {
    fn from(x: PlutusRedeemerRecord) -> Self {
        EventData::PlutusRedeemer(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PlutusDatumRecord {
    pub datum_hash: String,
    pub plutus_data: JsonValue,
}

impl From<PlutusDatumRecord> for EventData {
    fn from(x: PlutusDatumRecord) -> Self {
        EventData::PlutusDatum(x)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BlockRecord {
    pub era: Era,
    pub epoch: Option<u64>,
    pub epoch_slot: Option<u64>,
    pub body_size: usize,
    pub issuer_vkey: String,
    pub vrf_vkey: String,
    pub tx_count: usize,
    pub slot: u64,
    pub hash: String,
    pub number: u64,
    pub previous_hash: String,
    pub cbor_hex: Option<String>,
    pub transactions: Option<Vec<TransactionRecord>>,
}

impl From<BlockRecord> for EventData {
    fn from(x: BlockRecord) -> Self {
        EventData::Block(x)
    }
}

#[derive(Serialize, Deserialize, Display, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EventData {
    Block(BlockRecord),
    BlockEnd(BlockRecord),
    Transaction(TransactionRecord),
    TransactionEnd(TransactionRecord),
    TxInput(TxInputRecord),
    TxOutput(TxOutputRecord),
    OutputAsset(OutputAssetRecord),
    Metadata(MetadataRecord),

    VKeyWitness(VKeyWitnessRecord),
    NativeWitness(NativeWitnessRecord),
    PlutusWitness(PlutusWitnessRecord),
    PlutusRedeemer(PlutusRedeemerRecord),
    PlutusDatum(PlutusDatumRecord),

    #[serde(rename = "cip25_asset")]
    CIP25Asset(CIP25AssetRecord),

    #[serde(rename = "cip15_asset")]
    CIP15Asset(CIP15AssetRecord),

    Mint(MintRecord),
    Collateral {
        tx_id: String,
        index: u64,
    },
    NativeScript {
        policy_id: String,
        script: JsonValue,
    },
    PlutusScript {
        hash: String,
        data: String,
    },
    StakeRegistration(StakeRegistrationRecord),
    StakeDeregistration(StakeDeregistrationRecord),
    StakeDelegation(StakeDelegationRecord),
    PoolRegistration(PoolRegistrationRecord),
    PoolRetirement(PoolRetirementRecord),
    GenesisKeyDelegation(GenesisKeyDelegationRecord),
    MoveInstantaneousRewardsCert(MoveInstantaneousRewardsCertRecord),
    RegCert(RegCertRecord),
    UnRegCert(UnRegCertRecord),
    VoteDeleg(VoteDelegCertRecord),
    StakeVoteDeleg(StakeVoteDelegCertRecord),
    StakeRegDeleg(StakeRegDelegCertRecord),
    VoteRegDeleg(VoteRegDelegCertRecord),
    StakeVoteRegDeleg(StakeVoteRegDelegCertRecord),
    AuthCommitteeHot(AuthCommitteeHotCertRecord),
    ResignCommitteeCold(ResignCommitteeColdCertRecord),
    RegDRepCert(RegDRepCertRecord),
    UnRegDRepCert(UnRegDRepCertRecord),
    UpdateDRepCert(UpdateDRepCertRecord),

    RollBack {
        block_slot: u64,
        block_hash: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProtocolParamUpdateRecord {
    pub minfee_a: Option<u32>,
    pub minfee_b: Option<u32>,
    pub max_block_body_size: Option<u32>,
    pub max_transaction_size: Option<u32>,
    pub max_block_header_size: Option<u32>,
    pub key_deposit: Option<u64>,
    pub pool_deposit: Option<u64>,
    pub maximum_epoch: Option<u64>,
    pub desired_number_of_stake_pools: Option<u32>,
    pub pool_pledge_influence: Option<RationalNumberRecord>,
    pub expansion_rate: Option<UnitIntervalRecord>,
    pub treasury_growth_rate: Option<UnitIntervalRecord>,
    pub decentralization_constant: Option<UnitIntervalRecord>,
    pub extra_entropy: Option<NonceRecord>,
    pub protocol_version: Option<(u64, u64)>,
    pub min_pool_cost: Option<u64>,
    pub ada_per_utxo_byte: Option<u64>,
    pub cost_models_for_script_languages: Option<CostModelsRecord>,
    pub execution_costs: Option<JsonValue>,
    pub max_tx_ex_units: Option<ExUnitsRecord>,
    pub max_block_ex_units: Option<ExUnitsRecord>,
    pub max_value_size: Option<u32>,
    pub collateral_percentage: Option<u32>,
    pub max_collateral_inputs: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UpdateRecord {
    pub proposed_protocol_parameter_updates: HashMap<String, ProtocolParamUpdateRecord>,
    pub epoch: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub context: EventContext,

    #[serde(flatten)]
    pub data: EventData,

    pub fingerprint: Option<String>,
}
