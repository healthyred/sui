// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use async_graphql::{connection::Connection, *};
use sui_json_rpc::name_service::NameServiceConfig;
use sui_types::TypeTag;

use super::{
    address::Address,
    available_range::AvailableRange,
    checkpoint::{Checkpoint, CheckpointId},
    coin::Coin,
    coin_metadata::CoinMetadata,
    epoch::Epoch,
    event::{Event, EventFilter},
    move_type::MoveType,
    object::{Object, ObjectFilter},
    owner::{ObjectOwner, Owner},
    protocol_config::ProtocolConfigs,
    sui_address::SuiAddress,
    sui_system_state_summary::SuiSystemStateSummary,
    transaction_block::{TransactionBlock, TransactionBlockFilter},
};
use crate::{
    config::ServiceConfig, context_data::db_data_provider::PgManager, error::Error,
    mutation::Mutation,
};

pub(crate) struct Query;
pub(crate) type SuiGraphQLSchema = async_graphql::Schema<Query, Mutation, EmptySubscription>;

#[Object]
impl Query {
    /// First four bytes of the network's genesis checkpoint digest (uniquely identifies the
    /// network).
    async fn chain_identifier(&self, ctx: &Context<'_>) -> Result<String> {
        ctx.data_unchecked::<PgManager>()
            .fetch_chain_identifier()
            .await
            .extend()
    }

    /// Range of checkpoints that the RPC has data available for (for data
    /// that can be tied to a particular checkpoint).
    async fn available_range(&self) -> Result<AvailableRange> {
        Ok(AvailableRange)
    }

    /// Configuration for this RPC service
    async fn service_config(&self, ctx: &Context<'_>) -> Result<ServiceConfig> {
        ctx.data()
            .map_err(|_| Error::Internal("Unable to fetch service configuration.".to_string()))
            .cloned()
            .extend()
    }

    // availableRange - pending impl. on IndexerV2
    // dryRunTransactionBlock
    // coinMetadata

    async fn owner(&self, address: SuiAddress) -> Option<ObjectOwner> {
        Some(ObjectOwner::Owner(Owner { address }))
    }

    async fn object(
        &self,
        ctx: &Context<'_>,
        address: SuiAddress,
        version: Option<u64>,
    ) -> Result<Option<Object>> {
        ctx.data_unchecked::<PgManager>()
            .fetch_obj(address, version)
            .await
            .extend()
    }

    async fn address(&self, address: SuiAddress) -> Option<Address> {
        Some(Address { address })
    }

    /// Fetch a structured representation of a concrete type, including its layout information.
    /// Fails if the type is malformed.
    async fn type_(&self, type_: String) -> Result<MoveType> {
        Ok(MoveType::new(
            TypeTag::from_str(&type_)
                .map_err(|e| Error::Client(format!("Bad type: {e}")))
                .extend()?,
        ))
    }

    /// Fetch epoch information by ID (defaults to the latest epoch).
    async fn epoch(&self, ctx: &Context<'_>, id: Option<u64>) -> Result<Option<Epoch>> {
        if let Some(epoch_id) = id {
            ctx.data_unchecked::<PgManager>()
                .fetch_epoch(epoch_id)
                .await
                .extend()
        } else {
            Ok(Some(
                ctx.data_unchecked::<PgManager>()
                    .fetch_latest_epoch()
                    .await
                    .extend()?,
            ))
        }
    }

    /// Fetch checkpoint information by sequence number or digest (defaults to the latest available
    /// checkpoint).
    async fn checkpoint(
        &self,
        ctx: &Context<'_>,
        id: Option<CheckpointId>,
    ) -> Result<Option<Checkpoint>> {
        if let Some(id) = id {
            match (&id.digest, &id.sequence_number) {
                (Some(_), Some(_)) => Err(Error::InvalidCheckpointQuery.extend()),
                _ => ctx
                    .data_unchecked::<PgManager>()
                    .fetch_checkpoint(id.digest.as_deref(), id.sequence_number)
                    .await
                    .extend(),
            }
        } else {
            Ok(Some(
                ctx.data_unchecked::<PgManager>()
                    .fetch_latest_checkpoint()
                    .await
                    .extend()?,
            ))
        }
    }

    /// Fetch a transaction block by its transaction digest.
    async fn transaction_block(
        &self,
        ctx: &Context<'_>,
        digest: String,
    ) -> Result<Option<TransactionBlock>> {
        ctx.data_unchecked::<PgManager>()
            .fetch_tx(&digest)
            .await
            .extend()
    }

    /// The coin objects that exist in the network.
    ///
    /// The type field is a string of the inner type of the coin by which to filter
    /// (e.g. `0x2::sui::SUI`). If no type is provided, it will default to `0x2::sui::SUI`.
    async fn coin_connection(
        &self,
        ctx: &Context<'_>,
        first: Option<u64>,
        after: Option<String>,
        last: Option<u64>,
        before: Option<String>,
        type_: Option<String>,
    ) -> Result<Option<Connection<String, Coin>>> {
        ctx.data_unchecked::<PgManager>()
            .fetch_coins(None, type_, first, after, last, before)
            .await
            .extend()
    }

    async fn checkpoint_connection(
        &self,
        ctx: &Context<'_>,
        first: Option<u64>,
        after: Option<String>,
        last: Option<u64>,
        before: Option<String>,
    ) -> Result<Option<Connection<String, Checkpoint>>> {
        ctx.data_unchecked::<PgManager>()
            .fetch_checkpoints(first, after, last, before, None)
            .await
            .extend()
    }

    async fn transaction_block_connection(
        &self,
        ctx: &Context<'_>,
        first: Option<u64>,
        after: Option<String>,
        last: Option<u64>,
        before: Option<String>,
        filter: Option<TransactionBlockFilter>,
    ) -> Result<Option<Connection<String, TransactionBlock>>> {
        ctx.data_unchecked::<PgManager>()
            .fetch_txs(first, after, last, before, filter)
            .await
            .extend()
    }

    async fn event_connection(
        &self,
        ctx: &Context<'_>,
        first: Option<u64>,
        after: Option<String>,
        last: Option<u64>,
        before: Option<String>,
        filter: Option<EventFilter>,
    ) -> Result<Option<Connection<String, Event>>> {
        ctx.data_unchecked::<PgManager>()
            .fetch_events(first, after, last, before, filter)
            .await
            .extend()
    }

    async fn object_connection(
        &self,
        ctx: &Context<'_>,
        first: Option<u64>,
        after: Option<String>,
        last: Option<u64>,
        before: Option<String>,
        filter: Option<ObjectFilter>,
    ) -> Result<Option<Connection<String, Object>>> {
        ctx.data_unchecked::<PgManager>()
            .fetch_objs(first, after, last, before, filter)
            .await
            .extend()
    }

    /// Fetch the protocol config by protocol version (defaults to the latest protocol
    /// version known to the GraphQL)
    async fn protocol_config(
        &self,
        ctx: &Context<'_>,
        protocol_version: Option<u64>,
    ) -> Result<ProtocolConfigs> {
        ctx.data_unchecked::<PgManager>()
            .fetch_protocol_configs(protocol_version)
            .await
            .extend()
    }

    /// Resolves the owner address of the provided domain name
    async fn resolve_name_service_address(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> Result<Option<Address>> {
        ctx.data_unchecked::<PgManager>()
            .resolve_name_service_address(ctx.data_unchecked::<NameServiceConfig>(), name)
            .await
            .extend()
    }

    async fn latest_sui_system_state(&self, ctx: &Context<'_>) -> Result<SuiSystemStateSummary> {
        ctx.data_unchecked::<PgManager>()
            .fetch_latest_sui_system_state()
            .await
            .extend()
    }

    async fn coin_metadata(
        &self,
        ctx: &Context<'_>,
        coin_type: String,
    ) -> Result<Option<CoinMetadata>> {
        ctx.data_unchecked::<PgManager>()
            .fetch_coin_metadata(coin_type)
            .await
            .extend()
    }
}
