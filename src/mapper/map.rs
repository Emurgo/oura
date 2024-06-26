use std::collections::HashMap;

use pallas::ledger::primitives::alonzo::{
    CostMdls, CostModel, ExUnits, Language, MintedWitnessSet, Nonce, NonceVariant,
    PositiveInterval, ProtocolParamUpdate, RationalNumber, UnitInterval, Update,
};
use pallas::ledger::primitives::babbage::{MintedDatumOption, Script, ScriptRef};
use pallas::ledger::traverse::{ComputeHash, OriginalHash};
use pallas::{codec::utils::KeepRaw, crypto::hash::Hash};

use pallas::ledger::primitives::{
    alonzo::{
        self as alonzo, AuxiliaryData, Certificate, InstantaneousRewardSource,
        InstantaneousRewardTarget, Metadatum, MetadatumLabel, MintedBlock, NetworkId, Relay,
        TransactionBody, TransactionInput, Value,
    },
    babbage, ToCanonicalJson,
};

use pallas::network::miniprotocols::Point;
use serde_json::{json, Value as JsonValue};

use crate::model::{
    AnchorRecord, AuthCommitteeHotCertRecord, BlockRecord, CertificateRecord, CostModelRecord,
    CostModelsRecord, DRep, Era, EventData, ExUnitsRecord, GenesisKeyDelegationRecord,
    LanguageVersionRecord, MetadataRecord, MetadatumRendition, MintRecord,
    MoveInstantaneousRewardsCertRecord, NativeWitnessRecord, NonceRecord, NonceVariantRecord,
    OutputAssetRecord, PlutusDatumRecord, PlutusRedeemerRecord, PlutusWitnessRecord,
    PoolRegistrationRecord, PoolRetirementRecord, PositiveIntervalRecord,
    ProtocolParamUpdateRecord, RationalNumberRecord, RegCertRecord, RegDRepCertRecord,
    ResignCommitteeColdCertRecord, ScriptRefRecord, StakeCredential, StakeDelegationRecord,
    StakeDeregistrationRecord, StakeRegDelegCertRecord, StakeRegistrationRecord,
    StakeVoteDelegCertRecord, StakeVoteRegDelegCertRecord, TransactionRecord, TxInputRecord,
    TxOutputRecord, UnRegCertRecord, UnRegDRepCertRecord, UnitIntervalRecord, UpdateDRepCertRecord,
    UpdateRecord, VKeyWitnessRecord, VoteDelegCertRecord, VoteRegDelegCertRecord,
};

use crate::model::ScriptRefRecord::{NativeScript, PlutusV1, PlutusV2, PlutusV3};
use crate::utils::time::TimeProvider;
use crate::Error;

use super::EventWriter;

pub trait ToHex {
    fn to_hex(&self) -> String;
}

impl ToHex for Vec<u8> {
    fn to_hex(&self) -> String {
        hex::encode(self)
    }
}

impl ToHex for &[u8] {
    fn to_hex(&self) -> String {
        hex::encode(self)
    }
}

impl<const BYTES: usize> ToHex for Hash<BYTES> {
    fn to_hex(&self) -> String {
        hex::encode(self)
    }
}

impl From<&alonzo::StakeCredential> for StakeCredential {
    fn from(other: &alonzo::StakeCredential) -> Self {
        match other {
            alonzo::StakeCredential::AddrKeyhash(x) => StakeCredential::AddrKeyhash(x.to_hex()),
            alonzo::StakeCredential::Scripthash(x) => StakeCredential::Scripthash(x.to_hex()),
        }
    }
}

impl From<&alonzo::DRep> for DRep {
    fn from(other: &alonzo::DRep) -> Self {
        match other {
            alonzo::DRep::Key(x) => DRep::KeyHash(x.to_hex()),
            alonzo::DRep::Script(x) => DRep::ScriptHash(x.to_hex()),
            alonzo::DRep::Abstain => DRep::Abstain,
            alonzo::DRep::NoConfidence => DRep::NoConfidence,
        }
    }
}

impl From<&alonzo::Anchor> for AnchorRecord {
    fn from(other: &alonzo::Anchor) -> Self {
        AnchorRecord {
            url: other.0.clone(),
            data_hash: other.1.to_hex(),
        }
    }
}

fn to_option_anchor_record(anchor: &Option<alonzo::Anchor>) -> Option<AnchorRecord> {
    match anchor {
        Some(anchor) => Some(anchor.into()),
        None => None,
    }
}

fn ip_string_from_bytes(bytes: &[u8]) -> String {
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

fn relay_to_string(relay: &Relay) -> String {
    match relay {
        Relay::SingleHostAddr(port, ipv4, ipv6) => {
            let ip = match (ipv6, ipv4) {
                (None, None) => "".to_string(),
                (_, Some(x)) => ip_string_from_bytes(x.as_ref()),
                (Some(x), _) => ip_string_from_bytes(x.as_ref()),
            };

            match port {
                Some(port) => format!("{ip}:{port}"),
                None => ip,
            }
        }
        Relay::SingleHostName(port, host) => match port {
            Some(port) => format!("{host}:{port}"),
            None => host.clone(),
        },
        Relay::MultiHostName(host) => host.clone(),
    }
}

fn metadatum_to_string_key(datum: &Metadatum) -> String {
    match datum {
        Metadatum::Int(x) => x.to_string(),
        Metadatum::Bytes(x) => hex::encode(x.as_slice()),
        Metadatum::Text(x) => x.to_owned(),
        x => {
            log::warn!("unexpected metadatum type for label: {:?}", x);
            Default::default()
        }
    }
}

fn get_tx_output_coin_value(amount: &Value) -> u64 {
    match amount {
        Value::Coin(x) => *x,
        Value::Multiasset(x, _) => *x,
    }
}

impl EventWriter {
    pub fn to_metadatum_json_map_entry(
        &self,
        pair: (&Metadatum, &Metadatum),
    ) -> Result<(String, JsonValue), Error> {
        let key = metadatum_to_string_key(pair.0);
        let value = self.to_metadatum_json(pair.1)?;
        Ok((key, value))
    }

    pub fn to_metadatum_json(&self, source: &Metadatum) -> Result<JsonValue, Error> {
        match source {
            Metadatum::Int(x) => Ok(json!(i128::from(*x))),
            Metadatum::Bytes(x) => Ok(json!(hex::encode(x.as_slice()))),
            Metadatum::Text(x) => Ok(json!(x)),
            Metadatum::Array(x) => {
                let items: Result<Vec<_>, _> =
                    x.iter().map(|x| self.to_metadatum_json(x)).collect();

                Ok(json!(items?))
            }
            Metadatum::Map(x) => {
                let map: Result<HashMap<_, _>, _> = x
                    .iter()
                    .map(|(key, value)| self.to_metadatum_json_map_entry((key, value)))
                    .collect();

                Ok(json!(map?))
            }
        }
    }

    pub fn to_metadata_record(
        &self,
        label: &MetadatumLabel,
        value: &Metadatum,
    ) -> Result<MetadataRecord, Error> {
        let data = MetadataRecord {
            label: label.to_string(),
            content: match value {
                Metadatum::Int(x) => MetadatumRendition::IntScalar(i128::from(*x)),
                Metadatum::Bytes(x) => MetadatumRendition::BytesHex(hex::encode(x.as_slice())),
                Metadatum::Text(x) => MetadatumRendition::TextScalar(x.clone()),
                Metadatum::Array(_) => {
                    MetadatumRendition::ArrayJson(self.to_metadatum_json(value)?)
                }
                Metadatum::Map(_) => MetadatumRendition::MapJson(self.to_metadatum_json(value)?),
            },
        };

        Ok(data)
    }

    pub fn to_transaction_input_record(&self, input: &TransactionInput) -> TxInputRecord {
        TxInputRecord {
            tx_id: input.transaction_id.to_hex(),
            index: input.index,
        }
    }

    pub fn to_legacy_output_record(
        &self,
        output: &alonzo::TransactionOutput,
    ) -> Result<TxOutputRecord, Error> {
        let address = pallas::ledger::addresses::Address::from_bytes(&output.address)?;

        Ok(TxOutputRecord {
            address: address.to_string(),
            amount: get_tx_output_coin_value(&output.amount),
            assets: self.collect_asset_records(&output.amount).into(),
            datum_hash: output.datum_hash.map(|hash| hash.to_string()),
            inline_datum: None,
            inlined_script: None,
        })
    }

    pub fn to_post_alonzo_output_record(
        &self,
        output: &babbage::MintedPostAlonzoTransactionOutput,
    ) -> Result<TxOutputRecord, Error> {
        let address = pallas::ledger::addresses::Address::from_bytes(&output.address)?;

        Ok(TxOutputRecord {
            address: address.to_string(),
            amount: get_tx_output_coin_value(&output.value),
            assets: self.collect_asset_records(&output.value).into(),
            datum_hash: match &output.datum_option {
                Some(MintedDatumOption::Hash(x)) => Some(x.to_string()),
                Some(MintedDatumOption::Data(x)) => Some(x.original_hash().to_hex()),
                None => None,
            },
            inline_datum: match &output.datum_option {
                Some(MintedDatumOption::Data(x)) => Some(self.to_plutus_datum_record(x)?),
                _ => None,
            },
            inlined_script: match &output.script_ref {
                Some(script) => Some(self.to_script_ref_record(script)?),
                None => None,
            },
        })
    }

    pub fn to_transaction_output_asset_record(
        &self,
        policy: &Hash<28>,
        asset: &pallas::codec::utils::Bytes,
        amount: u64,
    ) -> OutputAssetRecord {
        OutputAssetRecord {
            policy: policy.to_hex(),
            asset: asset.to_hex(),
            asset_ascii: String::from_utf8(asset.to_vec()).ok(),
            amount,
        }
    }

    pub fn to_mint_record(
        &self,
        policy: &Hash<28>,
        asset: &pallas::codec::utils::Bytes,
        quantity: i64,
    ) -> MintRecord {
        MintRecord {
            policy: policy.to_hex(),
            asset: asset.to_hex(),
            quantity,
        }
    }

    pub fn to_aux_native_script_event(&self, script: &alonzo::NativeScript) -> EventData {
        EventData::NativeScript {
            policy_id: script.compute_hash().to_hex(),
            script: script.to_json(),
        }
    }

    pub fn to_aux_plutus_script_event(&self, script: &alonzo::PlutusScript) -> EventData {
        EventData::PlutusScript {
            hash: script.compute_hash().to_hex(),
            data: script.0.to_hex(),
        }
    }

    pub fn to_plutus_redeemer_record(
        &self,
        redeemer: &alonzo::Redeemer,
    ) -> Result<PlutusRedeemerRecord, crate::Error> {
        Ok(PlutusRedeemerRecord {
            purpose: match redeemer.tag {
                alonzo::RedeemerTag::Spend => "spend".to_string(),
                alonzo::RedeemerTag::Mint => "mint".to_string(),
                alonzo::RedeemerTag::Cert => "cert".to_string(),
                alonzo::RedeemerTag::Reward => "reward".to_string(),
                alonzo::RedeemerTag::Voting => "voting".to_string(),
                alonzo::RedeemerTag::Proposing => "proposing".to_string(),
            },
            ex_units_mem: redeemer.ex_units.mem,
            ex_units_steps: redeemer.ex_units.steps,
            input_idx: redeemer.index,
            plutus_data: redeemer.data.to_json(),
        })
    }

    pub fn to_plutus_datum_record(
        &self,
        datum: &KeepRaw<'_, alonzo::PlutusData>,
    ) -> Result<PlutusDatumRecord, crate::Error> {
        Ok(PlutusDatumRecord {
            datum_hash: datum.original_hash().to_hex(),
            plutus_data: datum.to_json(),
        })
    }

    pub fn to_plutus_v1_witness_record(
        &self,
        script: &alonzo::PlutusScript,
    ) -> Result<PlutusWitnessRecord, crate::Error> {
        Ok(PlutusWitnessRecord {
            script_hash: script.compute_hash().to_hex(),
            script_hex: script.as_ref().to_hex(),
        })
    }

    pub fn to_plutus_v2_witness_record(
        &self,
        script: &babbage::PlutusV2Script,
    ) -> Result<PlutusWitnessRecord, crate::Error> {
        Ok(PlutusWitnessRecord {
            script_hash: script.compute_hash().to_hex(),
            script_hex: script.as_ref().to_hex(),
        })
    }

    pub fn to_native_witness_record(
        &self,
        script: &alonzo::NativeScript,
    ) -> Result<NativeWitnessRecord, crate::Error> {
        Ok(NativeWitnessRecord {
            policy_id: script.compute_hash().to_hex(),
            script_json: script.to_json(),
        })
    }

    pub fn to_script_ref_record(
        &self,
        script_ref: &ScriptRef,
    ) -> Result<ScriptRefRecord, crate::Error> {
        match &script_ref.0 {
            Script::PlutusV1Script(script) => Ok(PlutusV1 {
                script_hash: script.compute_hash().to_hex(),
                script_hex: script.as_ref().to_hex(),
            }),
            Script::PlutusV2Script(script) => Ok(PlutusV2 {
                script_hash: script.compute_hash().to_hex(),
                script_hex: script.as_ref().to_hex(),
            }),
            Script::PlutusV3Script(script) => Ok(PlutusV3 {
                script_hash: script.compute_hash().to_hex(),
                script_hex: script.as_ref().to_hex(),
            }),
            Script::NativeScript(script) => Ok(NativeScript {
                policy_id: script.compute_hash().to_hex(),
                script_json: script.to_json(),
            }),
        }
    }

    pub fn to_vkey_witness_record(
        &self,
        witness: &alonzo::VKeyWitness,
    ) -> Result<VKeyWitnessRecord, crate::Error> {
        Ok(VKeyWitnessRecord {
            vkey_hex: witness.vkey.to_hex(),
            signature_hex: witness.signature.to_hex(),
        })
    }

    pub fn to_certificate_record(&self, certificate: &Certificate) -> CertificateRecord {
        match certificate {
            Certificate::StakeRegistration(credential) => {
                CertificateRecord::StakeRegistration(StakeRegistrationRecord {
                    credential: credential.into(),
                })
            }
            Certificate::StakeDeregistration(credential) => {
                CertificateRecord::StakeDeregistration(StakeDeregistrationRecord {
                    credential: credential.into(),
                })
            }
            Certificate::StakeDelegation(credential, pool) => {
                CertificateRecord::StakeDelegation(StakeDelegationRecord {
                    credential: credential.into(),
                    pool_hash: pool.to_hex(),
                })
            }
            Certificate::PoolRegistration {
                operator,
                vrf_keyhash,
                pledge,
                cost,
                margin,
                reward_account,
                pool_owners,
                relays,
                pool_metadata,
            } => CertificateRecord::PoolRegistration(PoolRegistrationRecord {
                operator: operator.to_hex(),
                vrf_keyhash: vrf_keyhash.to_hex(),
                pledge: *pledge,
                cost: *cost,
                margin: self.to_rational_number_record(margin),
                reward_account: reward_account.to_hex(),
                pool_owners: pool_owners.iter().map(|p| p.to_hex()).collect(),
                relays: relays.iter().map(relay_to_string).collect(),
                pool_metadata: pool_metadata.as_ref().map(|m| m.url.clone()),
                pool_metadata_hash: pool_metadata.as_ref().map(|m| m.hash.clone().to_hex()),
            }),
            Certificate::PoolRetirement(pool, epoch) => {
                CertificateRecord::PoolRetirement(PoolRetirementRecord {
                    pool: pool.to_hex(),
                    epoch: *epoch,
                })
            }
            Certificate::MoveInstantaneousRewardsCert(move_) => {
                CertificateRecord::MoveInstantaneousRewardsCert(
                    MoveInstantaneousRewardsCertRecord {
                        from_reserves: matches!(move_.source, InstantaneousRewardSource::Reserves),
                        from_treasury: matches!(move_.source, InstantaneousRewardSource::Treasury),
                        to_stake_credentials: match &move_.target {
                            InstantaneousRewardTarget::StakeCredentials(creds) => {
                                let x = creds.iter().map(|(k, v)| (k.into(), *v)).collect();
                                Some(x)
                            }
                            _ => None,
                        },
                        to_other_pot: match move_.target {
                            InstantaneousRewardTarget::OtherAccountingPot(x) => Some(x),
                            _ => None,
                        },
                    },
                )
            }
            Certificate::GenesisKeyDelegation(
                genesis_hash,
                genesis_delegate_hash,
                vrf_key_hash,
            ) => CertificateRecord::GenesisKeyDelegation(GenesisKeyDelegationRecord {
                genesis_hash: genesis_hash.to_hex(),
                genesis_delegate_hash: genesis_delegate_hash.to_hex(),
                vrf_key_hash: vrf_key_hash.to_hex(),
            }),
            Certificate::Reg(credential, coin) => CertificateRecord::RegCert(RegCertRecord {
                credential: credential.into(),
                coin: *coin,
            }),
            Certificate::UnReg(credential, coin) => CertificateRecord::UnRegCert(UnRegCertRecord {
                credential: credential.into(),
                coin: *coin,
            }),
            Certificate::VoteDeleg(credential, drep) => {
                CertificateRecord::VoteDeleg(VoteDelegCertRecord {
                    credential: credential.into(),
                    drep: drep.into(),
                })
            }
            Certificate::StakeVoteDeleg(credential, pool, drep) => {
                CertificateRecord::StakeVoteDeleg(StakeVoteDelegCertRecord {
                    credential: credential.into(),
                    pool_keyhash: pool.to_hex(),
                    drep: drep.into(),
                })
            }
            Certificate::StakeRegDeleg(credential, pool, coin) => {
                CertificateRecord::StakeRegDeleg(StakeRegDelegCertRecord {
                    credential: credential.into(),
                    pool_keyhash: pool.to_hex(),
                    coin: *coin,
                })
            }
            Certificate::VoteRegDeleg(credential, drep, coin) => {
                CertificateRecord::VoteRegDeleg(VoteRegDelegCertRecord {
                    credential: credential.into(),
                    drep: drep.into(),
                    coin: *coin,
                })
            }
            Certificate::StakeVoteRegDeleg(credential, pool, drep, coin) => {
                CertificateRecord::StakeVoteRegDeleg(StakeVoteRegDelegCertRecord {
                    credential: credential.into(),
                    pool_keyhash: pool.to_hex(),
                    drep: drep.into(),
                    coin: *coin,
                })
            }
            Certificate::AuthCommitteeHot(cold, hot) => {
                CertificateRecord::AuthCommitteeHot(AuthCommitteeHotCertRecord {
                    committee_cold_credential: cold.into(),
                    committee_hot_credential: hot.into(),
                })
            }
            Certificate::ResignCommitteeCold(cold, anchor) => {
                CertificateRecord::ResignCommitteeCold(ResignCommitteeColdCertRecord {
                    committee_cold_credential: cold.into(),
                    anchor: to_option_anchor_record(anchor),
                })
            }
            Certificate::RegDRepCert(drep, coin, anchor) => {
                CertificateRecord::RegDRepCert(RegDRepCertRecord {
                    credential: drep.into(),
                    coin: *coin,
                    anchor: to_option_anchor_record(anchor),
                })
            }
            Certificate::UnRegDRepCert(drep, coin) => {
                CertificateRecord::UnRegDRepCert(UnRegDRepCertRecord {
                    credential: drep.into(),
                    coin: *coin,
                })
            }
            Certificate::UpdateDRepCert(credential, anchor) => {
                CertificateRecord::UpdateDRepCert(UpdateDRepCertRecord {
                    credential: credential.into(),
                    anchor: to_option_anchor_record(anchor),
                })
            }
        }
    }

    pub fn to_rational_number_record(&self, rational: &RationalNumber) -> RationalNumberRecord {
        RationalNumberRecord {
            numerator: rational.numerator,
            denominator: rational.denominator,
        }
    }

    pub fn to_rational_number_record_option(
        &self,
        rational: &Option<RationalNumber>,
    ) -> Option<RationalNumberRecord> {
        match rational {
            Some(rational) => Some(self.to_rational_number_record(rational)),
            None => None,
        }
    }

    pub fn to_unit_interval_record(
        &self,
        interval: &Option<UnitInterval>,
    ) -> Option<UnitIntervalRecord> {
        match interval {
            Some(interval) => Some(UnitIntervalRecord(
                interval.numerator as u64,
                interval.denominator,
            )),
            None => None,
        }
    }

    pub fn to_positive_interval_record(
        &self,
        interval: &PositiveInterval,
    ) -> PositiveIntervalRecord {
        PositiveIntervalRecord(interval.numerator as u64, interval.denominator)
    }

    pub fn to_nonce_record(&self, nonce: &Option<Nonce>) -> Option<NonceRecord> {
        match nonce {
            Some(nonce) => Some(NonceRecord {
                variant: self.to_nonce_variant_record(&nonce.variant),
                hash: nonce.hash.map(|x| x.to_hex()),
            }),
            None => None,
        }
    }

    pub fn to_cost_models_record(
        &self,
        cost_models: &Option<CostMdls>,
    ) -> Option<CostModelsRecord> {
        match cost_models {
            Some(cost_models) => {
                let mut cost_models_record = HashMap::new();
                for cost_model_pair in cost_models.clone().to_vec() {
                    let language_version_record =
                        self.to_language_version_record(&cost_model_pair.0);
                    let cost_model_record = self.to_cost_model_record(cost_model_pair.1);
                    cost_models_record.insert(language_version_record, cost_model_record);
                }
                Some(CostModelsRecord(cost_models_record))
            }
            None => None,
        }
    }

    pub fn to_language_version_record(&self, language_version: &Language) -> LanguageVersionRecord {
        match language_version {
            Language::PlutusV1 => LanguageVersionRecord::PlutusV1,
        }
    }

    pub fn to_cost_model_record(&self, cost_model: CostModel) -> CostModelRecord {
        CostModelRecord(cost_model)
    }

    pub fn to_nonce_variant_record(&self, nonce_variant: &NonceVariant) -> NonceVariantRecord {
        match nonce_variant {
            NonceVariant::NeutralNonce => NonceVariantRecord::NeutralNonce,
            NonceVariant::Nonce => NonceVariantRecord::Nonce,
        }
    }

    pub fn to_ex_units_record(&self, ex_units: &Option<ExUnits>) -> Option<ExUnitsRecord> {
        match ex_units {
            Some(ex_units) => Some(ExUnitsRecord {
                mem: ex_units.mem,
                steps: ex_units.steps,
            }),
            None => None,
        }
    }

    pub fn to_certificate_event(&self, certificate: &Certificate) -> EventData {
        let certificate_record = self.to_certificate_record(certificate);
        match certificate_record {
            CertificateRecord::StakeRegistration(cert_record) => {
                EventData::StakeRegistration(cert_record)
            }
            CertificateRecord::StakeDeregistration(cert_record) => {
                EventData::StakeDeregistration(cert_record)
            }
            CertificateRecord::StakeDelegation(cert_record) => {
                EventData::StakeDelegation(cert_record)
            }
            CertificateRecord::PoolRegistration(cert_record) => {
                EventData::PoolRegistration(cert_record)
            }
            CertificateRecord::PoolRetirement(cert_record) => {
                EventData::PoolRetirement(cert_record)
            }
            CertificateRecord::MoveInstantaneousRewardsCert(cert_record) => {
                EventData::MoveInstantaneousRewardsCert(cert_record)
            }
            CertificateRecord::GenesisKeyDelegation(cert_record) => {
                EventData::GenesisKeyDelegation(cert_record)
            }
            CertificateRecord::RegCert(cert_record) => EventData::RegCert(cert_record),
            CertificateRecord::UnRegCert(cert_record) => EventData::UnRegCert(cert_record),
            CertificateRecord::VoteDeleg(cert_record) => EventData::VoteDeleg(cert_record),
            CertificateRecord::StakeVoteDeleg(cert_record) => {
                EventData::StakeVoteDeleg(cert_record)
            }
            CertificateRecord::StakeRegDeleg(cert_record) => EventData::StakeRegDeleg(cert_record),
            CertificateRecord::VoteRegDeleg(cert_record) => EventData::VoteRegDeleg(cert_record),
            CertificateRecord::StakeVoteRegDeleg(cert_record) => {
                EventData::StakeVoteRegDeleg(cert_record)
            }
            CertificateRecord::AuthCommitteeHot(cert_record) => {
                EventData::AuthCommitteeHot(cert_record)
            }
            CertificateRecord::ResignCommitteeCold(cert_record) => {
                EventData::ResignCommitteeCold(cert_record)
            }
            CertificateRecord::RegDRepCert(cert_record) => EventData::RegDRepCert(cert_record),
            CertificateRecord::UnRegDRepCert(cert_record) => EventData::UnRegDRepCert(cert_record),
            CertificateRecord::UpdateDRepCert(cert_record) => {
                EventData::UpdateDRepCert(cert_record)
            }
        }
    }

    pub fn to_collateral_event(&self, collateral: &TransactionInput) -> EventData {
        EventData::Collateral {
            tx_id: collateral.transaction_id.to_hex(),
            index: collateral.index,
        }
    }

    pub fn to_tx_size(
        &self,
        body: &KeepRaw<TransactionBody>,
        aux_data: Option<&KeepRaw<AuxiliaryData>>,
        witness_set: Option<&KeepRaw<MintedWitnessSet>>,
    ) -> usize {
        body.raw_cbor().len()
            + aux_data.map(|ax| ax.raw_cbor().len()).unwrap_or(2)
            + witness_set.map(|ws| ws.raw_cbor().len()).unwrap_or(1)
    }

    pub fn to_transaction_record(
        &self,
        body: &KeepRaw<TransactionBody>,
        tx_hash: &str,
        aux_data: Option<&KeepRaw<AuxiliaryData>>,
        witness_set: Option<&KeepRaw<MintedWitnessSet>>,
    ) -> Result<TransactionRecord, Error> {
        let mut record = TransactionRecord {
            hash: tx_hash.to_owned(),
            size: self.to_tx_size(body, aux_data, witness_set) as u32,
            fee: body.fee,
            ttl: body.ttl,
            validity_interval_start: body.validity_interval_start,
            network_id: body.network_id.as_ref().map(|x| match x {
                NetworkId::One => 1,
                NetworkId::Two => 2,
            }),
            ..TransactionRecord::default()
        };

        let outputs = self.collect_legacy_output_records(&body.outputs)?;
        record.output_count = outputs.len();
        record.total_output = outputs.iter().map(|o| o.amount).sum();

        let inputs = self.collect_input_records(&body.inputs);
        record.input_count = inputs.len();

        if let Some(mint) = &body.mint {
            let mints = self.collect_mint_records(mint);
            record.mint_count = mints.len();

            if self.config.include_transaction_details {
                record.mint = mints.into();
            }
        }

        if let Some(certs) = &body.certificates {
            let certs = self.collect_certificate_records(certs);
            record.certificate_count = certs.len();

            if self.config.include_transaction_details {
                record.certs = certs.into();
            }
        }

        if let Some(update) = &body.update {
            if self.config.include_transaction_details {
                record.update = Some(self.to_update_record(update));
            }
        }

        if let Some(req_signers) = &body.required_signers {
            let req_signers = self.collect_required_signers_records(req_signers)?;
            record.required_signers_count = req_signers.len();

            if self.config.include_transaction_details {
                record.required_signers = Some(req_signers);
            }
        }

        // TODO
        // TransactionBodyComponent::ScriptDataHash(_)
        // TransactionBodyComponent::AuxiliaryDataHash(_)

        if self.config.include_transaction_details {
            record.outputs = outputs.into();
            record.inputs = inputs.into();

            record.metadata = match aux_data {
                Some(aux_data) => self.collect_metadata_records(aux_data)?.into(),
                None => None,
            };

            if let Some(witnesses) = witness_set {
                record.vkey_witnesses = self
                    .collect_vkey_witness_records(&witnesses.vkeywitness)?
                    .into();

                record.native_witnesses = self
                    .collect_native_witness_records(&witnesses.native_script)?
                    .into();

                record.plutus_witnesses = self
                    .collect_plutus_v1_witness_records(&witnesses.plutus_script)?
                    .into();

                record.plutus_redeemers = self
                    .collect_plutus_redeemer_records(&witnesses.redeemer)?
                    .into();

                record.plutus_data = self
                    .collect_witness_plutus_datum_records(&witnesses.plutus_data)?
                    .into();
            }

            if let Some(withdrawals) = &body.withdrawals {
                record.withdrawals = self.collect_withdrawal_records(withdrawals).into();
            }
        }

        Ok(record)
    }

    pub fn to_block_record(
        &self,
        source: &MintedBlock,
        hash: &Hash<32>,
        cbor: &[u8],
        era: Era,
    ) -> Result<BlockRecord, Error> {
        let relative_epoch = self
            .utils
            .time
            .as_ref()
            .map(|time| time.absolute_slot_to_relative(source.header.header_body.slot));

        let mut record = BlockRecord {
            era,
            body_size: source.header.header_body.block_body_size as usize,
            issuer_vkey: source.header.header_body.issuer_vkey.to_hex(),
            vrf_vkey: source.header.header_body.vrf_vkey.to_hex(),
            tx_count: source.transaction_bodies.len(),
            hash: hex::encode(hash),
            number: source.header.header_body.block_number,
            slot: source.header.header_body.slot,
            epoch: relative_epoch.map(|(epoch, _)| epoch),
            epoch_slot: relative_epoch.map(|(_, epoch_slot)| epoch_slot),
            previous_hash: source
                .header
                .header_body
                .prev_hash
                .map(hex::encode)
                .unwrap_or_default(),
            cbor_hex: match self.config.include_block_cbor {
                true => hex::encode(cbor).into(),
                false => None,
            },
            transactions: None,
        };

        if self.config.include_block_details {
            record.transactions = Some(self.collect_shelley_tx_records(source)?);
        }

        Ok(record)
    }

    pub fn to_protocol_update_record(
        &self,
        update: &ProtocolParamUpdate,
    ) -> ProtocolParamUpdateRecord {
        ProtocolParamUpdateRecord {
            minfee_a: update.minfee_a,
            minfee_b: update.minfee_b,
            max_block_body_size: update.max_block_body_size,
            max_transaction_size: update.max_transaction_size,
            max_block_header_size: update.max_block_header_size,
            key_deposit: update.key_deposit,
            pool_deposit: update.pool_deposit,
            maximum_epoch: update.maximum_epoch,
            desired_number_of_stake_pools: update.desired_number_of_stake_pools,
            pool_pledge_influence: self
                .to_rational_number_record_option(&update.pool_pledge_influence),
            expansion_rate: self.to_unit_interval_record(&update.expansion_rate),
            treasury_growth_rate: self.to_unit_interval_record(&update.treasury_growth_rate),
            decentralization_constant: self
                .to_unit_interval_record(&update.decentralization_constant),
            extra_entropy: self.to_nonce_record(&update.extra_entropy),
            protocol_version: update.protocol_version,
            min_pool_cost: update.min_pool_cost,
            ada_per_utxo_byte: update.ada_per_utxo_byte,
            cost_models_for_script_languages: self
                .to_cost_models_record(&update.cost_models_for_script_languages),
            execution_costs: match &update.execution_costs {
                Some(execution_costs) => Some(json!(execution_costs)),
                None => None,
            },
            max_tx_ex_units: self.to_ex_units_record(&update.max_tx_ex_units),
            max_block_ex_units: self.to_ex_units_record(&update.max_block_ex_units),
            max_value_size: update.max_value_size,
            collateral_percentage: update.collateral_percentage,
            max_collateral_inputs: update.max_collateral_inputs,
        }
    }

    pub fn to_update_record(&self, update: &Update) -> UpdateRecord {
        let mut updates = HashMap::new();
        for update in update.proposed_protocol_parameter_updates.clone().to_vec() {
            updates.insert(update.0.to_hex(), self.to_protocol_update_record(&update.1));
        }

        UpdateRecord {
            proposed_protocol_parameter_updates: updates,
            epoch: update.epoch,
        }
    }

    pub(crate) fn append_rollback_event(&self, point: &Point) -> Result<(), Error> {
        let data = match point {
            Point::Origin => EventData::RollBack {
                block_slot: 0,
                block_hash: "".to_string(),
            },
            Point::Specific(slot, hash) => EventData::RollBack {
                block_slot: *slot,
                block_hash: hex::encode(hash),
            },
        };

        self.append(data)
    }
}
