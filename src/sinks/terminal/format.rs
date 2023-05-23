use std::fmt::{Display, Write};

use crossterm::style::{Attribute, Color, Stylize};
use unicode_truncate::UnicodeTruncateStr;

use crate::{
    model::{
        BlockRecord, CIP15AssetRecord, CIP25AssetRecord, Event, EventData, MetadataRecord,
        MintRecord, NativeWitnessRecord, OutputAssetRecord, PlutusDatumRecord,
        PlutusRedeemerRecord, PlutusWitnessRecord, TransactionRecord, TxInputRecord,
        TxOutputRecord, VKeyWitnessRecord,
    },
    utils::Utils,
};

pub struct LogLine {
    prefix: &'static str,
    color: Color,
    tx_idx: Option<usize>,
    block_num: Option<u64>,
    content: String,
    max_width: Option<usize>,
}

impl LogLine {
    fn new_raw(
        source: &Event,
        prefix: &'static str,
        color: Color,
        max_width: Option<usize>,
        content: String,
    ) -> Self {
        LogLine {
            prefix,
            color,
            content,
            max_width,
            tx_idx: source.context.tx_idx,
            block_num: source.context.block_number,
        }
    }
}

impl LogLine {
    pub fn new(source: &Event, max_width: Option<usize>, utils: &Utils) -> LogLine {
        match &source.data {
            EventData::Block(BlockRecord {
                era,
                body_size,
                issuer_vkey,
                tx_count,
                slot,
                hash,
                number,
                ..
            }) => LogLine::new_raw(
                    source,
                    "BLOCK",
                    Color::Magenta,
                    max_width,
                    format!(
                        "{{ era: {:?}, slot: {}, hash: {}, number: {}, body size: {}, tx_count: {}, issuer vkey: {}, timestamp: {} }}",
                        era,
                        slot,
                        hash,
                        number,
                        body_size,
                        tx_count,
                        issuer_vkey,
                        source.context.timestamp.unwrap_or_default(),
                    ),
                ),
            EventData::BlockEnd(BlockRecord {
                slot,
                hash,
                number,
                ..
            }) => LogLine::new_raw(
                source,
                "ENDBLK",
                Color::DarkMagenta,
                max_width,
                format!("{{ slot: {slot}, hash: {hash}, number: {number} }}")),

            EventData::Transaction(TransactionRecord {
                total_output,
                fee,
                ttl,
                hash,
                ..
            }) => LogLine::new_raw(
                source,
                "TX",
                Color::DarkBlue,
                max_width,
                format!("{{ total_output: {total_output}, fee: {fee}, hash: {hash}, ttl: {ttl:?} }}"),
            ),
            EventData::TransactionEnd(TransactionRecord { hash, .. }) => LogLine::new_raw(
                source,
                "ENDTX",
                Color::DarkBlue,
                max_width,
                format!("{{ hash: {hash} }}"),
            ),
            EventData::TxInput(TxInputRecord { tx_id, index }) => LogLine::new_raw(
                source,
                "STXI",
                Color::Blue,
                max_width,
                format!("{{ tx_id: {tx_id}, index: {index} }}"),
            ),
            EventData::TxOutput(TxOutputRecord {
                address, amount, ..
            }) => LogLine::new_raw(
                source,
                "UTXO",
                Color::Blue,
                max_width,
                format!("{{ to: {address}, amount: {amount} }}"),
            ),
            EventData::OutputAsset(OutputAssetRecord {
                policy,
                asset,
                asset_ascii,
                ..
            }) if policy == &utils.well_known.adahandle_policy => LogLine::new_raw(
                source,
                "$HNDL",
                Color::DarkGreen,
                max_width,
                format!(
                    "{{ {} => {} }}",
                    asset_ascii.as_deref().unwrap_or(asset),
                    source.context.output_address.as_deref().unwrap_or_default(),
                ),
            ),
            EventData::OutputAsset(OutputAssetRecord {
                policy,
                asset,
                asset_ascii,
                amount,
                ..
            }) => LogLine::new_raw(
                source,
                "ASSET",
                Color::Green,
                max_width,
                format!(
                    "{{ policy: {}, asset: {}, amount: {} }}",
                    policy, asset_ascii.as_deref().unwrap_or(asset), amount
                ),
            ),
            EventData::Metadata(MetadataRecord { label, content }) => LogLine::new_raw(
                source,
                "META",
                Color::Yellow,
                max_width,
                format!("{{ label: {label}, content: {content} }}"),
            ),
            EventData::Mint(MintRecord {
                policy,
                asset,
                quantity,
            }) => LogLine::new_raw(
                source,
                "MINT",
                Color::DarkGreen,
                max_width,
                format!(
                    "{{ policy: {policy}, asset: {asset}, quantity: {quantity} }}"),
            ),
            EventData::NativeScript { policy_id, script } => LogLine::new_raw(
                source,
                "NATIVE",
                Color::White,
                max_width,
                format!("{{ policy: {policy_id}, script: {script} }}"),
            ),
            EventData::PlutusScript { hash, .. } => LogLine::new_raw(
                source,
                "PLUTUS",
                Color::White,
                max_width,
                format!("{{ hash: {hash} }}"),
            ),
            EventData::PlutusDatum(PlutusDatumRecord { datum_hash, .. }) => LogLine::new_raw(
                source,
                "DATUM",
                Color::White,
                max_width,
                format!("{{ hash: {datum_hash} }}"),
            ),
            EventData::PlutusRedeemer(PlutusRedeemerRecord { purpose, input_idx, .. }) => LogLine::new_raw(
                source,
                "REDEEM",
                Color::White,
                max_width,
                format!("{{ purpose: {purpose}, input: {input_idx} }}"),
            ),
            EventData::PlutusWitness(PlutusWitnessRecord { script_hash, .. }) => LogLine::new_raw(
                source,
                "WITNESS",
                Color::White,
                max_width,
                format!("{{ plutus script: {script_hash} }}"),
            ),
            EventData::NativeWitness(NativeWitnessRecord { policy_id, .. }) => LogLine::new_raw(
                source,
                "WITNESS",
                Color::White,
                max_width,
                format!("{{ native policy: {policy_id} }}"),
            ),
            EventData::VKeyWitness(VKeyWitnessRecord { vkey_hex, .. }) => LogLine::new_raw(
                source,
                "WITNESS",
                Color::White,
                max_width,
                format!("{{ vkey: {vkey_hex} }}"),
            ),
            EventData::StakeRegistration(cert) => LogLine::new_raw(
                source,
                "STAKE+",
                Color::Magenta,
                max_width,
                format!("{{ credential: {0:?} }}", cert.credential),
            ),
            EventData::StakeDeregistration(cert) => LogLine::new_raw(
                source,
                "STAKE-",
                Color::DarkMagenta,
                max_width,
                format!("{{ credential: {0:?} }}", cert.credential),
            ),
            EventData::StakeDelegation(cert) => LogLine::new_raw(
                source,
                "DELE",
                Color::Magenta,
                max_width,
                format!("{{ credential: {0:?}, pool: {1} }}", cert.credential, cert.pool_hash),
            ),
            EventData::PoolRegistration(cert) => LogLine::new_raw(
                source,
                "POOL+",
                Color::Magenta,
                max_width,
                format!(
                    "{{ operator: {0}, pledge: {1}, cost: {2}, margin: {{ numerator: {3}, denominator: {4} }}, metadata: {5:?} }}",
                    cert.operator, cert.pledge, cert.cost, cert.margin.numerator, cert.margin.denominator, cert.pool_metadata),
            ),
            EventData::PoolRetirement(cert) => LogLine::new_raw(
                source,
                "POOL-",
                Color::DarkMagenta,
                max_width,
                format!("{{ pool: {0}, epoch: {1} }}", cert.pool, cert.epoch),
            ),
            EventData::GenesisKeyDelegation(cert) => LogLine::new_raw(
                source,
                "GENESIS",
                Color::Magenta,
                max_width,
                format!("{{ genesis_hash: {0}, genesis_delegate_hash: {1}, vrf_key_hash: {2} }}",
                cert.genesis_hash, cert.genesis_delegate_hash, cert.vrf_key_hash),
            ),
            EventData::MoveInstantaneousRewardsCert(cert) => LogLine::new_raw(
                source,
                "MOVE",
                Color::Magenta,
                max_width,
                format!(
                    "{{ reserves: {0}, treasury: {1}, to_credentials: {2:?}, to_other_pot: {3:?} }}",
                cert.from_reserves, cert.from_treasury, cert.to_stake_credentials, cert.to_other_pot),
            ),
            EventData::RollBack {
                block_slot,
                block_hash,
            } => LogLine::new_raw(
                source,
                "RLLBCK",
                Color::Red,
                max_width,
                format!("{{ slot: {block_slot}, hash: {block_hash} }}"),
            ),
            EventData::Collateral { tx_id, index } => LogLine::new_raw(
                source,
                "COLLAT",
                Color::Blue,
                max_width,
                format!("{{ tx_id: {tx_id}, index: {index} }}"),
            ),
            EventData::CIP25Asset(CIP25AssetRecord {
                policy,
                asset,
                name,
                image,
                ..
            }) => LogLine::new_raw(
                source,
                "CIP25",
                Color::DarkYellow,
                max_width,
                format!(
                    "{{ policy: {}, asset: {}, name: {}, image: {} }}",
                    policy,
                    asset,
                    name.as_deref().unwrap_or("?"),
                    image.as_deref().unwrap_or("?")
                ),
            ),
            EventData::CIP15Asset(CIP15AssetRecord {
                voting_key,
                stake_pub,
                ..
            }) => LogLine::new_raw(
                source,
                "CIP15",
                Color::DarkYellow,
                max_width,
                format!("{{ voting key: {voting_key}, stake pub: {stake_pub} }}"),
            ),
        }
    }
}

impl Display for LogLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!(
            "BLOCK:{:0>7} █ TX:{:0>2}",
            self.block_num
                .map(|x| x.to_string())
                .unwrap_or_else(|| "-------".to_string()),
            self.tx_idx
                .map(|x| x.to_string())
                .unwrap_or_else(|| "--".to_string()),
        )
        .stylize()
        .with(Color::Grey)
        .attribute(Attribute::Dim)
        .fmt(f)?;

        f.write_char(' ')?;

        format!("█ {:6}", self.prefix)
            .stylize()
            .with(self.color)
            .fmt(f)?;

        f.write_char(' ')?;

        {
            let available_width = self.max_width.map(|x| x - 35);

            match available_width {
                Some(width) if width < self.content.len() => {
                    let (partial, _) = &self.content.unicode_truncate(width);
                    let partial = format!("{partial}...");
                    partial.with(Color::Grey).fmt(f)?;
                }
                _ => {
                    let full = &self.content[..];
                    full.with(Color::Grey).fmt(f)?;
                }
            };
        }

        f.write_str("\n")?;
        Ok(())
    }
}
