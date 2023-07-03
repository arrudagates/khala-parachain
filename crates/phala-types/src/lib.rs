#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod contract;

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use codec::{Decode, Encode};
use core::fmt::Debug;
use scale_info::TypeInfo;
use sp_core::H256;

#[cfg(feature = "enable_serde")]
use serde::{Deserialize, Serialize};

// Messages: Phase Wallet

pub mod messaging {
    use alloc::collections::btree_map::BTreeMap;
    use alloc::vec::Vec;
    use codec::{Decode, Encode};
    use core::fmt::Debug;
    use scale_info::TypeInfo;
    use sp_core::U256;

    #[cfg(feature = "enable_serde")]
    use serde::{Deserialize, Serialize};

    use super::{EcdhPublicKey, MasterPublicKey, WorkerIdentity, WorkerPublicKey};

    pub use phala_mq::bind_topic;
    pub use phala_mq::types::*;

    // TODO.kevin: reuse the Payload in secret_channel.rs.
    #[derive(Encode, Decode, Debug, TypeInfo)]
    pub enum CommandPayload<T> {
        Plain(T),
    }

    /// A fixed point number with 64 integer bits and 64 fractional bits.
    pub type U64F64Bits = u128;

    // Messages: System
    #[derive(Encode, Decode, TypeInfo)]
    pub struct WorkerEventWithKey {
        pub pubkey: WorkerPublicKey,
        pub event: WorkerEvent,
    }

    impl Debug for WorkerEventWithKey {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let pubkey = hex::encode(self.pubkey.0);
            f.debug_struct("WorkerEventWithKey")
                .field("pubkey", &pubkey)
                .field("event", &self.event)
                .finish()
        }
    }

    #[derive(Encode, Decode, Debug, TypeInfo)]
    pub struct WorkerInfo {
        pub confidence_level: u8,
    }

    #[derive(Encode, Decode, Debug, TypeInfo)]
    pub enum WorkerEvent {
        /// pallet-registry --> worker
        ///  Indicate a worker register succeeded.
        Registered(WorkerInfo),
        /// pallet-registry --> worker
        ///  When a worker register succeed, the chain request the worker to benchmark.
        ///   duration: Number of blocks the benchmark to keep on.
        BenchStart { duration: u32 },
        /// pallet-registry --> worker
        ///  The init bench score caculated by pallet.
        BenchScore(u32),
        /// pallet-computing --> worker
        ///  When a worker start to compute, push this message to the worker to start the benchmark task.
        ///   session_id: Generated by pallet. Each computing session should have a unique session_id.
        Started {
            session_id: u32,
            init_v: U64F64Bits,
            init_p: u32,
        },
        /// pallet-computing --> worker
        ///  When a worker entered CoolingDown state, push this message to the worker, so that it can stop the
        ///  benchmark task.
        Stopped,
        /// pallet-computing --> worker
        ///  When a worker entered Unresponsive state, push this message to the worker to suppress the subsequent
        ///  heartbeat responses.
        EnterUnresponsive,
        /// pallet-computing --> worker
        ///  When a worker recovered to WorkerIdle state from Unresponsive, push this message to the worker to
        ///  resume the subsequent heartbeat responses.
        ExitUnresponsive,
    }

    bind_topic!(SystemEvent, b"phala/system/event");
    #[derive(Encode, Decode, Debug, TypeInfo)]
    pub enum SystemEvent {
        WorkerEvent(WorkerEventWithKey),
        HeartbeatChallenge(HeartbeatChallenge),
    }

    impl SystemEvent {
        pub fn new_worker_event(pubkey: WorkerPublicKey, event: WorkerEvent) -> SystemEvent {
            SystemEvent::WorkerEvent(WorkerEventWithKey { pubkey, event })
        }
    }

    #[derive(Encode, Decode, Debug, Default, Clone, PartialEq, Eq, TypeInfo)]
    pub struct HeartbeatChallenge {
        pub seed: U256,
        pub online_target: U256,
    }

    bind_topic!(WorkingReportEvent, b"phala/mining/report");
    #[derive(Encode, Decode, Clone, Debug, TypeInfo)]
    pub enum WorkingReportEvent {
        Heartbeat {
            /// The computing session id.
            session_id: u32,
            /// The challenge block number.
            challenge_block: u32,
            /// The challenge block timestamp.
            challenge_time: u64,
            /// Benchmark iterations since working_start_time.
            iterations: u64,
        },
        HeartbeatV2 {
            /// The computing session id.
            session_id: u32,
            /// The challenge block number.
            challenge_block: u32,
            /// The challenge block timestamp.
            challenge_time: u64,
            /// Benchmark iterations since working_start_time.
            iterations: u64,
            /// Number of current deployed clusters.
            n_clusters: u32,
            /// Number of current deployed contracts.
            n_contracts: u32,
        },
    }

    bind_topic!(WorkingInfoUpdateEvent<BlockNumber>, b"^phala/mining/update");
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
    pub struct WorkingInfoUpdateEvent<BlockNumber> {
        /// The block emiting this message.
        pub block_number: BlockNumber,
        /// The timestamp of the block emiting this message.
        pub timestamp_ms: u64,
        /// Workers that do not responce the heartbeat challenge in time. Each delay only report once.
        pub offline: Vec<WorkerPublicKey>,
        /// Workers that received a heartbeat in offline state.
        pub recovered_to_online: Vec<WorkerPublicKey>,
        /// V update and payout info
        pub settle: Vec<SettleInfo>,
        // NOTE: Take care of the is_empty method when adding fields
    }

    impl<BlockNumber> WorkingInfoUpdateEvent<BlockNumber> {
        pub fn new(block_number: BlockNumber, timestamp_ms: u64) -> Self {
            Self {
                block_number,
                timestamp_ms,
                offline: Default::default(),
                recovered_to_online: Default::default(),
                settle: Default::default(),
            }
        }

        pub fn is_empty(&self) -> bool {
            self.offline.is_empty() && self.settle.is_empty() && self.recovered_to_online.is_empty()
        }
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct SettleInfo {
        pub pubkey: WorkerPublicKey,
        pub v: U64F64Bits,
        pub payout: U64F64Bits,
        pub treasury: U64F64Bits,
    }

    // Messages: Gatekeeper launch
    bind_topic!(GatekeeperLaunch, b"phala/gatekeeper/launch");
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub enum GatekeeperLaunch {
        FirstGatekeeper(NewGatekeeperEvent),
        MasterPubkeyOnChain(MasterPubkeyEvent),
        RotateMasterKey(RotateMasterKeyEvent),
        MasterPubkeyRotated(MasterPubkeyEvent),
    }

    impl GatekeeperLaunch {
        pub fn first_gatekeeper(
            pubkey: WorkerPublicKey,
            ecdh_pubkey: EcdhPublicKey,
        ) -> GatekeeperLaunch {
            GatekeeperLaunch::FirstGatekeeper(NewGatekeeperEvent {
                pubkey,
                ecdh_pubkey,
            })
        }

        pub fn master_pubkey_on_chain(master_pubkey: MasterPublicKey) -> GatekeeperLaunch {
            GatekeeperLaunch::MasterPubkeyOnChain(MasterPubkeyEvent { master_pubkey })
        }

        pub fn rotate_master_key(
            rotation_id: u64,
            gk_identities: Vec<WorkerIdentity>,
        ) -> GatekeeperLaunch {
            GatekeeperLaunch::RotateMasterKey(RotateMasterKeyEvent {
                rotation_id,
                gk_identities,
            })
        }

        pub fn master_pubkey_rotated(master_pubkey: MasterPublicKey) -> GatekeeperLaunch {
            GatekeeperLaunch::MasterPubkeyRotated(MasterPubkeyEvent { master_pubkey })
        }
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct NewGatekeeperEvent {
        /// The public key of registered gatekeeper
        pub pubkey: WorkerPublicKey,
        /// The ecdh public key of registered gatekeeper
        pub ecdh_pubkey: EcdhPublicKey,
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct RemoveGatekeeperEvent {
        /// The public key of registered gatekeeper
        pub pubkey: WorkerPublicKey,
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct MasterPubkeyEvent {
        pub master_pubkey: MasterPublicKey,
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct RotateMasterKeyEvent {
        pub rotation_id: u64,
        pub gk_identities: Vec<WorkerIdentity>,
    }

    // Messages: Gatekeeper change
    bind_topic!(GatekeeperChange, b"phala/gatekeeper/change");
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub enum GatekeeperChange {
        Registered(NewGatekeeperEvent),
        Unregistered(RemoveGatekeeperEvent),
    }

    impl GatekeeperChange {
        pub fn gatekeeper_registered(
            pubkey: WorkerPublicKey,
            ecdh_pubkey: EcdhPublicKey,
        ) -> GatekeeperChange {
            GatekeeperChange::Registered(NewGatekeeperEvent {
                pubkey,
                ecdh_pubkey,
            })
        }

        pub fn gatekeeper_unregistered(pubkey: WorkerPublicKey) -> GatekeeperChange {
            GatekeeperChange::Unregistered(RemoveGatekeeperEvent { pubkey })
        }
    }

    // Messages: Distribution of master key and contract keys
    bind_topic!(KeyDistribution<BlockNumber>, b"phala/gatekeeper/key");
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub enum KeyDistribution<BlockNumber> {
        /// Legacy single master key sharing, use `MasterKeyHistory` after we enable master key rotation
        ///
        /// MessageOrigin::Gatekeeper -> MessageOrigin::Worker
        MasterKeyDistribution(DispatchMasterKeyEvent),
        // TODO.shelven: a better way for real large batch key distribution
        /// MessageOrigin::Worker -> ALL
        ///
        /// The origin cannot be Gatekeeper, else the leakage of old master key will further leak the following keys
        MasterKeyRotation(BatchRotateMasterKeyEvent),
        /// MessageOrigin::Gatekeeper -> MessageOrigin::Worker
        MasterKeyHistory(DispatchMasterKeyHistoryEvent<BlockNumber>),
    }

    impl<BlockNumber> KeyDistribution<BlockNumber> {
        pub fn master_key_distribution(
            dest: WorkerPublicKey,
            ecdh_pubkey: EcdhPublicKey,
            encrypted_master_key: Vec<u8>,
            iv: AeadIV,
        ) -> KeyDistribution<BlockNumber> {
            KeyDistribution::MasterKeyDistribution(DispatchMasterKeyEvent {
                dest,
                ecdh_pubkey,
                encrypted_master_key,
                iv,
            })
        }
    }

    pub type AeadIV = [u8; 12];

    /// Secret key encrypted with AES-256-GCM algorithm
    ///
    /// The encryption key is generated with sr25519-based ECDH
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct EncryptedKey {
        /// The ecdh public key of key source
        pub ecdh_pubkey: EcdhPublicKey,
        /// Key encrypted with aead key
        pub encrypted_key: Vec<u8>,
        /// Aead IV
        pub iv: AeadIV,
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct DispatchMasterKeyEvent {
        /// The target to dispatch master key
        pub dest: WorkerPublicKey,
        /// The ecdh public key of master key source
        pub ecdh_pubkey: EcdhPublicKey,
        /// Master key encrypted with aead key
        pub encrypted_master_key: Vec<u8>,
        /// Aead IV
        pub iv: AeadIV,
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct DispatchMasterKeyHistoryEvent<BlockNumber> {
        /// The target to dispatch master key
        pub dest: WorkerPublicKey,
        pub encrypted_master_key_history: Vec<(u64, BlockNumber, EncryptedKey)>,
    }

    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct BatchRotateMasterKeyEvent {
        pub rotation_id: u64,
        pub secret_keys: BTreeMap<WorkerPublicKey, EncryptedKey>,
        pub sender: WorkerPublicKey,
        pub sig: Vec<u8>,
    }

    #[derive(Encode)]
    pub(crate) struct BatchRotateMasterKeyData<'a> {
        pub(crate) rotation_id: u64,
        pub(crate) secret_keys: &'a BTreeMap<WorkerPublicKey, EncryptedKey>,
        pub(crate) sender: WorkerPublicKey,
    }

    impl BatchRotateMasterKeyEvent {
        pub fn data_be_signed(&self) -> Vec<u8> {
            BatchRotateMasterKeyData {
                rotation_id: self.rotation_id,
                secret_keys: &self.secret_keys,
                sender: self.sender,
            }
            .encode()
        }
    }

    // Messages: Gatekeeper
    bind_topic!(GatekeeperEvent, b"phala/gatekeeper/event");
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub enum GatekeeperEvent {
        NewRandomNumber(RandomNumberEvent),
        TokenomicParametersChanged(TokenomicParameters),
        /// Deprecated after <https://github.com/Phala-Network/phala-blockchain/pull/499>
        /// Dropped in Phala. The index is reserved here for Khala+pruntime-v0 compatibility.
        _RepairV,
        /// Trigger a set of changes:
        /// - <https://github.com/Phala-Network/phala-blockchain/issues/693>
        /// - <https://github.com/Phala-Network/phala-blockchain/issues/676>
        /// Dropped in Phala. The index is reserved here for Khala+pruntime-v0 compatibility.
        _PhalaLaunched,
        /// Fix the payout duration problem in unresponsive state.
        /// Dropped in Phala. The index is reserved here for Khala+pruntime-v0 compatibility.
        _UnrespFix,
    }

    impl GatekeeperEvent {
        pub fn new_random_number(
            block_number: u32,
            random_number: RandomNumber,
            last_random_number: RandomNumber,
        ) -> GatekeeperEvent {
            GatekeeperEvent::NewRandomNumber(RandomNumberEvent {
                block_number,
                random_number,
                last_random_number,
            })
        }
    }

    pub type RandomNumber = [u8; 32];
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct RandomNumberEvent {
        pub block_number: u32,
        pub random_number: RandomNumber,
        pub last_random_number: RandomNumber,
    }

    #[cfg_attr(feature = "enable_serde", derive(Serialize, Deserialize))]
    #[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
    pub struct TokenomicParameters {
        // V calculation
        pub pha_rate: U64F64Bits,
        pub rho: U64F64Bits,
        pub budget_per_block: U64F64Bits,
        pub v_max: U64F64Bits,
        pub cost_k: U64F64Bits,
        pub cost_b: U64F64Bits,
        pub slash_rate: U64F64Bits,
        // Payout
        pub treasury_ratio: U64F64Bits,
        pub heartbeat_window: u32,
        // Ve calculation
        pub rig_k: U64F64Bits,
        pub rig_b: U64F64Bits,
        pub re: U64F64Bits,
        pub k: U64F64Bits,
        // Slash calculation
        pub kappa: U64F64Bits,
    }
}

// Types used in storage

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub enum AttestationReport {
    SgxIas {
        ra_report: Vec<u8>,
        signature: Vec<u8>,
        raw_signing_cert: Vec<u8>,
    },
}

#[cfg_attr(feature = "enable_serde", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, TypeInfo, Debug, Copy, Clone, PartialEq, Eq)]
pub enum AttestationProvider {
    #[cfg_attr(feature = "enable_serde", serde(rename = "root"))]
    Root,
    #[cfg_attr(feature = "enable_serde", serde(rename = "ias"))]
    Ias,
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Default, Clone, TypeInfo)]
pub enum WorkerStateEnum<BlockNumber> {
    #[default]
    Empty,
    Free,
    Gatekeeper,
    Pending,
    Computing(BlockNumber),
    Stopping,
}

#[derive(Encode, Decode, Debug, Default, Clone, TypeInfo)]
pub struct WorkerInfo<BlockNumber> {
    // identity
    pub machine_id: Vec<u8>,
    pub pubkey: Vec<u8>,
    pub last_updated: u64,
    // computing
    pub state: WorkerStateEnum<BlockNumber>,
    // performance
    pub score: Option<Score>,
    pub attestation_provider: Option<AttestationProvider>,
    pub confidence_level: u8,
    // version
    pub runtime_version: u32,
}

#[derive(Encode, Decode, Default, TypeInfo)]
pub struct StashInfo<AccountId: Default> {
    pub controller: AccountId,
    pub payout_prefs: PayoutPrefs<AccountId>,
}

#[derive(Encode, Decode, Default, TypeInfo)]
pub struct PayoutPrefs<AccountId: Default> {
    pub commission: u32,
    pub target: AccountId,
}

#[derive(Encode, Decode, Debug, Default, Clone, TypeInfo)]
pub struct Score {
    pub overall_score: u32,
    pub features: Vec<u32>,
}

type MachineId = Vec<u8>;
pub use sp_core::sr25519::Public as WorkerPublicKey;
pub use sp_core::sr25519::Public as ContractPublicKey;
pub use sp_core::sr25519::Public as ClusterPublicKey;
pub use sp_core::sr25519::Public as MasterPublicKey;
pub use sp_core::sr25519::Public as EcdhPublicKey;
pub use sp_core::sr25519::Signature as Sr25519Signature;

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerIdentity {
    pub pubkey: WorkerPublicKey,
    pub ecdh_pubkey: EcdhPublicKey,
}

/// One-time Challenge for WorkerKey handover
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct HandoverChallenge<BlockNumber> {
    pub sgx_target_info: Vec<u8>,
    // The challenge is only considered valid within 150 blocks (~30 min)
    pub block_number: BlockNumber,
    pub now: u64,
    pub dev_mode: bool,
    pub nonce: [u8; 32],
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct ChallengeHandlerInfo<BlockNumber> {
    pub challenge: HandoverChallenge<BlockNumber>,
    pub sgx_local_report: Vec<u8>,
    pub ecdh_pubkey: EcdhPublicKey,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct EncryptedWorkerKey {
    pub genesis_block_hash: H256,
    pub para_id: u32,
    pub dev_mode: bool,
    pub encrypted_key: messaging::EncryptedKey,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerRegistrationInfo<AccountId> {
    pub version: u32,
    pub machine_id: MachineId,
    pub pubkey: WorkerPublicKey,
    pub ecdh_pubkey: EcdhPublicKey,
    pub genesis_block_hash: H256,
    pub features: Vec<u32>,
    pub operator: Option<AccountId>,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerRegistrationInfoV2<AccountId> {
    pub version: u32,
    pub machine_id: MachineId,
    pub pubkey: WorkerPublicKey,
    pub ecdh_pubkey: EcdhPublicKey,
    pub genesis_block_hash: H256,
    pub features: Vec<u32>,
    pub operator: Option<AccountId>,
    pub para_id: u32,
    pub max_consensus_version: u32,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub enum VersionedWorkerEndpoints {
    V1(Vec<String>),
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerEndpointPayload {
    pub pubkey: WorkerPublicKey,
    pub versioned_endpoints: VersionedWorkerEndpoints,
    pub signing_time: u64,
}

#[derive(Encode, Decode, Debug, Default, TypeInfo)]
pub struct RoundInfo<BlockNumber> {
    pub round: u32,
    pub start_block: BlockNumber,
}

#[derive(Encode, Decode, Debug, Default, TypeInfo)]
pub struct StashWorkerStats<Balance> {
    pub slash: Balance,
    pub compute_received: Balance,
    pub online_received: Balance,
}

#[derive(Encode, Decode, Debug, Default, Clone, PartialEq, Eq, TypeInfo)]
pub struct RoundStats {
    pub round: u32,
    pub online_workers: u32,
    pub compute_workers: u32,
    /// The targeted online reward counts in fraction (base: 100_000)
    pub frac_target_online_reward: u32,
    pub total_power: u32,
    /// The targeted compute reward counts in fraction (base: 100_000)
    pub frac_target_compute_reward: u32,
}

#[derive(Encode, Decode, Debug, Default, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerStatsDelta {
    pub num_worker: i32,
    pub num_power: i32,
}

#[derive(Encode, Decode, Debug, Default, Clone, PartialEq, Eq, TypeInfo)]
pub enum PayoutReason {
    #[default]
    OnlineReward,
    ComputeReward,
}

#[repr(u8)]
pub enum SignedContentType {
    MqMessage = 0,
    RpcResponse = 1,
    EndpointInfo = 2,
    MasterKeyRotation = 3,
    MasterKeyStore = 4,
    ClusterStateRequest = 5,
}

pub fn wrap_content_to_sign(data: &[u8], sigtype: SignedContentType) -> Cow<[u8]> {
    match sigtype {
        // We don't wrap mq messages for backward compatibility.
        SignedContentType::MqMessage => data.into(),
        _ => {
            let mut wrapped: Vec<u8> = Vec::new();
            // MessageOrigin::Reserved.encode() == 0xff
            wrapped.push(0xff);
            wrapped.push(sigtype as u8);
            wrapped.extend_from_slice(data);
            wrapped.into()
        }
    }
}
