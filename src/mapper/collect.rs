use std::option::IntoIter;
use pallas::ledger::primitives::alonzo::{AddrKeyhash, AddrKeyhashes, Certificates, RequiredSigners, TransactionInputs};
use pallas::{
    codec::utils::{KeepRaw, KeyValuePairs, MaybeIndefArray},
    ledger::{
        primitives::{
            alonzo::{
                AuxiliaryData, Coin, MintedBlock, Multiasset, NativeScript, PlutusData,
                PlutusScript, Redeemer, RewardAccount, TransactionInput, VKeyWitness, Value,
            },
            babbage::{
                LegacyTransactionOutput, MintedPostAlonzoTransactionOutput,
                MintedTransactionOutput, PlutusV2Script,
            },
        },
        traverse::OriginalHash,
    },
};
use pallas::ledger::primitives::babbage::{KeepRawPlutusDatas, NativeScripts, PlutusV1Scripts, Redeemers, VKeyWitnesses};

use crate::model::{CertificateRecord, RequiredSignerRecord};
use crate::{
    model::{
        MetadataRecord, MintRecord, NativeWitnessRecord, OutputAssetRecord, PlutusDatumRecord,
        PlutusRedeemerRecord, PlutusWitnessRecord, TransactionRecord, TxInputRecord,
        TxOutputRecord, VKeyWitnessRecord, WithdrawalRecord,
    },
    Error,
};

use super::{map::ToHex, EventWriter};

impl EventWriter {
    pub fn collect_input_records(&self, source: &TransactionInputs) -> Vec<TxInputRecord> {
        source
            .iter()
            .map(|i| self.to_transaction_input_record(i))
            .collect()
    }

    pub fn collect_legacy_output_records(
        &self,
        source: &[LegacyTransactionOutput],
    ) -> Result<Vec<TxOutputRecord>, Error> {
        source
            .iter()
            .map(|i| self.to_legacy_output_record(i))
            .collect()
    }

    pub fn collect_post_alonzo_output_records(
        &self,
        source: &[MintedPostAlonzoTransactionOutput],
    ) -> Result<Vec<TxOutputRecord>, Error> {
        source
            .iter()
            .map(|i| self.to_post_alonzo_output_record(i))
            .collect()
    }

    pub fn collect_any_output_records(
        &self,
        source: &[MintedTransactionOutput],
    ) -> Result<Vec<TxOutputRecord>, Error> {
        source
            .iter()
            .map(|x| match x {
                MintedTransactionOutput::Legacy(x) => self.to_legacy_output_record(x),
                MintedTransactionOutput::PostAlonzo(x) => self.to_post_alonzo_output_record(x),
            })
            .collect()
    }

    pub fn collect_asset_records(&self, amount: &Value) -> Vec<OutputAssetRecord> {
        match amount {
            Value::Coin(_) => vec![],
            Value::Multiasset(_, policies) => policies
                .iter()
                .flat_map(|(policy, assets)| {
                    assets.iter().map(|(asset, amount)| {
                        self.to_transaction_output_asset_record(policy, asset, *amount)
                    })
                })
                .collect(),
        }
    }

    pub fn collect_mint_records(&self, mint: &Multiasset<i64>) -> Vec<MintRecord> {
        mint.iter()
            .flat_map(|(policy, assets)| {
                assets
                    .iter()
                    .map(|(asset, amount)| self.to_mint_record(policy, asset, *amount))
            })
            .collect()
    }

    pub fn collect_certificate_records(
        &self,
        certificates: &Certificates,
    ) -> Vec<CertificateRecord> {
        certificates
            .iter()
            .map(|cert| self.to_certificate_record(cert))
            .collect()
    }

    pub fn collect_withdrawal_records(
        &self,
        withdrawls: &KeyValuePairs<RewardAccount, Coin>,
    ) -> Vec<WithdrawalRecord> {
        withdrawls
            .iter()
            .map(|(reward_account, coin)| WithdrawalRecord {
                reward_account: {
                    let hex = reward_account.to_hex();
                    hex.strip_prefix("e1").map(|x| x.to_string()).unwrap_or(hex)
                },
                coin: *coin,
            })
            .collect()
    }

    pub fn collect_metadata_records(
        &self,
        aux_data: &AuxiliaryData,
    ) -> Result<Vec<MetadataRecord>, Error> {
        let metadata = match aux_data {
            AuxiliaryData::PostAlonzo(data) => data.metadata.as_deref(),
            AuxiliaryData::Shelley(data) => Some(data.as_ref()),
            AuxiliaryData::ShelleyMa(data) => Some(data.transaction_metadata.as_ref()),
        };

        match metadata {
            Some(x) => x
                .iter()
                .map(|(label, content)| self.to_metadata_record(label, content))
                .collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_vkey_witness_records(
        &self,
        witness_set: &Option<Vec<VKeyWitness>>,
    ) -> Result<Vec<VKeyWitnessRecord>, Error> {
        match witness_set {
            Some(all) => all.iter().map(|i| self.to_vkey_witness_record(i)).collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_vkey_witness_records_babbage(
        &self,
        witness_set: &Option<VKeyWitnesses>,
    ) -> Result<Vec<VKeyWitnessRecord>, Error> {
        match witness_set {
            Some(all) => all.iter().map(|i| self.to_vkey_witness_record(i)).collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_native_witness_records(
        &self,
        witness_set: &Option<Vec<NativeScript>>,
    ) -> Result<Vec<NativeWitnessRecord>, Error> {
        match witness_set {
            Some(all) => all
                .iter()
                .map(|i| self.to_native_witness_record(i))
                .collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_native_witness_records_babbage(
        &self,
        witness_set: &Option<NativeScripts>,
    ) -> Result<Vec<NativeWitnessRecord>, Error> {
        match witness_set {
            Some(all) => all
                .iter()
                .map(|i| self.to_native_witness_record(i))
                .collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_plutus_v1_witness_records(
        &self,
        witness_set: &Option<Vec<PlutusScript>>,
    ) -> Result<Vec<PlutusWitnessRecord>, Error> {
        match &witness_set {
            Some(all) => all
                .iter()
                .map(|i| self.to_plutus_v1_witness_record(i))
                .collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_plutus_v1_witness_records_babbage(
        &self,
        witness_set: &Option<PlutusV1Scripts>,
    ) -> Result<Vec<PlutusWitnessRecord>, Error> {
        match &witness_set {
            Some(all) => all
                .iter()
                .map(|i| self.to_plutus_v1_witness_record(i))
                .collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_plutus_v2_witness_records(
        &self,
        witness_set: &Option<MaybeIndefArray<PlutusV2Script>>,
    ) -> Result<Vec<PlutusWitnessRecord>, Error> {
        match &witness_set {
            Some(all) => all
                .iter()
                .map(|i| self.to_plutus_v2_witness_record(i))
                .collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_plutus_redeemer_records(
        &self,
        witness_set: &Option<Vec<Redeemer>>,
    ) -> Result<Vec<PlutusRedeemerRecord>, Error> {
        match &witness_set {
            Some(all) => all
                .iter()
                .map(|i| self.to_plutus_redeemer_record(i))
                .collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_plutus_redeemer_records_2(
        &self,
        witness_set: &Option<Redeemers>,
    ) -> Result<Vec<PlutusRedeemerRecord>, Error> {
        match &witness_set {
            Some(all) => all
                .iter()
                .map(|i| self.to_plutus_redeemer_record(i))
                .collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_witness_plutus_datum_records(
        &self,
        witness_set: &Option<Vec<KeepRaw<PlutusData>>>,
    ) -> Result<Vec<PlutusDatumRecord>, Error> {
        match &witness_set {
            Some(all) => all.iter().map(|i| self.to_plutus_datum_record(i)).collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_witness_plutus_datum_records_babbage(
        &self,
        witness_set: &Option<KeepRawPlutusDatas>,
    ) -> Result<Vec<PlutusDatumRecord>, Error> {
        match &witness_set {
            Some(all) => all.into_iter().map(|i| self.to_plutus_datum_record(i)).collect(),
            None => Ok(vec![]),
        }
    }

    pub fn collect_shelley_tx_records(
        &self,
        block: &MintedBlock,
    ) -> Result<Vec<TransactionRecord>, Error> {
        block
            .transaction_bodies
            .iter()
            .enumerate()
            .map(|(idx, tx)| {
                let aux_data = block
                    .auxiliary_data_set
                    .iter()
                    .find(|(k, _)| *k == (idx as u32))
                    .map(|(_, v)| v);

                let witness_set = block.transaction_witness_sets.get(idx);

                let tx_hash = tx.original_hash().to_hex();

                self.to_transaction_record(tx, &tx_hash, aux_data, witness_set)
            })
            .collect()
    }

    pub fn collect_required_signers_records(
        &self,
        req_signers: &Vec<AddrKeyhash>,
    ) -> Result<Vec<RequiredSignerRecord>, Error> {
        let mut signers = vec![];
        for req_sign in req_signers {
            let hex = req_sign.to_hex();
            signers.push(RequiredSignerRecord(hex));
        }

        Ok(signers)
    }
}
