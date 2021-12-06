/*
 * BSD 3-Clause License
 *
 * Copyright (c) 2021, Alex Katlein
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 */

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_route53::model::{
    Change, ChangeAction, ChangeBatch, HostedZone, ResourceRecord, ResourceRecordSet, RrType,
};
use aws_sdk_route53::{Client, Region};
use dyndns::anyhow::{Context, Error};
use dyndns::config::Config;
use dyndns::provider::{DnsProvider, DnsRecords, DnsZones, Record, Zone};
use dyndns::result::DynResult;
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

pub struct AwsRoute53Provider {
    client: Client,
    _config: aws_config::Config,
}

impl Default for AwsRoute53Provider {
    fn default() -> Self {
        let build_instance = async {
            let region_provider =
                RegionProviderChain::default_provider().or_else(Region::new("us-west-1"));

            let config = aws_config::from_env().region(region_provider).load().await;
            let client = Client::new(&config);

            AwsRoute53Provider {
                client,
                _config: config,
            }
        };

        futures::executor::block_on(build_instance)
    }
}

impl DnsProvider for AwsRoute53Provider {
    fn current(&self, config: &Config) -> DynResult<DnsZones> {
        futures::executor::block_on(current(self, config))
    }

    fn update(&self, zone: &Zone, record: Record) -> DynResult<()> {
        futures::executor::block_on(update(self, zone, record))
    }
}

async fn current(provider: &AwsRoute53Provider, config: &Config) -> DynResult<DnsZones> {
    let mut aws_zones: Vec<HostedZone> = Vec::new();

    let mut last_aws_zone_marker = None;
    loop {
        let aws_list_zones_request = provider
            .client
            .list_hosted_zones()
            .set_marker(last_aws_zone_marker.clone());

        match aws_list_zones_request.send().await {
            Ok(aws_output) => {
                aws_zones.append(
                    &mut aws_output
                        .hosted_zones
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|hosted_zone| {
                            if let Some(hz_name) = &hosted_zone.name {
                                config.zones.contains_key(hz_name)
                            } else {
                                false
                            }
                        })
                        .collect(),
                );

                // aws_zones.append(&mut aws_output.hosted_zones.unwrap_or_default());
                if aws_output.is_truncated {
                    last_aws_zone_marker = aws_output.marker
                } else {
                    break;
                }
            }
            Err(err) => return Err(Error::from(err)),
        }
    }

    let mut result = HashMap::new();
    for aws_zone in aws_zones {
        let aws_zone_name = aws_zone.name.unwrap();
        let configured_zone = config.zones.get(&aws_zone_name).unwrap();

        let mut dns_records: DnsRecords = Vec::new();

        let aws_zone_id = aws_zone.id.unwrap();
        let mut last_aws_record_identifier = None;
        loop {
            let aws_list_records_request = provider
                .client
                .list_resource_record_sets()
                .hosted_zone_id(&aws_zone_id)
                .set_start_record_identifier(last_aws_record_identifier.clone());

            match aws_list_records_request.send().await {
                Ok(aws_output) => {
                    dns_records.append(
                        &mut aws_output
                            .resource_record_sets
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|record_set| {
                                let record_set_name = record_set.name.unwrap();
                                let record_set_type = record_set.r#type.unwrap();

                                configured_zone
                                    .iter()
                                    .find(|record| match record_set_type {
                                        RrType::A => {
                                            if let Some(a_record) = record.a.as_ref() {
                                                a_record == &record_set_name
                                            } else {
                                                false
                                            }
                                        }
                                        RrType::Aaaa => {
                                            if let Some(aaaa_record) = record.aaaa.as_ref() {
                                                aaaa_record == &record_set_name
                                            } else {
                                                false
                                            }
                                        }
                                        _ => false,
                                    })?;

                                let records = record_set.resource_records.unwrap_or_default();
                                if !records.is_empty() {
                                    match record_set_type {
                                        RrType::A => Some(Record::A {
                                            name: record_set_name,
                                            value: Ipv4Addr::from_str(
                                                if let Some(Some(value)) =
                                                    records.first().map(|it| &it.value)
                                                {
                                                    value
                                                } else {
                                                    return None;
                                                },
                                            )
                                            .unwrap(),
                                        }),
                                        RrType::Aaaa => Some(Record::AAAA {
                                            name: record_set_name,
                                            value: Ipv6Addr::from_str(
                                                if let Some(Some(value)) =
                                                    records.first().map(|it| &it.value)
                                                {
                                                    value
                                                } else {
                                                    return None;
                                                },
                                            )
                                            .unwrap(),
                                        }),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    );

                    if aws_output.is_truncated {
                        last_aws_record_identifier = aws_output.next_record_identifier
                    } else {
                        break;
                    }
                }
                Err(err) => return Err(Error::from(err)),
            }
        }

        result.insert(Zone::with_id(aws_zone_name, aws_zone_id), dns_records);
    }

    Ok(result)
}

async fn update(provider: &AwsRoute53Provider, zone: &Zone, record: Record) -> DynResult<()> {
    let zone_id = if let Some(zone_id) = &zone.id {
        zone_id.clone()
    } else {
        eprintln!("No such hosted zone: {}", zone.name);
        return Ok(());
    };

    let change_resource_record_set_request = provider
        .client
        .change_resource_record_sets()
        .hosted_zone_id(zone_id)
        .change_batch(
            ChangeBatch::builder()
                .changes(
                    Change::builder()
                        .action(ChangeAction::Upsert)
                        .resource_record_set(match &record {
                            Record::A { name, value } => ResourceRecordSet::builder()
                                .name(name)
                                .r#type(RrType::A)
                                .resource_records(
                                    ResourceRecord::builder().value(value.to_string()).build(),
                                )
                                .build(),
                            Record::AAAA { name, value } => ResourceRecordSet::builder()
                                .name(name)
                                .r#type(RrType::Aaaa)
                                .resource_records(
                                    ResourceRecord::builder().value(value.to_string()).build(),
                                )
                                .build(),
                        })
                        .build(),
                )
                .build(),
        );

    change_resource_record_set_request
        .send()
        .await
        .context(format!("failed to update record {}", record))?;

    Ok(())
}
