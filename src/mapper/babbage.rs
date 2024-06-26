use pallas::codec::utils::KeepRaw;
use std::collections::HashMap;

use pallas::ledger::primitives::babbage::{
    AuxiliaryData, CostMdls, Language, MintedBlock, MintedDatumOption,
    MintedPostAlonzoTransactionOutput, MintedTransactionBody, MintedTransactionOutput,
    MintedWitnessSet, NetworkId, ProtocolParamUpdate, Update,
};

use pallas::crypto::hash::Hash;
use pallas::ledger::traverse::OriginalHash;
use serde_json::json;

use crate::model::{
    BlockRecord, CostModelRecord, CostModelsRecord, Era, LanguageVersionRecord,
    ProtocolParamUpdateRecord, TransactionRecord, UpdateRecord,
};
use crate::utils::time::TimeProvider;
use crate::{
    model::{EventContext, EventData},
    Error,
};

use super::{map::ToHex, EventWriter};

impl EventWriter {
    pub fn to_babbage_tx_size(
        &self,
        body: &KeepRaw<MintedTransactionBody>,
        aux_data: Option<&KeepRaw<AuxiliaryData>>,
        witness_set: Option<&KeepRaw<MintedWitnessSet>>,
    ) -> usize {
        body.raw_cbor().len()
            + aux_data.map(|ax| ax.raw_cbor().len()).unwrap_or(2)
            + witness_set.map(|ws| ws.raw_cbor().len()).unwrap_or(1)
    }

    pub fn to_babbage_transaction_record(
        &self,
        body: &KeepRaw<MintedTransactionBody>,
        tx_hash: &str,
        aux_data: Option<&KeepRaw<AuxiliaryData>>,
        witness_set: Option<&KeepRaw<MintedWitnessSet>>,
    ) -> Result<TransactionRecord, Error> {
        let mut record = TransactionRecord {
            hash: tx_hash.to_owned(),
            size: self.to_babbage_tx_size(body, aux_data, witness_set) as u32,
            fee: body.fee,
            ttl: body.ttl,
            validity_interval_start: body.validity_interval_start,
            network_id: body.network_id.as_ref().map(|x| match x {
                NetworkId::One => 1,
                NetworkId::Two => 2,
            }),
            ..Default::default()
        };

        let outputs = self.collect_any_output_records(&body.outputs)?;
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

        // Add Collateral Stuff
        let collateral_inputs = &body.collateral;
        record.collateral_input_count = collateral_inputs.iter().count();
        record.has_collateral_output = body.collateral_return.is_some();

        if let Some(update) = &body.update {
            if self.config.include_transaction_details {
                record.update = Some(self.to_babbage_update_record(update));
            }
        }

        if let Some(req_signers) = &body.required_signers {
            let req_signers = self.collect_required_signers_records(req_signers.into())?;
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

            // transaction_details collateral stuff
            record.collateral_inputs =
                collateral_inputs.as_ref().map(|inputs| self.collect_input_records(inputs));

            record.collateral_output = body.collateral_return.as_ref().map(|output| match output {
                MintedTransactionOutput::Legacy(x) => self.to_legacy_output_record(x).unwrap(),
                MintedTransactionOutput::PostAlonzo(x) => {
                    self.to_post_alonzo_output_record(x).unwrap()
                }
            });

            record.metadata = match aux_data {
                Some(aux_data) => self.collect_metadata_records(aux_data)?.into(),
                None => None,
            };

            if let Some(witnesses) = witness_set {
                record.vkey_witnesses = self
                    .collect_vkey_witness_records_babbage(&witnesses.vkeywitness)?
                    .into();

                record.native_witnesses = self
                    .collect_native_witness_records_babbage(&witnesses.native_script)?
                    .into();

                record.plutus_witnesses = self
                    .collect_plutus_v1_witness_records_babbage(&witnesses.plutus_v1_script)?
                    .into();

                record.plutus_redeemers = self
                    .collect_plutus_redeemer_records_2(&witnesses.redeemer)?
                    .into();

                record.plutus_data = self
                    .collect_witness_plutus_datum_records_babbage(&witnesses.plutus_data)?
                    .into();
            }

            if let Some(withdrawals) = &body.withdrawals {
                record.withdrawals = self.collect_withdrawal_records(withdrawals).into();
            }
        }

        Ok(record)
    }

    pub fn to_babbage_block_record(
        &self,
        source: &MintedBlock,
        hash: &Hash<32>,
        cbor: &[u8],
    ) -> Result<BlockRecord, Error> {
        let relative_epoch = self
            .utils
            .time
            .as_ref()
            .map(|time| time.absolute_slot_to_relative(source.header.header_body.slot));

        let mut record = BlockRecord {
            era: Era::Babbage,
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
            record.transactions = Some(self.collect_babbage_tx_records(source)?);
        }

        Ok(record)
    }

    pub fn collect_babbage_tx_records(
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

                self.to_babbage_transaction_record(tx, &tx_hash, aux_data, witness_set)
            })
            .collect()
    }

    fn crawl_post_alonzo_output(
        &self,
        output: &MintedPostAlonzoTransactionOutput,
    ) -> Result<(), Error> {
        let record = self.to_post_alonzo_output_record(output)?;
        self.append(record.into())?;

        let address = pallas::ledger::addresses::Address::from_bytes(&output.address)?;

        let child = &self.child_writer(EventContext {
            output_address: address.to_string().into(),
            ..EventContext::default()
        });

        child.crawl_transaction_output_amount(&output.value)?;

        if let Some(MintedDatumOption::Data(datum)) = &output.datum_option {
            let record = self.to_plutus_datum_record(datum)?;
            child.append(record.into())?;
        }

        Ok(())
    }

    fn crawl_babbage_transaction_output(
        &self,
        output: &MintedTransactionOutput,
    ) -> Result<(), Error> {
        match output {
            MintedTransactionOutput::Legacy(x) => self.crawl_legacy_output(x),
            MintedTransactionOutput::PostAlonzo(x) => self.crawl_post_alonzo_output(x),
        }
    }

    fn crawl_babbage_witness_set(
        &self,
        witness_set: &KeepRaw<MintedWitnessSet>,
    ) -> Result<(), Error> {
        if let Some(native) = &witness_set.native_script {
            for script in native.iter() {
                self.append_from(self.to_native_witness_record(script)?)?;
            }
        }

        if let Some(plutus) = &witness_set.plutus_v1_script {
            for script in plutus.iter() {
                self.append_from(self.to_plutus_v1_witness_record(script)?)?;
            }
        }

        if let Some(redeemers) = &witness_set.redeemer {
            for redeemer in redeemers.iter() {
                self.append_from(self.to_plutus_redeemer_record(redeemer)?)?;
            }
        }

        if let Some(datums) = &witness_set.plutus_data {
            for datum in datums {
                self.append_from(self.to_plutus_datum_record(datum)?)?;
            }
        }

        Ok(())
    }

    fn crawl_babbage_transaction(
        &self,
        tx: &KeepRaw<MintedTransactionBody>,
        tx_hash: &str,
        aux_data: Option<&KeepRaw<AuxiliaryData>>,
        witness_set: Option<&KeepRaw<MintedWitnessSet>>,
    ) -> Result<(), Error> {
        let record = self.to_babbage_transaction_record(tx, tx_hash, aux_data, witness_set)?;

        self.append_from(record.clone())?;

        for (idx, input) in tx.inputs.iter().enumerate() {
            let child = self.child_writer(EventContext {
                input_idx: Some(idx),
                ..EventContext::default()
            });

            child.crawl_transaction_input(input)?;
        }

        for (idx, output) in tx.outputs.iter().enumerate() {
            let child = self.child_writer(EventContext {
                output_idx: Some(idx),
                ..EventContext::default()
            });

            child.crawl_babbage_transaction_output(output)?;
        }

        if let Some(certs) = &tx.certificates {
            for (idx, cert) in certs.iter().enumerate() {
                let child = self.child_writer(EventContext {
                    certificate_idx: Some(idx),
                    ..EventContext::default()
                });

                child.crawl_certificate(cert)?;
            }
        }

        if let Some(collateral) = &tx.collateral {
            for (_idx, collateral) in collateral.iter().enumerate() {
                // TODO: collateral context?

                self.crawl_collateral(collateral)?;
            }
        }

        if let Some(mint) = &tx.mint {
            self.crawl_mints(mint)?;
        }

        if let Some(aux_data) = aux_data {
            self.crawl_auxdata(aux_data)?;
        }

        if let Some(witness_set) = witness_set {
            self.crawl_babbage_witness_set(witness_set)?;
        }

        if self.config.include_transaction_end_events {
            self.append(EventData::TransactionEnd(record))?;
        }

        Ok(())
    }

    fn crawl_babbage_block(
        &self,
        block: &MintedBlock,
        hash: &Hash<32>,
        cbor: &[u8],
    ) -> Result<(), Error> {
        let record = self.to_babbage_block_record(block, hash, cbor)?;

        self.append(EventData::Block(record.clone()))?;

        for (idx, tx) in block.transaction_bodies.iter().enumerate() {
            let aux_data = block
                .auxiliary_data_set
                .iter()
                .find(|(k, _)| *k == (idx as u32))
                .map(|(_, v)| v);

            let witness_set = block.transaction_witness_sets.get(idx);

            let tx_hash = tx.original_hash().to_hex();

            let child = self.child_writer(EventContext {
                tx_idx: Some(idx),
                tx_hash: Some(tx_hash.to_owned()),
                ..EventContext::default()
            });

            child.crawl_babbage_transaction(tx, &tx_hash, aux_data, witness_set)?;
        }

        if self.config.include_block_end_events {
            self.append(EventData::BlockEnd(record))?;
        }

        Ok(())
    }

    pub fn to_babbage_cost_models_record(
        &self,
        cost_models: &Option<CostMdls>,
    ) -> Option<CostModelsRecord> {
        match cost_models {
            Some(cost_models) => {
                let mut cost_models_record = HashMap::new();
                if let Some(cost_model_v1) = &cost_models.plutus_v1 {
                    let language_version_record = LanguageVersionRecord::PlutusV1;
                    let cost_model_record = CostModelRecord(cost_model_v1.clone());
                    cost_models_record.insert(language_version_record, cost_model_record);
                }
                if let Some(cost_model_v2) = &cost_models.plutus_v2 {
                    let language_version_record = LanguageVersionRecord::PlutusV2;
                    let cost_model_record = CostModelRecord(cost_model_v2.clone());
                    cost_models_record.insert(language_version_record, cost_model_record);
                }

                Some(CostModelsRecord(cost_models_record))
            }
            None => None,
        }
    }

    pub fn to_babbage_language_version_record(
        &self,
        language_version: &Language,
    ) -> LanguageVersionRecord {
        match language_version {
            Language::PlutusV1 => LanguageVersionRecord::PlutusV1,
            Language::PlutusV2 => LanguageVersionRecord::PlutusV2,
        }
    }

    pub fn to_babbage_protocol_update_record(
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
            decentralization_constant: None,
            extra_entropy: None,
            protocol_version: update.protocol_version,
            min_pool_cost: update.min_pool_cost,
            ada_per_utxo_byte: update.ada_per_utxo_byte,
            cost_models_for_script_languages: self
                .to_babbage_cost_models_record(&update.cost_models_for_script_languages),
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

    pub fn to_babbage_update_record(&self, update: &Update) -> UpdateRecord {
        let mut updates = HashMap::new();
        for update in update.proposed_protocol_parameter_updates.clone().to_vec() {
            updates.insert(
                update.0.to_hex(),
                self.to_babbage_protocol_update_record(&update.1),
            );
        }

        UpdateRecord {
            proposed_protocol_parameter_updates: updates,
            epoch: update.epoch,
        }
    }

    /// Mapper entry-point for decoded Babbage blocks
    ///
    /// Entry-point to start crawling a blocks for events. Meant to be used when
    /// we already have a decoded block (for example, N2C). The raw CBOR is also
    /// passed through in case we need to attach it to outbound events.
    pub fn crawl_babbage_with_cbor<'b>(
        &self,
        block: &'b MintedBlock<'b>,
        cbor: &'b [u8],
    ) -> Result<(), Error> {
        let hash = block.header.original_hash();

        let child = self.child_writer(EventContext {
            block_hash: Some(hex::encode(hash)),
            block_number: Some(block.header.header_body.block_number),
            slot: Some(block.header.header_body.slot),
            timestamp: self.compute_timestamp(block.header.header_body.slot),
            ..EventContext::default()
        });

        child.crawl_babbage_block(block, &hash, cbor)
    }

    /// Mapper entry-point for raw Babbage cbor blocks
    ///
    /// Entry-point to start crawling a blocks for events. Meant to be used when
    /// we haven't decoded the CBOR yet (for example, N2N).
    pub fn crawl_from_babbage_cbor(&self, cbor: &[u8]) -> Result<(), Error> {
        let (_, block): (u16, MintedBlock) = pallas::codec::minicbor::decode(cbor)?;
        self.crawl_babbage_with_cbor(&block, cbor)
    }
}
